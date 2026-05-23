use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, WindowId};

pub mod handle;
pub mod js;

pub use handle::{MainToJs, UserEvent, WindowEntryId, WindowShared, WinitHandle};
pub use js::{JsState, JsWindow, SharedJsState, WindowMirror, with_state, with_state_ref};

use crate::clipboard::SystemClipboard;
use crate::gpu::GpuContext;
use crate::terminal_colors;
use crate::window::GpuWindow;

#[derive(Clone, Debug)]
pub struct AppConfig {
    /// Entry module to execute (resolved absolute path).
    pub entry: PathBuf,
    /// Root directory used for module/node resolution.
    pub app_root: PathBuf,
    /// Extra runtime args exposed via `Deno.args`.
    pub args: Vec<String>,
    /// Root directory for `Uz.path.resource(rel)`.
    /// In dev: project dir (where `uzumaki.config.json` sits, or the entry's parent).
    /// In standalone: `<exe_dir>/resources` (or wherever the host stages bundled files).
    pub resource_root: PathBuf,
    /// App identifier (e.g. `com.uzumaki.playground`)
    pub identifier: String,
    pub jsx_import_source: Option<String>,
}

/// Main-thread per-window state. Owns the winit window handle plus all GPU
/// resources. The `shared` Arc is held in lock-step with the JS thread's
/// `Window` so size/scale/frame coordination is lock-free.
struct MainWindow {
    gpu: GpuWindow,
    animation_frame_pending: bool,
}

pub struct Application {
    gpu: GpuContext,
    clipboard: SystemClipboard,
    windows: HashMap<WindowEntryId, MainWindow>,
    winit_id_to_entry_id: HashMap<WindowId, WindowEntryId>,
    /// Queue of frame-build requests that have been forwarded to JS but whose
    /// `FrameReady` reply hasn't arrived yet. We track by id so a flurry of
    /// `RedrawRequested`s coalesces into one outstanding `BuildFrame`.
    frame_build_outstanding: HashMap<WindowEntryId, bool>,

    event_loop: Option<winit::event_loop::EventLoop<UserEvent>>,
    main_to_js: flume::Sender<MainToJs>,
    js_thread: Option<std::thread::JoinHandle<()>>,
}

impl Application {
    pub fn new_with_root(
        startup_snapshot: Option<&'static [u8]>,
        config: AppConfig,
    ) -> Result<Self> {
        let event_loop: winit::event_loop::EventLoop<UserEvent> =
            winit::event_loop::EventLoop::with_user_event().build()?;
        let proxy = event_loop.create_proxy();

        let gpu = pollster::block_on(GpuContext::new()).expect("Failed to create GPU context");
        let clipboard = SystemClipboard::new().expect("failed to initialize system clipboard");

        let (main_to_js_tx, main_to_js_rx) = flume::unbounded::<MainToJs>();

        let js_thread = js::spawn_js_thread(startup_snapshot, config, proxy, main_to_js_rx);

        Ok(Self {
            gpu,
            clipboard,
            windows: HashMap::new(),
            winit_id_to_entry_id: HashMap::new(),
            frame_build_outstanding: HashMap::new(),
            event_loop: Some(event_loop),
            main_to_js: main_to_js_tx,
            js_thread: Some(js_thread),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let Some(event_loop) = self.event_loop.take() else {
            return Ok(());
        };

        event_loop.run_app(self)?;

        let _ = self.main_to_js.send(MainToJs::Shutdown);
        if let Some(handle) = self.js_thread.take() {
            let _ = handle.join();
        }
        Ok(())
    }

    fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowEntryId,
        options: crate::ops::window::WindowOptions,
    ) {
        let attributes = options.to_window_attributes().with_visible(false);
        // We always create hidden, then flip visibility after surface setup —
        // avoids a flash of the default desktop background.
        let target_visible = options.visible();
        let transparent = options.transparent();

        let Ok(winit_window) = event_loop.create_window(attributes.clone()) else {
            eprintln!("Failed to create window");
            return;
        };
        winit_window.set_ime_allowed(true);

        let winit_window = Arc::new(winit_window);
        let winit_id = winit_window.id();
        let scale = winit_window.scale_factor();
        let physical = winit_window.inner_size();
        let logical = (
            (physical.width as f64 / scale).round() as u32,
            (physical.height as f64 / scale).round() as u32,
        );

        let shared = Arc::new(WindowShared::new(
            id,
            WinitHandle::new(winit_window.clone()),
            logical,
            scale,
        ));

        let gpu_window =
            match GpuWindow::new(&self.gpu, winit_window.clone(), shared.clone(), transparent) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Error creating GPU window: {e:#?}");
                    return;
                }
            };

        options.apply_post_create_state(&winit_window);
        winit_window.set_visible(target_visible);

        self.windows.insert(
            id,
            MainWindow {
                gpu: gpu_window,
                animation_frame_pending: false,
            },
        );
        self.winit_id_to_entry_id.insert(winit_id, id);

        let theme = winit_window.theme();
        let _ = self
            .main_to_js
            .send(MainToJs::WindowCreated { id, shared, theme });
    }

