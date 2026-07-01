use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use anyhow::Result;
use deno_core::v8;
use deno_runtime::worker::MainWorker;
use serde_json::Value;
use winit::event_loop::EventLoopProxy;
use winit::window::{Theme, WindowButtons, WindowLevel};

use crate::app::handle::{MainToJs, PendingDestroy, UserEvent, WindowEntryId};
use crate::app::{AppConfig, print_runtime_error};
use crate::cursor::UzCursorIcon;
use crate::element::ImageData;
use crate::events::{self, AppEvent};
use crate::node::{UzNodeId, VarBinding};
use crate::ops::style_util::{clear_node_style, set_node_style};
use crate::ops::window::WindowOptions;
use crate::prop_keys::{AttrValue, AttributeKind};
use crate::runtime::worker::{WorkerBuildOptions, create_worker};
use crate::ui::UIState;
use crate::window;

/// JS-thread-only state. Holds every per-window DOM, the image cache, and the
/// cppgc-deferred destroy queue. Never accessed from the main winit thread.
pub struct JsState {
    pub proxy: EventLoopProxy<UserEvent>,
    pub windows: HashMap<WindowEntryId, JsWindow>,
    pub image_cache: HashMap<String, ImageData>,
    pub pending_destroy: VecDeque<PendingDestroy>,
    /// Net change in external bytes attributable to retained DOM nodes since
    /// the last flush. Pushed to V8 with
    /// `Isolate::adjust_amount_of_external_allocated_memory`.
    pub external_memory_delta: i64,
}

impl JsState {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            proxy,
            windows: HashMap::new(),
            image_cache: HashMap::new(),
            pending_destroy: VecDeque::new(),
            external_memory_delta: 0,
        }
    }

    /// Process the deferred destroy queue. Each entry is a node whose JS
    /// `CoreNode` wrapper has been finalized by cppgc. If the node is no
    /// longer connected to its document root we free it; if it is still
    /// connected, the tree still owns it, so we re-queue and re-check on the
    /// next drain (which runs after every event tick).
    pub fn drain_pending_destroy(&mut self) {
        let pass = self.pending_destroy.len();
        for _ in 0..pass {
            let Some((window_id, node_id)) = self.pending_destroy.pop_front() else {
                break;
            };
            let Some(entry) = self.windows.get_mut(&window_id) else {
                continue;
            };
            if !entry.dom.nodes.contains(node_id) {
                continue;
            }
            if entry.dom.is_connected(node_id) {
                self.pending_destroy.push_back((window_id, node_id));
            } else {
                entry.dom.destroy_node(node_id);
            }
        }
    }
}

pub type SharedJsState = Rc<RefCell<JsState>>;

pub fn with_state<R>(state: &SharedJsState, f: impl FnOnce(&mut JsState) -> R) -> R {
    f(&mut state.borrow_mut())
}

pub fn with_state_ref<R>(state: &SharedJsState, f: impl FnOnce(&JsState) -> R) -> R {
    f(&state.borrow())
}

/// JS-thread per-window state. Holds the DOM, the text renderer (font/layout
/// contexts), and a mirror of every window attribute exposed to JS so getters
/// stay synchronous without round-tripping to main.
///
/// `window` is `None` between `op_create_window` and the matching
/// `MainToJs::WindowCreated` reply i.e. before the winit window exists.
pub struct JsWindow {
    pub window: Option<window::Window>,
    pub dom: UIState,
    pub rem_base: f32,
    pub cursor_blink_generation: u64,

    pub blink_timer: Option<tokio::task::JoinHandle<()>>,
    pub mouse_buttons: events::MouseButtons,
    pub modifiers: events::KeyModifiers,
    pub state: WindowMirror,
    /// Theme variables addressable from style attrs as `$name`.
    pub vars: HashMap<String, String>,
}

/// Mirror of every window attribute that JS may read synchronously. Updates
/// happen in two places: (1) setters apply the change locally and forward via
/// `UserEvent`; (2) main-thread events (resize/focus/move) update the mirror
/// when forwarded as `WindowEvent`s.
pub struct WindowMirror {
    pub title: String,
    pub visible: bool,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub maximized: bool,
    pub minimized: bool,
    pub fullscreen: bool,
    pub window_level: WindowLevel,
    pub content_protected: bool,
    pub enabled_buttons: WindowButtons,
    pub theme: Option<Theme>,
    pub focused: bool,
    pub outer_position: Option<winit::dpi::PhysicalPosition<i32>>,
    pub outer_size: Option<winit::dpi::PhysicalSize<u32>>,
}

impl WindowMirror {
    pub fn from_options(options: &WindowOptions) -> Self {
        Self {
            title: options.title().to_string(),
            visible: options.visible(),
            resizable: options.resizable(),
            decorations: options.decorations(),
            transparent: options.transparent(),
            maximized: options.maximized(),
            minimized: options.minimized(),
            fullscreen: options.fullscreen(),
            window_level: options.window_level(),
            content_protected: options.content_protected(),
            enabled_buttons: options.enabled_buttons(),
            theme: options.theme_winit(),
            focused: false,
            outer_position: None,
            outer_size: None,
        }
    }
}

impl JsWindow {
    pub fn new(options: &WindowOptions) -> Self {
        let mut dom = UIState::new();
        let root = dom.create_view(crate::style::UzStyle::root());
        dom.set_root(root);
        Self {
            window: None,
            dom,
            rem_base: 16.0,
            cursor_blink_generation: 0,
            blink_timer: None,
            mouse_buttons: events::MouseButtons::empty(),
            modifiers: events::KeyModifiers::empty(),
            state: WindowMirror::from_options(options),
            vars: HashMap::new(),
        }
    }

