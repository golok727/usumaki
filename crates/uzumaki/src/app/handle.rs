use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use vello::Scene;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::window::{Window as WinitWindow, WindowButtons, WindowLevel};

use crate::cursor::UzCursorIcon;
use crate::node::UzNodeId;
use crate::ops::window::WindowOptions;

pub type WindowEntryId = u32;

/// Thin wrapper around `Arc<winit::window::Window>` exposing only the methods
/// safe to call from the JS thread. winit's `Window` is `Send + Sync`, but we
/// deliberately restrict the surface so JS can't accidentally drive
/// platform-specific APIs that should be funneled through `UserEvent`.
#[derive(Clone)]
pub struct WinitHandle {
    window: Arc<WinitWindow>,
}

impl WinitHandle {
    pub fn new(window: Arc<WinitWindow>) -> Self {
        Self { window }
    }

    pub fn id(&self) -> winit::window::WindowId {
        self.window.id()
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

/// Per-window state shared between the main (winit/GPU) thread and the JS
/// thread. The JS thread reads `inner_size` and `scale_factor` via atomics,
/// publishes built frames into `pending_frame`, and uses `winit` for redraws.
pub struct WindowShared {
    pub window_id: WindowEntryId,
    pub winit: WinitHandle,
    /// (width as u64) << 32 | (height as u64). Logical pixels.
    pub inner_size: AtomicU64,
    /// f64::to_bits of the device scale factor.
    pub scale_factor: AtomicU64,
    pub pending_frame: Mutex<Option<Scene>>,
}

impl WindowShared {
    pub fn new(
        window_id: WindowEntryId,
        winit: WinitHandle,
        inner_size: (u32, u32),
        scale_factor: f64,
    ) -> Self {
        Self {
            window_id,
            winit,
            inner_size: AtomicU64::new(pack_size(inner_size.0, inner_size.1)),
            scale_factor: AtomicU64::new(scale_factor.to_bits()),
            pending_frame: Mutex::new(None),
        }
    }

    pub fn load_inner_size(&self) -> (u32, u32) {
        unpack_size(self.inner_size.load(Ordering::Acquire))
    }

    pub fn store_inner_size(&self, width: u32, height: u32) {
        self.inner_size
            .store(pack_size(width, height), Ordering::Release);
    }

    pub fn load_scale_factor(&self) -> f64 {
        f64::from_bits(self.scale_factor.load(Ordering::Acquire))
    }

    pub fn store_scale_factor(&self, scale: f64) {
        self.scale_factor.store(scale.to_bits(), Ordering::Release);
    }
}

fn pack_size(w: u32, h: u32) -> u64 {
    ((w as u64) << 32) | (h as u64)
}

fn unpack_size(packed: u64) -> (u32, u32) {
    ((packed >> 32) as u32, packed as u32)
}

/// Events the JS thread sends back to the main winit thread.
///
/// All winit calls other than `id()` and `request_redraw()` go through this
/// channel — set_cursor, IME area, window-attribute setters, lifecycle, and
/// clipboard reads/writes (which use OS APIs that must run on main on macOS).
pub enum UserEvent {
    CreateWindow {
        id: WindowEntryId,
        options: WindowOptions,
    },
    CloseWindow {
        id: WindowEntryId,
    },
    FrameReady {
        id: WindowEntryId,
    },
    AnimationFramePending {
        id: WindowEntryId,
        pending: bool,
    },
    SetCursor {
        id: WindowEntryId,
        icon: UzCursorIcon,
    },
    SetImeArea {
        id: WindowEntryId,
        position: LogicalPosition<f64>,
        size: LogicalSize<f32>,
    },
    SetTitle {
        id: WindowEntryId,
        title: String,
    },
    SetVisible {
        id: WindowEntryId,
        visible: bool,
    },
    SetResizable {
        id: WindowEntryId,
        resizable: bool,
    },
    SetDecorations {
        id: WindowEntryId,
        decorations: bool,
    },
    SetTransparent {
        id: WindowEntryId,
        transparent: bool,
    },
    SetMaximized {
        id: WindowEntryId,
        maximized: bool,
    },
    SetMinimized {
        id: WindowEntryId,
        minimized: bool,
    },
    SetFullscreen {
        id: WindowEntryId,
        fullscreen: bool,
    },
    SetWindowLevel {
        id: WindowEntryId,
        level: WindowLevel,
    },
    SetMinSize {
        id: WindowEntryId,
        size: LogicalSize<f64>,
    },
    SetMaxSize {
        id: WindowEntryId,
        size: LogicalSize<f64>,
    },
    SetPosition {
        id: WindowEntryId,
        position: LogicalPosition<f64>,
    },
    SetTheme {
        id: WindowEntryId,
        theme: Option<winit::window::Theme>,
    },
    SetContentProtected {
        id: WindowEntryId,
        protected: bool,
    },
    SetEnabledButtons {
        id: WindowEntryId,
        buttons: WindowButtons,
    },
    Focus {
        id: WindowEntryId,
    },
    CursorBlink {
        id: WindowEntryId,
        generation: u64,
    },
    ClipboardRead {
        reply: flume::Sender<Option<String>>,
    },
    ClipboardWrite {
        text: String,
        reply: flume::Sender<bool>,
    },
    Quit,
}

/// Events the main thread sends to the JS thread.
pub enum MainToJs {
    WindowCreated {
        id: WindowEntryId,
        shared: Arc<WindowShared>,
        theme: Option<winit::window::Theme>,
    },
    WindowEvent {
        id: WindowEntryId,
        event: winit::event::WindowEvent,
    },
    /// Deno's main module is loaded on the JS thread once on first poll;
    /// after that the winit `resumed` callback no longer needs to message us.
    Resumed,
    /// A `RedrawRequested` arrived from winit; build the frame and reply with
    /// `UserEvent::FrameReady`.
    BuildFrame {
        id: WindowEntryId,
    },
    /// Forwarded from main after winit delivers a `UserEvent::CursorBlink`.
    /// The blink scheduler lives JS-side but the timer task can't capture
    /// non-Send JS state, so it round-trips via the proxy.
    CursorBlink {
        id: WindowEntryId,
        generation: u64,
    },
    /// Sent by main after a window has finished closing (GPU resources
    /// dropped). Triggers the JS thread to remove its `JsWindow` entry. The
    /// round-trip lets any deferred React commit / microtask work run with
    /// the entry still present before its DOM is dropped.
    DropJsWindow {
        id: WindowEntryId,
    },
    Shutdown,
}

/// Per-window finalizer requests deferred from cppgc. Drained on the JS
/// thread; never touches the main thread.
pub type PendingDestroy = (WindowEntryId, UzNodeId);