    fn close_window(&mut self, event_loop: &ActiveEventLoop, id: WindowEntryId) {
        // JS already dispatched the windowClose lifecycle event and dropped
        // its native window handle. Here we drop GPU resources, then bounce a
        // `DropJsWindow` back so JS releases the `JsWindow` entry only after
        // any deferred React commit / microtask work has drained.
        if let Some(win) = self.windows.remove(&id) {
            self.winit_id_to_entry_id.remove(&win.gpu.id());
        }
        self.frame_build_outstanding.remove(&id);
        let _ = self.main_to_js.send(MainToJs::DropJsWindow { id });
        if self.windows.is_empty() {
            event_loop.exit();
        }
    }

    fn request_build_frame(&mut self, id: WindowEntryId) {
        // Coalesce: if a BuildFrame is already in flight for this window, the
        // pending RedrawRequested will be served by the in-flight build (JS
        // always renders the latest DOM state when it processes the message).
        if self.frame_build_outstanding.insert(id, true) == Some(true) {
            return;
        }
        let _ = self.main_to_js.send(MainToJs::BuildFrame { id });
    }
}

impl ApplicationHandler<UserEvent> for Application {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        let _ = self.main_to_js.send(MainToJs::Resumed);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::CreateWindow { id, options } => {
                self.create_window(event_loop, id, options);
            }
            UserEvent::CloseWindow { id } => {
                self.close_window(event_loop, id);
            }
            UserEvent::FrameReady { id } => {
                self.frame_build_outstanding.remove(&id);
                if let Some(win) = self.windows.get_mut(&id) {
                    win.gpu.present_pending_frame();
                    if win.animation_frame_pending {
                        win.gpu.winit_window.request_redraw();
                    }
                }
            }
            UserEvent::AnimationFramePending { id, pending } => {
                if let Some(win) = self.windows.get_mut(&id) {
                    let was_pending = win.animation_frame_pending;
                    win.animation_frame_pending = pending;
                    if pending && !was_pending {
                        win.gpu.winit_window.request_redraw();
                    }
                }
            }
            UserEvent::SetCursor { id, icon } => {
                if let Some(win) = self.windows.get(&id) {
                    win.gpu.winit_window.set_cursor(icon.to_winit());
                }
            }
            UserEvent::SetImeArea { id, position, size } => {
                if let Some(win) = self.windows.get(&id) {
                    win.gpu.winit_window.set_ime_cursor_area(position, size);
                }
            }
            UserEvent::SetTitle { id, title } => {
                apply(&self.windows, id, |w| w.gpu.winit_window.set_title(&title))
            }
            UserEvent::SetVisible { id, visible } => apply(&self.windows, id, |w| {
                w.gpu.winit_window.set_visible(visible)
            }),
            UserEvent::SetResizable { id, resizable } => apply(&self.windows, id, |w| {
                w.gpu.winit_window.set_resizable(resizable)
            }),
            UserEvent::SetDecorations { id, decorations } => apply(&self.windows, id, |w| {
                w.gpu.winit_window.set_decorations(decorations)
            }),
            UserEvent::SetTransparent { id, transparent } => {
                if let Some(win) = self.windows.get_mut(&id) {
                    win.gpu.set_transparent(transparent);
                }
            }
            UserEvent::SetMaximized { id, maximized } => apply(&self.windows, id, |w| {
                w.gpu.winit_window.set_maximized(maximized)
            }),
            UserEvent::SetMinimized { id, minimized } => apply(&self.windows, id, |w| {
                w.gpu.winit_window.set_minimized(minimized)
            }),
            UserEvent::SetFullscreen { id, fullscreen } => apply(&self.windows, id, |w| {
                let target = fullscreen.then_some(Fullscreen::Borderless(None));
                w.gpu.winit_window.set_fullscreen(target);
            }),
            UserEvent::SetWindowLevel { id, level } => apply(&self.windows, id, |w| {
                w.gpu.winit_window.set_window_level(level)
            }),
            UserEvent::SetMinSize { id, size } => apply(&self.windows, id, |w| {
                w.gpu.winit_window.set_min_inner_size(Some(size));
            }),
            UserEvent::SetMaxSize { id, size } => apply(&self.windows, id, |w| {
                w.gpu.winit_window.set_max_inner_size(Some(size));
            }),
            UserEvent::SetPosition { id, position } => apply(&self.windows, id, |w| {
                w.gpu.winit_window.set_outer_position(position)
            }),
            UserEvent::SetTheme { id, theme } => {
                apply(&self.windows, id, |w| w.gpu.winit_window.set_theme(theme))
            }
            UserEvent::SetContentProtected { id, protected } => apply(&self.windows, id, |w| {
                w.gpu.winit_window.set_content_protected(protected)
            }),
            UserEvent::SetEnabledButtons { id, buttons } => apply(&self.windows, id, |w| {
                w.gpu.winit_window.set_enabled_buttons(buttons)
            }),
            UserEvent::Focus { id } => {
                apply(&self.windows, id, |w| w.gpu.winit_window.focus_window())
            }
            UserEvent::CursorBlink { id, generation } => {
                let _ = self
                    .main_to_js
                    .send(MainToJs::CursorBlink { id, generation });
            }
            UserEvent::ClipboardRead { reply } => {
                let result = self.clipboard.read_text().unwrap_or(None);
                let _ = reply.send(result);
            }
            UserEvent::ClipboardWrite { text, reply } => {
                let ok = self.clipboard.write_text(&text).is_ok();
                let _ = reply.send(ok);
            }
            UserEvent::Quit => {
                self.windows.clear();
                self.winit_id_to_entry_id.clear();
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(&wid) = self.winit_id_to_entry_id.get(&window_id) else {
            return;
        };

        match &event {
            WindowEvent::Resized(size) => {
                if let Some(win) = self.windows.get_mut(&wid) {
                    win.gpu.on_resize(size.width, size.height);
                    let scale = win.gpu.winit_window.scale_factor();
                    let lw = (size.width as f64 / scale).round() as u32;
                    let lh = (size.height as f64 / scale).round() as u32;
                    win.gpu.shared.store_inner_size(lw, lh);
                    win.gpu.winit_window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(win) = self.windows.get(&wid) {
                    win.gpu.shared.store_scale_factor(*scale_factor);
                }
            }
            WindowEvent::RedrawRequested => {
                self.request_build_frame(wid);
                return;
            }
            _ => {}
        }

        // Forward every WindowEvent to JS (clone is needed because winit owns
        // the original by value and we already pattern-matched).
        let _ = self
            .main_to_js
            .send(MainToJs::WindowEvent { id: wid, event });
    }
}

fn apply<F: FnOnce(&MainWindow)>(
    map: &HashMap<WindowEntryId, MainWindow>,
    id: WindowEntryId,
    f: F,
) {
    if let Some(win) = map.get(&id) {
        f(win);
    }
}

pub(crate) fn print_runtime_error(err: &anyhow::Error) {
    eprintln!("{} {:#}", terminal_colors::red_bold("Error"), err);
}