    /// Inner size in logical pixels.
    pub fn inner_size(&self) -> Option<(u32, u32)> {
        self.window.as_ref().map(|w| {
            let scale = w.scale_factor();
            let (pw, ph) = w.inner_size();
            (
                (pw as f64 / scale).round() as u32,
                (ph as f64 / scale).round() as u32,
            )
        })
    }

    pub fn scale_factor(&self) -> Option<f32> {
        self.window.as_ref().map(|w| w.scale_factor() as f32)
    }
}

/// Prefix that marks an attribute value as a window var reference.
const VAR_PREFIX: &str = "$";

impl JsWindow {
    pub(crate) fn set_attribute(&mut self, node_id: UzNodeId, name: &str, value: &str) {
        if let Some(node) = self.dom.nodes.get_mut(node_id) {
            node.var_bindings.retain(|b| b.attr_name != name);
        }

        if let Some(var_name) = value.strip_prefix(VAR_PREFIX) {
            if let Some(node) = self.dom.nodes.get_mut(node_id) {
                node.var_bindings.push(VarBinding {
                    attr_name: name.to_string(),
                    var_name: var_name.to_string(),
                });
            }
            let Some(resolved) = self.vars.get(var_name).cloned() else {
                // Unknown var:  leave the prior style as-is; the binding will
                // pick up the real value once `set_var` defines it.
                return;
            };
            self.apply_attribute(node_id, name, &resolved);
            return;
        }

        self.apply_attribute(node_id, name, value);
    }

    fn apply_attribute(&mut self, node_id: UzNodeId, name: &str, value: &str) {
        let kind = AttributeKind::parse(name);
        match kind {
            AttributeKind::Element(name) => {
                if let Some(node) = self.dom.nodes.get_mut(node_id) {
                    node.set_attr(name, AttrValue::from(value));
                }
            }
            AttributeKind::Style(prop, variant) => {
                set_node_style(
                    &mut self.dom,
                    node_id,
                    prop,
                    variant,
                    AttrValue::from(value),
                    self.rem_base,
                );
            }
        };
    }

    pub fn clear_attribute(&mut self, node_id: UzNodeId, name: &str) {
        if let Some(node) = self.dom.nodes.get_mut(node_id) {
            node.var_bindings.retain(|b| b.attr_name != name);
        }
        let kind = AttributeKind::parse(name);
        let Some(node) = self.dom.nodes.get_mut(node_id) else {
            return;
        };

        match kind {
            AttributeKind::Element(name) => {
                node.clear_attr(name);
            }
            AttributeKind::Style(prop, variant) => {
                clear_node_style(&mut self.dom, node_id, prop, variant)
            }
        };
    }

    /// Set or remove a single window variable, then re-apply every attribute
    /// bound to it. `None` removes the var; the bound attrs are cleared back
    /// to their defaults.
    pub fn set_var(&mut self, key: &str, value: Option<String>) {
        match value {
            Some(v) => {
                self.vars.insert(key.to_string(), v);
            }
            None => {
                self.vars.remove(key);
            }
        }

        let affected: Vec<(UzNodeId, String)> = self
            .dom
            .nodes
            .iter()
            .flat_map(|(id, node)| {
                node.var_bindings
                    .iter()
                    .filter(|b| b.var_name == key)
                    .map(move |b| (id, b.attr_name.clone()))
            })
            .collect();

        let resolved = self.vars.get(key).cloned();
        for (nid, attr) in affected {
            match &resolved {
                Some(v) => self.apply_attribute(nid, &attr, v),
                None => {
                    let kind = AttributeKind::parse(&attr);
                    if let AttributeKind::Style(prop, variant) = kind {
                        clear_node_style(&mut self.dom, nid, prop, variant);
                    }
                }
            }
        }
    }

    pub fn get_attribute(&self, node_id: UzNodeId, name: &str) -> Value {
        let kind = AttributeKind::parse(name);

        let Some(node) = self.dom.nodes.get(node_id) else {
            return Value::Null;
        };

        match kind {
            AttributeKind::Element(name) => node
                .as_element()
                .and_then(|el| el.get_attr(name))
                .unwrap_or(Value::Null),
            AttributeKind::Style(_, _variant) => Value::Null, // todo computed styls ?
        }
    }

    pub fn set_cursor(&mut self, _node_id: UzNodeId, _icon: UzCursorIcon) {
        todo!()
    }
}

/// Spawn the JS thread. Returns immediately; the thread owns the deno worker
/// and a current-thread tokio runtime for the lifetime of the application.
pub fn spawn_js_thread(
    snapshot: Option<&'static [u8]>,
    config: AppConfig,
    proxy: EventLoopProxy<UserEvent>,
    main_to_js: flume::Receiver<MainToJs>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("uzumaki-js".into())
        .spawn(move || {
            if let Err(err) = run_js_thread(snapshot, config, proxy, main_to_js) {
                print_runtime_error(&err);
            }
        })
        .expect("failed to spawn uzumaki-js thread")
}

fn run_js_thread(
    snapshot: Option<&'static [u8]>,
    config: AppConfig,
    proxy: EventLoopProxy<UserEvent>,
    main_to_js: flume::Receiver<MainToJs>,
) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("failed to create tokio runtime");

    let main_file = config.entry.clone();
    let app_root = config.app_root.clone();

    let mut worker = {
        let _guard = rt.enter();
        create_worker(WorkerBuildOptions {
            entry: &main_file,
            app_root: &app_root,
            args: config.args.clone(),
            headless: false,
            jsx_import_source: config.jsx_import_source.clone(),
            extensions: vec![crate::uzumaki::init()],
            startup_snapshot: snapshot,
        })?
    };

    let (event_dispatch, animation_frame_fn) = {
        let context = worker.js_runtime.main_context();
        deno_core::scope!(scope, &mut worker.js_runtime);
        let context_local = v8::Local::new(scope, context);
        let global_obj = context_local.global(scope);

        let dispatch = JSGlobalEventDispatch {
            mouse: get_global_fn(scope, global_obj, b"__uzumaki_dispatch_mouse__")?,
            keyboard: get_global_fn(scope, global_obj, b"__uzumaki_dispatch_keyboard__")?,
            input: get_global_fn(scope, global_obj, b"__uzumaki_dispatch_input__")?,
            focus: get_global_fn(scope, global_obj, b"__uzumaki_dispatch_focus__")?,
            clipboard: get_global_fn(scope, global_obj, b"__uzumaki_dispatch_clipboard__")?,
            selectionchange: get_global_fn(
                scope,
                global_obj,
                b"__uzumaki_dispatch_selectionchange__",
            )?,
            resize: get_global_fn(scope, global_obj, b"__uzumaki_dispatch_resize__")?,
            window_load: get_global_fn(scope, global_obj, b"__uzumaki_window_load__")?,
            window_close: get_global_fn(scope, global_obj, b"__uzumaki_window_close__")?,
            theme_changed: get_global_fn(scope, global_obj, b"__uzumaki_theme_changed__")?,
            hot_reload: get_global_fn(scope, global_obj, b"__uzumaki_hot_reload__")?,
        };

        let animation_frame =
            get_global_fn(scope, global_obj, b"__uzumaki_flush_animation_frame__")?;

        (dispatch, animation_frame)
    };

    let state: SharedJsState = Rc::new(RefCell::new(JsState::new(proxy.clone())));
    {
        let op_state = worker.js_runtime.op_state();
        let mut borrow = op_state.borrow_mut();
        borrow.put(state.clone());
        borrow.put(proxy.clone());
        borrow.put(config);
    }

    rt.block_on(async move {
        // Execute the entry module once. After this returns, the deno event
        // loop will keep running on subsequent `run_event_loop` calls inside
        // the main select.
        let specifier = deno_core::resolve_path(
            main_file
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("entry path is not valid utf-8"))?,
            &app_root,
        )?;
        worker.execute_main_module(&specifier).await?;

        run_main_loop(
            &mut worker,
            &state,
            &event_dispatch,
            &animation_frame_fn,
            &main_to_js,
        )
        .await
    })
}

async fn run_main_loop(
    worker: &mut MainWorker,
    state: &SharedJsState,
    dispatch: &JSGlobalEventDispatch,
    animation_frame_fn: &v8::Global<v8::Function>,
    main_to_js: &flume::Receiver<MainToJs>,
) -> Result<()> {
    loop {
        tokio::select! {
            biased;
            msg = main_to_js.recv_async() => {
                let Ok(msg) = msg else { break };
                if !handle_message(msg, worker, state, dispatch, animation_frame_fn) {
                    break;
                }
            }
            res = worker.run_event_loop(false) => {
                if let Err(e) = res {
                    print_runtime_error(&anyhow::Error::new(e));
                }
                // JS event loop is fully drained, park until the next
                // message. We don't re-enter `run_event_loop` because it
                // would return immediately and busy-loop.
                let Ok(msg) = main_to_js.recv_async().await else { break };
                if !handle_message(msg, worker, state, dispatch, animation_frame_fn) {
                    break;
                }
            }
        }

        flush_external_memory(worker, state);
        with_state(state, |s| s.drain_pending_destroy());
    }
    Ok(())
}

/// Returns `false` to break the main loop.
fn handle_message(
    msg: MainToJs,
    worker: &mut MainWorker,
    state: &SharedJsState,
    dispatch: &JSGlobalEventDispatch,
    animation_frame_fn: &v8::Global<v8::Function>,
) -> bool {
    match msg {
        MainToJs::WindowCreated { id, shared, theme } => {
            let proxy = with_state_ref(state, |s| s.proxy.clone());
            let window = window::Window::new(shared, proxy);
            with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&id) {
                    entry.window = Some(window);
                    if let Some(theme) = theme {
                        entry.state.theme = Some(theme);
                    }
                }
            });
            refresh_cursor_blink_timer(state, id);
            if let Some(theme) = theme {
                let theme = match theme {
                    Theme::Dark => "dark",
                    Theme::Light => "light",
                };
                dispatch.dispatch(
                    worker,
                    &AppEvent::ThemeChanged(events::UzThemeEvent {
                        window_id: id,
                        theme: theme.to_string(),
                    }),
                );
            }
            dispatch.dispatch(
                worker,
                &AppEvent::WindowLoad(events::UzWindowEvent { window_id: id }),
            );
        }
        MainToJs::BuildFrame { id } => {
            flush_animation_frame_callbacks(worker, animation_frame_fn, id);
            build_frame(state, id);
            let proxy = with_state_ref(state, |s| s.proxy.clone());
            let _ = proxy.send_event(UserEvent::FrameReady { id });
        }
        MainToJs::WindowEvent { id, event } => {
            handle_window_event(worker, state, dispatch, id, event);
        }
        MainToJs::CursorBlink { id, generation } => {
            handle_cursor_blink(state, id, generation);
        }
        MainToJs::DropJsWindow { id } => {
            with_state(state, |s| {
                s.windows.remove(&id);
            });
        }
        MainToJs::Resumed => {}
        MainToJs::Shutdown => return false,
    }
    true
}

fn flush_animation_frame_callbacks(
    worker: &mut MainWorker,
    animation_frame_fn: &v8::Global<v8::Function>,
    id: WindowEntryId,
) {
    deno_core::scope!(scope, &mut worker.js_runtime);
    v8::tc_scope!(scope, scope);

    let func = v8::Local::new(scope, animation_frame_fn);
    let undefined = v8::undefined(scope);
    let window_id = v8::Number::new(scope, id as f64);

    let _ = func.call(scope, undefined.into(), &[window_id.into()]);

    if let Some(exception) = scope.exception() {
        let error = deno_core::error::JsError::from_v8_exception(scope, exception);
        eprintln!("{} {error}", crate::terminal_colors::red_bold("Error"));
    }
}

/// Run layout + paint on the JS thread and park the resulting `Scene` in
/// `WindowShared::pending_frame` for main to present.
fn build_frame(state: &SharedJsState, id: WindowEntryId) {
    use crate::paint::render::Painter;

    with_state(state, |s| {
        let Some(entry) = s.windows.get_mut(&id) else {
            return;
        };
        let JsWindow { window, dom, .. } = entry;
        let Some(window) = window.as_mut() else {
            return;
        };
        let (pw, ph) = window.shared.load_inner_size();
        let scale = window.shared.load_scale_factor();
        if pw == 0 || ph == 0 {
            return;
        }

        let mut scene = vello::Scene::new();
        dom.compute_layout(pw as f32, ph as f32, &mut window.text_renderer, scale);
        dom.build_text_select_runs();
        let paint_start = std::time::Instant::now();
        Painter::new(dom, &mut window.text_renderer, scale).paint(&mut scene);
        if crate::perf::enabled() {
            crate::perf::record_paint(paint_start.elapsed());
        }
        dom.refresh_hit_test();

        *window.shared.pending_frame.lock().unwrap() = Some(scene);
    });
}

fn flush_external_memory(worker: &mut MainWorker, state: &SharedJsState) {
    let delta = with_state(state, |s| std::mem::take(&mut s.external_memory_delta));
    if delta != 0 {
        worker
            .js_runtime
            .v8_isolate()
            .adjust_amount_of_external_allocated_memory(delta);
    }
}

/// Numeric `EventType` discriminators, mirroring the `EventType` enum in
/// `js/events.ts`.
const ET_MOUSE_MOVE: f64 = 0.0;
const ET_MOUSE_DOWN: f64 = 1.0;
const ET_MOUSE_UP: f64 = 2.0;
const ET_CLICK: f64 = 3.0;
const ET_MOUSE_ENTER: f64 = 4.0;
const ET_MOUSE_LEAVE: f64 = 5.0;
const ET_MOUSE_OVER: f64 = 6.0;
const ET_MOUSE_OUT: f64 = 7.0;
const ET_KEY_DOWN: f64 = 10.0;
const ET_KEY_UP: f64 = 11.0;
const ET_INPUT: f64 = 20.0;
const ET_FOCUS: f64 = 21.0;
const ET_BLUR: f64 = 22.0;
const ET_BEFORE_INPUT: f64 = 23.0;
const ET_TEXT_UPDATE: f64 = 28.0;
const ET_COPY: f64 = 25.0;
const ET_CUT: f64 = 26.0;
const ET_PASTE: f64 = 27.0;

/// Global JS dispatch functions, one per event payload shape. Each is invoked
/// with primitive args instead of a serialized event object, so no serde round
/// trip happens on the hot path
pub struct JSGlobalEventDispatch {
    mouse: v8::Global<v8::Function>,
    keyboard: v8::Global<v8::Function>,
    input: v8::Global<v8::Function>,
    focus: v8::Global<v8::Function>,
    clipboard: v8::Global<v8::Function>,
    selectionchange: v8::Global<v8::Function>,
    resize: v8::Global<v8::Function>,
    window_load: v8::Global<v8::Function>,
    window_close: v8::Global<v8::Function>,
    theme_changed: v8::Global<v8::Function>,
    hot_reload: v8::Global<v8::Function>,
}

impl JSGlobalEventDispatch {
    /// Returns true if `preventDefault()` was called on the dispatched event.
    pub fn dispatch(&self, worker: &mut MainWorker, event: &AppEvent) -> bool {
        deno_core::scope!(scope, &mut worker.js_runtime);
        v8::tc_scope!(scope, scope);
        let undefined: v8::Local<v8::Value> = v8::undefined(scope).into();

        let (func, args): (&v8::Global<v8::Function>, Vec<v8::Local<v8::Value>>) = match event {
            AppEvent::Click(e) => (&self.mouse, mouse_args(scope, ET_CLICK, e)),
            AppEvent::MouseDown(e) => (&self.mouse, mouse_args(scope, ET_MOUSE_DOWN, e)),
            AppEvent::MouseUp(e) => (&self.mouse, mouse_args(scope, ET_MOUSE_UP, e)),
            AppEvent::MouseMove(e) => (&self.mouse, mouse_args(scope, ET_MOUSE_MOVE, e)),
            AppEvent::MouseEnter(e) => (&self.mouse, mouse_args(scope, ET_MOUSE_ENTER, e)),
            AppEvent::MouseLeave(e) => (&self.mouse, mouse_args(scope, ET_MOUSE_LEAVE, e)),
            AppEvent::MouseOver(e) => (&self.mouse, mouse_args(scope, ET_MOUSE_OVER, e)),
            AppEvent::MouseOut(e) => (&self.mouse, mouse_args(scope, ET_MOUSE_OUT, e)),
            AppEvent::KeyDown(e) => (&self.keyboard, keyboard_args(scope, ET_KEY_DOWN, e)),
            AppEvent::KeyUp(e) => (&self.keyboard, keyboard_args(scope, ET_KEY_UP, e)),
            AppEvent::Resize(e) => (
                &self.resize,
                vec![
                    v_num(scope, e.window_id as f64),
                    v_num(scope, e.width as f64),
                    v_num(scope, e.height as f64),
                ],
            ),
            AppEvent::Input(e) => (&self.input, input_args(scope, ET_INPUT, e)),
            AppEvent::BeforeInput(e) => (&self.input, input_args(scope, ET_BEFORE_INPUT, e)),
            AppEvent::TextUpdate(e) => (&self.input, input_args(scope, ET_TEXT_UPDATE, e)),
            AppEvent::Focus(e) => (&self.focus, focus_args(scope, ET_FOCUS, e)),
            AppEvent::Blur(e) => (&self.focus, focus_args(scope, ET_BLUR, e)),
            AppEvent::Copy(e) => (&self.clipboard, clipboard_args(scope, ET_COPY, e)),
            AppEvent::Cut(e) => (&self.clipboard, clipboard_args(scope, ET_CUT, e)),
            AppEvent::Paste(e) => (&self.clipboard, clipboard_args(scope, ET_PASTE, e)),
            AppEvent::SelectionChange(e) => (
                &self.selectionchange,
                vec![
                    v_num(scope, e.window_id as f64),
                    v_opt_node(scope, e.anchor_node_id),
                    v_num(scope, e.anchor_offset as f64),
                    v_opt_node(scope, e.focus_node_id),
                    v_num(scope, e.focus_offset as f64),
                    v_bool(scope, e.is_collapsed),
                ],
            ),
            AppEvent::WindowLoad(e) => (&self.window_load, vec![v_num(scope, e.window_id as f64)]),
            AppEvent::WindowClose(e) => {
                (&self.window_close, vec![v_num(scope, e.window_id as f64)])
            }
            AppEvent::ThemeChanged(e) => (
                &self.theme_changed,
                vec![
                    v_num(scope, e.window_id as f64),
                    v_bool(scope, e.theme == "dark"),
                ],
            ),
            AppEvent::HotReload => (&self.hot_reload, vec![]),
        };

        let func = v8::Local::new(scope, func);
        let result = func.call(scope, undefined, &args);

        if let Some(exception) = scope.exception() {
            let error = deno_core::error::JsError::from_v8_exception(scope, exception);
            eprintln!("{} {error}", crate::terminal_colors::red_bold("Error"));
            return false;
        }

        result.map(|v| v.is_true()).unwrap_or(false)
    }
}

fn get_global_fn(
    scope: &mut v8::PinScope<'_, '_>,
    global: v8::Local<v8::Object>,
    name: &'static [u8],
) -> Result<v8::Global<v8::Function>> {
    let key = v8::String::new_external_onebyte_static(scope, name)
        .ok_or_else(|| anyhow::anyhow!("failed to create v8 string"))?;
    let val = global.get(scope, key.into()).ok_or_else(|| {
        anyhow::anyhow!("{} not found on globalThis", String::from_utf8_lossy(name))
    })?;
    let func = v8::Local::<v8::Function>::try_from(val)
        .map_err(|_| anyhow::anyhow!("{} is not a function", String::from_utf8_lossy(name)))?;
    Ok(v8::Global::new(scope, func))
}

fn v_num<'s>(scope: &mut v8::PinScope<'s, '_>, n: f64) -> v8::Local<'s, v8::Value> {
    v8::Number::new(scope, n).into()
}

fn v_bool<'s>(scope: &mut v8::PinScope<'s, '_>, b: bool) -> v8::Local<'s, v8::Value> {
    v8::Boolean::new(scope, b).into()
}

fn v_str<'s>(scope: &mut v8::PinScope<'s, '_>, s: &str) -> v8::Local<'s, v8::Value> {
    v8::String::new(scope, s)
        .map(|v| v.into())
        .unwrap_or_else(|| v8::undefined(scope).into())
}

fn v_opt_str<'s>(scope: &mut v8::PinScope<'s, '_>, s: &Option<String>) -> v8::Local<'s, v8::Value> {
    match s {
        Some(s) => v_str(scope, s),
        None => v8::null(scope).into(),
    }
}

fn v_node<'s>(scope: &mut v8::PinScope<'s, '_>, id: UzNodeId) -> v8::Local<'s, v8::Value> {
    v8::Number::new(scope, id as f64).into()
}

fn v_opt_node<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    id: Option<UzNodeId>,
) -> v8::Local<'s, v8::Value> {
    match id {
        Some(id) => v_node(scope, id),
        None => v8::null(scope).into(),
    }
}

fn mouse_args<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    ty: f64,
    e: &events::UzMouseEvent,
) -> Vec<v8::Local<'s, v8::Value>> {
    vec![
        v_num(scope, ty),
        v_num(scope, e.window_id as f64),
        v_node(scope, e.node_id),
        v_num(scope, e.x as f64),
        v_num(scope, e.y as f64),
        v_num(scope, e.local_x as f64),
        v_num(scope, e.local_y as f64),
        v_num(scope, e.screen_x as f64),
        v_num(scope, e.screen_y as f64),
        v_num(scope, e.button as f64),
        v_num(scope, e.buttons.bits() as f64),
        v_opt_node(scope, e.related_node_id),
    ]
}

fn keyboard_args<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    ty: f64,
    e: &events::UzKeyboardEvent,
) -> Vec<v8::Local<'s, v8::Value>> {
    vec![
        v_num(scope, ty),
        v_num(scope, e.window_id as f64),
        v_opt_node(scope, e.node_id),
        v_str(scope, &e.key),
        v_str(scope, &e.code),
        v_num(scope, e.key_code as f64),
        v_num(scope, e.modifiers.bits() as f64),
        v_bool(scope, e.repeat),
    ]
}

fn input_args<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    ty: f64,
    e: &events::UzInputEvent,
) -> Vec<v8::Local<'s, v8::Value>> {
    vec![
        v_num(scope, ty),
        v_num(scope, e.window_id as f64),
        v_node(scope, e.node_id),
        v_str(scope, e.input_type),
        v_opt_str(scope, &e.data),
        v_opt_node(scope, e.start_node_id),
        v_num(scope, e.start_offset as f64),
        v_opt_node(scope, e.end_node_id),
        v_num(scope, e.end_offset as f64),
    ]
}

fn focus_args<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    ty: f64,
    e: &events::UzFocusEvent,
) -> Vec<v8::Local<'s, v8::Value>> {
    vec![
        v_num(scope, ty),
        v_num(scope, e.window_id as f64),
        v_node(scope, e.node_id),
    ]
}

fn clipboard_args<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    ty: f64,
    e: &events::UzClipboardEvent,
) -> Vec<v8::Local<'s, v8::Value>> {
    vec![
        v_num(scope, ty),
        v_num(scope, e.window_id as f64),
        v_opt_node(scope, e.node_id),
        v_opt_str(scope, &e.selection_text),
        v_opt_str(scope, &e.clipboard_text),
    ]
}

fn handle_window_event(
    worker: &mut MainWorker,
    state: &SharedJsState,
    dispatch: &JSGlobalEventDispatch,
    wid: WindowEntryId,
    event: winit::event::WindowEvent,
) {
    use winit::event::{Ime, WindowEvent};

    let mut needs_redraw = false;
    let mut refresh_blink_timer = false;

    match event {
        WindowEvent::Resized(_) => {
            // Main thread has already updated `shared.inner_size`
            let logical = with_state_ref(state, |s| {
                s.windows
                    .get(&wid)
                    .and_then(|e| e.window.as_ref().map(|w| w.inner_size()))
            });
            if let Some((w, h)) = logical
                && w > 0
                && h > 0
            {
                dispatch.dispatch(
                    worker,
                    &AppEvent::Resize(events::UzResizeEvent {
                        window_id: wid,
                        width: w,
                        height: h,
                    }),
                );
            }
        }
        WindowEvent::ScaleFactorChanged { .. } => {
            // Main has already updated `shared.scale_factor`
        }
        WindowEvent::CursorMoved { position, .. } => {
            let move_events = with_state(state, |s| {
                let entry = s.windows.get_mut(&wid)?;
                let mouse_buttons = entry.mouse_buttons;
                let JsWindow { window, dom, .. } = entry;
                let window = window.as_mut()?;
                let (redraw, events) =
                    events::handle_cursor_moved(dom, window, wid, position, mouse_buttons);
                if redraw {
                    needs_redraw = true;
                }
                Some(events)
            });
            if let Some(events) = move_events {
                for event in events {
                    dispatch.dispatch(worker, &event);
                }
            }
        }
        WindowEvent::MouseInput {
            state: btn_state,
            button,
            ..
        } => {
            refresh_blink_timer = true;
            let events = with_state(state, |s| {
                use events::MouseButtons;
                use winit::event::{ElementState, MouseButton};

                let button_bit = match button {
                    MouseButton::Left => MouseButtons::LEFT,
                    MouseButton::Right => MouseButtons::RIGHT,
                    MouseButton::Middle => MouseButtons::MIDDLE,
                    _ => MouseButtons::empty(),
                };

                let entry = s.windows.get_mut(&wid)?;
                match btn_state {
                    ElementState::Pressed => entry.mouse_buttons.insert(button_bit),
                    ElementState::Released => entry.mouse_buttons.remove(button_bit),
                }
                let mouse_buttons = entry.mouse_buttons;

                let JsWindow { window, dom, .. } = entry;
                let window = window.as_mut()?;
                let (redraw, mouse_events) =
                    events::handle_mouse_input(dom, window, wid, btn_state, button, mouse_buttons);
                if redraw {
                    needs_redraw = true;
                }
                Some(mouse_events)
            });

            if let Some(events) = events {
                for event in events {
                    dispatch.dispatch(worker, &event);
                }
            }
        }
        WindowEvent::KeyboardInput {
            event: key_event, ..
        } => {
            let modifiers = with_state_ref(state, |s| {
                s.windows.get(&wid).map(|e| e.modifiers).unwrap_or_default()
            });

            let raw_event = with_state_ref(state, |s| {
                s.windows.get(&wid).and_then(|entry| {
                    events::build_key_event(&entry.dom, wid, &key_event, modifiers)
                })
            });

            let prevented = if let Some(ref evt) = raw_event {
                dispatch.dispatch(worker, evt)
            } else {
                false
            };

            if !prevented {
                if let Some(AppEvent::HotReload) = raw_event {
                    // todo refresh
                } else {
                    let tab_outcome = with_state(state, |s| {
                        s.windows.get_mut(&wid).map(|entry| {
                            events::handle_tab_focus(&mut entry.dom, wid, &key_event, modifiers)
                        })
                    });
                    let tab_consumed = if let Some(outcome) = tab_outcome {
                        if outcome.needs_redraw {
                            needs_redraw = true;
                        }
                        for event in &outcome.events {
                            dispatch.dispatch(worker, event);
                        }
                        outcome.consumed
                    } else {
                        false
                    };

                    let clipboard_cmd = with_state(state, |s| {
                        let bridge = crate::clipboard::ClipboardBridge::new(&s.proxy);
                        s.windows.get(&wid).and_then(|entry| {
                            events::build_clipboard_command(
                                &entry.dom, &key_event, modifiers, &bridge,
                            )
                        })
                    });

                    let clipboard_consumed = if tab_consumed {
                        true
                    } else if let Some(cmd) = clipboard_cmd {
                        let clipboard_event = events::clipboard_command_to_event(&cmd, wid);
                        let clipboard_prevented = dispatch.dispatch(worker, &clipboard_event);

                        if !clipboard_prevented {
                            let (redraw, follow_up_events) = with_state(state, |s| {
                                let bridge = crate::clipboard::ClipboardBridge::new(&s.proxy);
                                if let Some(entry) = s.windows.get_mut(&wid) {
                                    let tr = entry.window.as_mut().map(|w| &mut w.text_renderer);
                                    if let Some(text_renderer) = tr {
                                        events::apply_clipboard_command(
                                            cmd,
                                            &mut entry.dom,
                                            wid,
                                            &bridge,
                                            text_renderer,
                                        )
                                    } else {
                                        (false, Vec::new())
                                    }
                                } else {
                                    (false, Vec::new())
                                }
                            });
                            if redraw {
                                needs_redraw = true;
                            }
                            for event in follow_up_events {
                                dispatch.dispatch(worker, &event);
                            }
                            if needs_redraw {
                                with_state(state, |s| {
                                    if let Some(entry) = s.windows.get_mut(&wid)
                                        && let Some(window) = entry.window.as_mut()
                                    {
                                        events::scroll_input_to_cursor(&mut entry.dom, window);
                                    }
                                });
                            }
                        }
                        true
                    } else {
                        false
                    };

                    if !clipboard_consumed {
                        // beforeinput fires before the edit commits; preventDefault
                        // cancels it without touching the editor.
                        let beforeinput = with_state_ref(state, |s| {
                            s.windows.get(&wid).and_then(|entry| {
                                events::build_beforeinput_event(
                                    &entry.dom, wid, &key_event, modifiers,
                                )
                            })
                        });
                        let input_prevented = if let Some(ref evt) = beforeinput {
                            dispatch.dispatch(worker, evt)
                        } else {
                            false
                        };

                        let input_events = with_state(state, |s| {
                            s.windows.get_mut(&wid).map(|entry| {
                                let window = entry.window.as_mut().unwrap();
                                let (redraw, events) = if input_prevented {
                                    (false, Vec::new())
                                } else {
                                    events::handle_key_for_input(
                                        &mut entry.dom,
                                        window,
                                        wid,
                                        &key_event,
                                        modifiers,
                                    )
                                };
                                let (checkbox_redraw, checkbox_events) =
                                    events::handle_key_for_checkbox(
                                        &mut entry.dom,
                                        wid,
                                        &key_event,
                                    );
                                let (button_redraw, button_events) =
                                    events::handle_key_for_button(&mut entry.dom, wid, &key_event);
                                let edit_context_events = if input_prevented {
                                    Vec::new()
                                } else {
                                    events::handle_key_for_edit_context(
                                        &mut entry.dom,
                                        wid,
                                        &key_event,
                                        modifiers,
                                    )
                                };
                                if redraw || checkbox_redraw || button_redraw {
                                    needs_redraw = true;
                                }
                                let mut all = events;
                                all.extend(checkbox_events);
                                all.extend(button_events);
                                all.extend(edit_context_events);
                                all
                            })
                        });

                        if let Some(events) = input_events {
                            for event in events {
                                dispatch.dispatch(worker, &event);
                            }
                        }

                        let view_sel_events = with_state(state, |s| {
                            let entry = s.windows.get_mut(&wid)?;
                            if entry.dom.focused_node.is_some() {
                                return None;
                            }
                            let (redraw, events) = events::handle_key_for_view_selection(
                                &mut entry.dom,
                                wid,
                                &key_event,
                                modifiers,
                            );
                            if redraw {
                                needs_redraw = true;
                            }
                            Some(events)
                        });
                        if let Some(events) = view_sel_events {
                            for event in events {
                                dispatch.dispatch(worker, &event);
                            }
                        }
                    }
                }
            }
            refresh_blink_timer = true;
        }
        WindowEvent::ModifiersChanged(mods) => {
            use events::KeyModifiers;
            let m = mods.state();
            let mut flags = KeyModifiers::empty();
            flags.set(KeyModifiers::CTRL, m.control_key());
            flags.set(KeyModifiers::ALT, m.alt_key());
            flags.set(KeyModifiers::SHIFT, m.shift_key());
            flags.set(KeyModifiers::SUPER, m.super_key());
            with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&wid) {
                    entry.modifiers = flags;
                }
            });
        }
        WindowEvent::Focused(focused) => {
            with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&wid) {
                    entry.state.focused = focused;
                    entry.dom.window_focused = focused;
                    if focused {
                        entry.dom.caret_blink.reset();
                    }
                    if focused && let Some(window) = entry.window.as_mut() {
                        events::update_ime_cursor_area(&mut entry.dom, window);
                    }
                    needs_redraw = true;
                }
            });
            refresh_blink_timer = true;
        }
        WindowEvent::Moved(pos) => {
            with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&wid) {
                    entry.state.outer_position = Some(pos);
                }
            });
        }
        WindowEvent::ThemeChanged(theme) => {
            with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&wid) {
                    entry.state.theme = Some(theme);
                }
            });
            let theme = match theme {
                Theme::Dark => "dark",
                Theme::Light => "light",
            };
            dispatch.dispatch(
                worker,
                &AppEvent::ThemeChanged(events::UzThemeEvent {
                    window_id: wid,
                    theme: theme.to_string(),
                }),
            );
        }
        WindowEvent::Ime(ime) => match ime {
            Ime::Commit(text) => {
                let input_events = with_state(state, |s| {
                    s.windows.get_mut(&wid).and_then(|entry| {
                        let window = entry.window.as_mut()?;
                        let fid = entry.dom.focused_node?;

                        if let Some(meta) = events::input_layout_meta(&entry.dom, fid)
                            && let Some(node) = entry.dom.nodes.get_mut(fid)
                            && let Some(is) = node.as_text_input_mut()
                        {
                            crate::text::apply_text_style_to_editor(
                                &mut is.editor,
                                &meta.text_style,
                            );
                            is.editor.set_width(if meta.multiline {
                                Some(meta.input_width)
                            } else {
                                None
                            });
                        }

                        let node = entry.dom.nodes.get_mut(fid)?;
                        let is = node.as_text_input_mut()?;
                        let _edit = is.commit_ime_text(&text, &mut window.text_renderer)?;
                        events::update_ime_cursor_area(&mut entry.dom, window);
                        needs_redraw = true;
                        Some(vec![events::AppEvent::Input(events::UzInputEvent::plain(
                            wid,
                            fid,
                            "insertCompositionText",
                            Some(text.clone()),
                        ))])
                    })
                });
                if let Some(events) = input_events {
                    for event in events {
                        dispatch.dispatch(worker, &event);
                    }
                }
                refresh_blink_timer = true;
            }
            Ime::Preedit(text, cursor) => {
                with_state(state, |s| {
                    if let Some(entry) = s.windows.get_mut(&wid)
                        && let Some(fid) = entry.dom.focused_node
                        && let Some(node) = entry.dom.nodes.get_mut(fid)
                        && let Some(is) = node.as_text_input_mut()
                    {
                        is.set_preedit(text.clone(), cursor);
                        if let Some(window) = entry.window.as_mut() {
                            events::update_ime_cursor_area(&mut entry.dom, window);
                        }
                        needs_redraw = true;
                    }
                });
                refresh_blink_timer = true;
            }
            Ime::Enabled => {}
            Ime::Disabled => {
                with_state(state, |s| {
                    if let Some(entry) = s.windows.get_mut(&wid)
                        && let Some(fid) = entry.dom.focused_node
                        && let Some(node) = entry.dom.nodes.get_mut(fid)
                        && let Some(is) = node.as_text_input_mut()
                    {
                        is.clear_preedit();
                        if let Some(window) = entry.window.as_mut() {
                            events::update_ime_cursor_area(&mut entry.dom, window);
                        }
                        needs_redraw = true;
                    }
                });
                refresh_blink_timer = true;
            }
        },
        WindowEvent::CursorLeft { .. } => {
            with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&wid) {
                    entry.dom.hit_state = Default::default();
                    if let Some(window) = entry.window.as_mut() {
                        window.set_cursor(UzCursorIcon::Default);
                    }
                    needs_redraw = true;
                }
            });
        }
        WindowEvent::MouseWheel { delta, .. } => {
            with_state(state, |s| {
                let (mut dx, mut dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        ((x as f64) * 40.0, (y as f64) * 40.0)
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
                };
                if let Some(entry) = s.windows.get_mut(&wid) {
                    if entry.modifiers.contains(events::KeyModifiers::SHIFT) && dx == 0.0 {
                        dx = dy;
                        dy = 0.0;
                    }
                    if let Some(window) = entry.window.as_mut()
                        && events::handle_mouse_wheel(&mut entry.dom, window, dx, dy)
                    {
                        needs_redraw = true;
                    }
                }
            });
        }
        WindowEvent::CloseRequested => {
            dispatch.dispatch(
                worker,
                &AppEvent::WindowClose(events::UzWindowEvent { window_id: wid }),
            );
            // Drop the native window handle so the OS window goes away
            // immediately, but keep the JsWindow (DOM, mirror, etc.) in state.
            // The React reconciler's unmount commit is queued as a microtask;
            // it needs the entry to still exist when ops fire from
            // `detachDeletedInstance`. Main will bounce `DropJsWindow` back
            // after GPU teardown, by which time the commit has drained.
            let proxy = with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&wid) {
                    entry.window = None;
                    if let Some(timer) = entry.blink_timer.take() {
                        timer.abort();
                    }
                }
                s.proxy.clone()
            });
            let _ = proxy.send_event(UserEvent::CloseWindow { id: wid });
            return;
        }
        _ => {}
    }

    if refresh_blink_timer {
        with_state(state, |s| {
            if let Some(entry) = s.windows.get_mut(&wid) {
                entry.dom.caret_blink.reset();
            }
        });
        refresh_cursor_blink_timer(state, wid);
        needs_redraw |= with_state_ref(state, |s| {
            s.windows
                .get(&wid)
                .is_some_and(|e| e.dom.next_caret_blink_in(e.dom.window_focused).is_some())
        });
    }

    if needs_redraw {
        with_state_ref(state, |s| {
            if let Some(entry) = s.windows.get(&wid)
                && let Some(window) = entry.window.as_ref()
            {
                window.request_redraw();
            }
        });
    }
}

fn refresh_cursor_blink_timer(state: &SharedJsState, id: WindowEntryId) {
    // Cancel the in-flight timer (if any) before computing the next one. The
    // generation counter still guards against an event that's already been
    // sent through the proxy but hasn't been pumped back yet.
    let (next_timer, proxy) = with_state(state, |s| {
        let proxy = s.proxy.clone();
        let Some(entry) = s.windows.get_mut(&id) else {
            return (None, proxy);
        };
        if let Some(prev) = entry.blink_timer.take() {
            prev.abort();
        }
        entry.cursor_blink_generation = entry.cursor_blink_generation.wrapping_add(1);
        let generation = entry.cursor_blink_generation;
        let focused = entry.dom.window_focused;
        let next_delay = entry.dom.next_caret_blink_in(focused);
        (next_delay.map(|delay| (generation, delay)), proxy)
    });

    if let Some((generation, delay)) = next_timer {
        let task = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = proxy.send_event(UserEvent::CursorBlink { id, generation });
        });
        with_state(state, |s| {
            if let Some(entry) = s.windows.get_mut(&id) {
                entry.blink_timer = Some(task);
            } else {
                task.abort();
            }
        });
    }
}

/// Handle a CursorBlink tick forwarded from main. The blink timer task lives
/// on tokio (JS thread) but can't capture non-Send JS state, so it sends a
/// `UserEvent::CursorBlink` to main, which round-trips back here via
/// `MainToJs::CursorBlink`.
pub fn handle_cursor_blink(state: &SharedJsState, id: WindowEntryId, generation: u64) {
    let should_redraw = with_state_ref(state, |s| {
        s.windows
            .get(&id)
            .filter(|entry| entry.cursor_blink_generation == generation)
            .and_then(|entry| entry.dom.next_caret_blink_in(entry.dom.window_focused))
            .is_some()
    });

    if should_redraw {
        with_state_ref(state, |s| {
            if let Some(entry) = s.windows.get(&id)
                && let Some(window) = entry.window.as_ref()
            {
                window.request_redraw();
            }
        });
        refresh_cursor_blink_timer(state, id);
    }
}
