use deno_core::*;
use refineable::Refineable;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::EventLoopProxy;
use winit::window::{
    Fullscreen, Theme, Window as WinitWindow, WindowAttributes, WindowButtons, WindowLevel,
};

use crate::app::{JsWindow, SharedJsState, UserEvent, WindowEntryId, with_state, with_state_ref};

const DEFAULT_MIN_WINDOW_WIDTH: f64 = 400.0;
const DEFAULT_MIN_WINDOW_HEIGHT: f64 = 300.0;

#[derive(Clone, Debug, PartialEq, Refineable, serde::Deserialize)]
#[refineable(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowOptions {
    width: u32,
    height: u32,
    title: String,
    visible: bool,
    resizable: bool,
    decorations: bool,
    transparent: bool,
    maximized: bool,
    minimized: bool,
    fullscreen: bool,
    window_level: Option<UzWindowLevel>,
    min_width: Option<f64>,
    min_height: Option<f64>,
    max_width: Option<f64>,
    max_height: Option<f64>,
    position: Option<WindowPosition>,
    theme: Option<WindowTheme>,
    active: Option<bool>,
    content_protected: bool,
    closable: bool,
    minimizable: bool,
    maximizable: bool,
}

pub(crate) type CreateWindowOptions = WindowOptionsRefinement;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowPosition {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum WindowTheme {
    Light,
    Dark,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
enum UzWindowLevel {
    Normal,
    AlwaysOnTop,
    AlwaysOnBottom,
}

impl UzWindowLevel {
    fn as_str(&self) -> &str {
        match self {
            UzWindowLevel::Normal => "normal",
            UzWindowLevel::AlwaysOnTop => "alwaysOnTop",
            UzWindowLevel::AlwaysOnBottom => "alwaysOnBottom",
        }
    }
}

impl WindowTheme {
    fn as_str(&self) -> &str {
        match self {
            WindowTheme::Light => "light",
            WindowTheme::Dark => "dark",
            WindowTheme::System => "system",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowSize {
    width: u32,
    height: u32,
}

impl WindowPosition {
    fn to_logical_position(self) -> LogicalPosition<f64> {
        LogicalPosition::new(self.x, self.y)
    }
}

impl WindowTheme {
    fn to_winit(self) -> Option<Theme> {
        match self {
            Self::Light => Some(Theme::Light),
            Self::Dark => Some(Theme::Dark),
            Self::System => None,
        }
    }

    fn from_winit(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self::Light,
            Theme::Dark => Self::Dark,
        }
    }
}

impl UzWindowLevel {
    fn to_winit(self) -> WindowLevel {
        match self {
            Self::Normal => WindowLevel::Normal,
            Self::AlwaysOnTop => WindowLevel::AlwaysOnTop,
            Self::AlwaysOnBottom => WindowLevel::AlwaysOnBottom,
        }
    }

    fn from_winit(level: WindowLevel) -> Self {
        match level {
            WindowLevel::Normal => Self::Normal,
            WindowLevel::AlwaysOnTop => Self::AlwaysOnTop,
            WindowLevel::AlwaysOnBottom => Self::AlwaysOnBottom,
        }
    }
}

impl<'a> From<&'a str> for UzWindowLevel {
    fn from(value: &'a str) -> Self {
        match value {
            "normal" => UzWindowLevel::Normal,
            "alwaysOnTop" => UzWindowLevel::AlwaysOnTop,
            "alwaysOnBottom" => UzWindowLevel::AlwaysOnBottom,
            _ => UzWindowLevel::Normal,
        }
    }
}

impl<'a> From<&'a str> for WindowTheme {
    fn from(value: &'a str) -> Self {
        match value {
            "light" => WindowTheme::Light,
            "dark" => WindowTheme::Dark,
            "system" => WindowTheme::System,
            _ => WindowTheme::System,
        }
    }
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            title: "uzumaki".to_string(),
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            maximized: false,
            minimized: false,
            fullscreen: false,
            window_level: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            position: None,
            theme: None,
            active: None,
            content_protected: false,
            closable: true,
            minimizable: true,
            maximizable: true,
        }
    }
}

impl WindowOptions {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn resizable(&self) -> bool {
        self.resizable
    }

    pub fn decorations(&self) -> bool {
        self.decorations
    }

    pub fn transparent(&self) -> bool {
        self.transparent
    }

    pub fn maximized(&self) -> bool {
        self.maximized
    }

    pub fn minimized(&self) -> bool {
        self.minimized
    }

    pub fn fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub fn window_level(&self) -> WindowLevel {
        self.window_level
            .map(UzWindowLevel::to_winit)
            .unwrap_or(WindowLevel::Normal)
    }

    pub fn enabled_buttons(&self) -> WindowButtons {
        window_buttons(self.closable, self.minimizable, self.maximizable)
    }

    pub fn content_protected(&self) -> bool {
        self.content_protected
    }

    pub fn theme_winit(&self) -> Option<Theme> {
        self.theme.and_then(WindowTheme::to_winit)
    }

    pub fn to_window_attributes(&self) -> WindowAttributes {
        let mut attributes = WindowAttributes::default()
            .with_title(self.title.clone())
            .with_inner_size(LogicalSize::new(self.width as f64, self.height as f64))
            .with_visible(self.visible)
            .with_resizable(self.resizable)
            .with_decorations(self.decorations)
            .with_transparent(self.transparent)
            .with_maximized(self.maximized)
            .with_window_level(self.window_level())
            .with_content_protected(self.content_protected)
            .with_enabled_buttons(self.enabled_buttons());

        if self.fullscreen {
            attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let default_min_size =
            LogicalSize::new(DEFAULT_MIN_WINDOW_WIDTH, DEFAULT_MIN_WINDOW_HEIGHT);
        if let Some(min_size) = try_logical_size(self.min_width, self.min_height) {
            attributes = attributes.with_min_inner_size(min_size);
        } else {
            attributes = attributes.with_min_inner_size(default_min_size);
        }
        if let Some(max_size) = try_logical_size(self.max_width, self.max_height) {
            attributes = attributes.with_max_inner_size(max_size);
        }
        if let Some(position) = self.position {
            attributes = attributes.with_position(position.to_logical_position());
        }
        if let Some(theme) = self.theme {
            attributes = attributes.with_theme(theme.to_winit());
        }
        if let Some(active) = self.active {
            attributes = attributes.with_active(active);
        }
        attributes
    }

    pub fn apply_post_create_state(&self, window: &WinitWindow) {
        if self.minimized {
            window.set_minimized(true);
        }
    }
}

fn try_logical_size(width: Option<f64>, height: Option<f64>) -> Option<LogicalSize<f64>> {
    match (width, height) {
        (Some(width), Some(height))
            if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 =>
        {
            Some(LogicalSize::new(width, height))
        }
        _ => None,
    }
}

fn window_buttons(closable: bool, minimizable: bool, maximizable: bool) -> WindowButtons {
    let mut buttons = WindowButtons::empty();
    if closable {
        buttons |= WindowButtons::CLOSE;
    }
    if minimizable {
        buttons |= WindowButtons::MINIMIZE;
    }
    if maximizable {
        buttons |= WindowButtons::MAXIMIZE;
    }
    buttons
}

#[op2]
#[cppgc]
pub fn op_create_window(
    state: &mut OpState,
    #[serde] options: CreateWindowOptions,
) -> Result<CoreWindow, deno_error::JsErrorBox> {
    static NEXT_WINDOW_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
    let id = NEXT_WINDOW_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let options = WindowOptions::default().refined(options);

    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        s.windows.insert(id, JsWindow::new(&options));
    });

    let proxy = state.borrow::<EventLoopProxy<UserEvent>>();
    proxy
        .send_event(UserEvent::CreateWindow { id, options })
        .map_err(|_| {
            deno_error::JsErrorBox::new(
                "UzumakiInternalError",
                "cannot create window after application free",
            )
        })?;

    Ok(CoreWindow::new(id))
}

#[op2(fast)]
pub fn op_request_quit(state: &mut OpState) -> Result<(), deno_error::JsErrorBox> {
    let proxy = state.borrow::<EventLoopProxy<UserEvent>>();
    proxy
        .send_event(UserEvent::Quit)
        .map_err(|_| deno_error::JsErrorBox::new("UzumakiInternalError", "error quitting"))?;
    Ok(())
}

#[op2(fast)]
pub fn op_request_redraw(
    state: &mut OpState,
    #[smi] window_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state_ref(&js_state, |s| {
        if let Some(entry) = s.windows.get(&window_id)
            && let Some(window) = entry.window.as_ref()
        {
            window.request_redraw();
        }
    });
    Ok(())
}

#[op2]
#[string]
pub fn op_read_clipboard_text(
    state: std::rc::Rc<std::cell::RefCell<OpState>>,
) -> impl std::future::Future<Output = Option<String>> {
    let proxy = state.borrow().borrow::<EventLoopProxy<UserEvent>>().clone();
    async move {
        let (reply, rx) = flume::bounded(1);
        proxy.send_event(UserEvent::ClipboardRead { reply }).ok()?;
        rx.recv_async().await.ok().flatten()
    }
}

#[op2]
pub fn op_write_clipboard_text(
    state: std::rc::Rc<std::cell::RefCell<OpState>>,
    #[string] text: String,
) -> impl std::future::Future<Output = bool> {
    let proxy = state.borrow().borrow::<EventLoopProxy<UserEvent>>().clone();
    async move {
        let (reply, rx) = flume::bounded(1);
        if proxy
            .send_event(UserEvent::ClipboardWrite { text, reply })
            .is_err()
        {
            return false;
        }
        rx.recv_async().await.unwrap_or(false)
    }
}

use deno_core::GarbageCollected;

pub struct CoreWindow {
    id: WindowEntryId,
}

impl CoreWindow {
    pub fn new(id: WindowEntryId) -> Self {
        Self { id }
    }

    fn with_entry<R>(&self, state: &OpState, f: impl FnOnce(&JsWindow) -> R) -> Option<R> {
        let js = state.borrow::<SharedJsState>().clone();
        with_state_ref(&js, |s| s.windows.get(&self.id).map(f))
    }

    fn with_entry_mut<R>(&self, state: &OpState, f: impl FnOnce(&mut JsWindow) -> R) -> Option<R> {
        let js = state.borrow::<SharedJsState>().clone();
        with_state(&js, |s| s.windows.get_mut(&self.id).map(f))
    }

    fn proxy_send(&self, state: &OpState, event: UserEvent) -> bool {
        state
            .borrow::<EventLoopProxy<UserEvent>>()
            .send_event(event)
            .is_ok()
    }

    /// Sync the local `WindowMirror` and forward the matching `UserEvent` to
    /// the main thread in one step. The closure returns the event to send
    /// after applying its mirror update; if the entry doesn't exist (window
    /// already closed) the event is skipped.
    fn sync_and_send(
        &self,
        state: &OpState,
        apply: impl FnOnce(&mut JsWindow) -> UserEvent,
    ) -> bool {
        match self.with_entry_mut(state, apply) {
            Some(event) => self.proxy_send(state, event),
            None => false,
        }
    }

    fn toggle_button(&self, state: &OpState, button: WindowButtons, enabled: bool) -> bool {
        let id = self.id;
        self.sync_and_send(state, |entry| {
            if enabled {
                entry.state.enabled_buttons |= button;
            } else {
                entry.state.enabled_buttons &= !button;
            }
            UserEvent::SetEnabledButtons {
                id,
                buttons: entry.state.enabled_buttons,
            }
        })
    }
}

unsafe impl GarbageCollected for CoreWindow {
    fn trace(&self, _visitor: &mut deno_core::v8::cppgc::Visitor) {}

    fn get_name(&self) -> &'static std::ffi::CStr {
        c"CoreWindow"
    }
}

#[op2]
#[allow(non_snake_case)]
impl CoreWindow {
    #[getter]
    pub fn id(&self) -> WindowEntryId {
        self.id
    }

    #[fast]
    pub fn close(&self, state: &OpState) -> Result<(), deno_error::JsErrorBox> {
        self.proxy_send(state, UserEvent::CloseWindow { id: self.id });
        Ok(())
    }

    /** inner width in logical pixels */
    #[getter]
    pub fn innerWidth(&self, state: &OpState) -> Option<u32> {
        self.with_entry(state, |entry| entry.inner_size().map(|(w, _)| w))
            .flatten()
    }

    /** inner height in logical pixels */
    #[getter]
    pub fn innerHeight(&self, state: &OpState) -> Option<u32> {
        self.with_entry(state, |entry| entry.inner_size().map(|(_, h)| h))
            .flatten()
    }

    #[getter]
    #[string]
    pub fn title(&self, state: &OpState) -> Option<String> {
        self.with_entry(state, |entry| entry.state.title.clone())
    }

    #[fast]
    #[setter]
    pub fn title(&self, state: &OpState, #[string] title: String) -> bool {
        let id = self.id;
        self.sync_and_send(state, move |entry| {
            entry.state.title = title.clone();
            UserEvent::SetTitle { id, title }
        })
    }

    #[getter]
    pub fn visible(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| entry.state.visible)
    }

    #[fast]
    #[setter]
    pub fn visible(&self, state: &OpState, visible: bool) -> bool {
        let id = self.id;
        self.sync_and_send(state, move |entry| {
            entry.state.visible = visible;
            UserEvent::SetVisible { id, visible }
        })
    }

    #[getter]
    pub fn transparent(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| entry.state.transparent)
    }

    #[fast]
    #[setter]
    pub fn transparent(&self, state: &OpState, transparent: bool) -> bool {
        let id = self.id;
        self.sync_and_send(state, move |entry| {
            entry.state.transparent = transparent;
            UserEvent::SetTransparent { id, transparent }
        })
    }

    #[getter]
    pub fn resizable(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| entry.state.resizable)
    }

    #[fast]
    #[setter]
    pub fn resizable(&self, state: &OpState, resizable: bool) -> bool {
        let id = self.id;
        self.sync_and_send(state, move |entry| {
            entry.state.resizable = resizable;
            UserEvent::SetResizable { id, resizable }
        })
    }

    #[getter]
    pub fn decorations(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| entry.state.decorations)
    }

    #[fast]
    #[setter]
    pub fn decorations(&self, state: &OpState, decorations: bool) -> bool {
        let id = self.id;
        self.sync_and_send(state, move |entry| {
            entry.state.decorations = decorations;
            UserEvent::SetDecorations { id, decorations }
        })
    }

    #[getter]
    pub fn maximized(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| entry.state.maximized)
    }

    #[fast]
    #[setter]
    pub fn maximized(&self, state: &OpState, maximized: bool) -> bool {
        let id = self.id;
        self.sync_and_send(state, move |entry| {
            entry.state.maximized = maximized;
            UserEvent::SetMaximized { id, maximized }
        })
    }

    #[getter]
    pub fn minimized(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| entry.state.minimized)
    }

    #[fast]
    #[setter]
    pub fn minimized(&self, state: &OpState, minimized: bool) -> bool {
        let id = self.id;
        self.sync_and_send(state, move |entry| {
            entry.state.minimized = minimized;
            UserEvent::SetMinimized { id, minimized }
        })
    }

    #[getter]
    pub fn fullscreen(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| entry.state.fullscreen)
    }

    #[fast]
    #[setter]
    pub fn fullscreen(&self, state: &OpState, fullscreen: bool) -> bool {
        let id = self.id;
        self.sync_and_send(state, move |entry| {
            entry.state.fullscreen = fullscreen;
            UserEvent::SetFullscreen { id, fullscreen }
        })
    }

    #[getter]
    #[string]
    pub fn windowLevel(&self, state: &OpState) -> Option<String> {
        self.with_entry(state, |entry| {
            UzWindowLevel::from_winit(entry.state.window_level)
                .as_str()
                .to_string()
        })
    }

    #[fast]
    #[setter]
    pub fn windowLevel(&self, state: &OpState, #[string] level: &str) -> bool {
        let level = UzWindowLevel::from(level).to_winit();
        let id = self.id;
        self.sync_and_send(state, move |entry| {
            entry.state.window_level = level;
            UserEvent::SetWindowLevel { id, level }
        })
    }

    #[fast]
    pub fn setMinSize(&self, state: &OpState, width: f64, height: f64) -> bool {
        let Some(size) = try_logical_size(Some(width), Some(height)) else {
            return false;
        };
        self.proxy_send(state, UserEvent::SetMinSize { id: self.id, size })
    }

    #[fast]
    pub fn setMaxSize(&self, state: &OpState, width: f64, height: f64) -> bool {
        let Some(size) = try_logical_size(Some(width), Some(height)) else {
            return false;
        };
        self.proxy_send(state, UserEvent::SetMaxSize { id: self.id, size })
    }

    #[getter]
    #[serde]
    pub fn innerSize(&self, state: &OpState) -> Option<WindowSize> {
        self.with_entry(state, |entry| {
            entry.inner_size().map(|(w, h)| WindowSize {
                width: w,
                height: h,
            })
        })
        .flatten()
    }

    #[getter]
    #[serde]
    pub fn outerSize(&self, state: &OpState) -> Option<WindowSize> {
        self.with_entry(state, |entry| {
            let scale = entry.scale_factor()? as f64;
            let outer = entry.state.outer_size?;
            Some(WindowSize {
                width: (outer.width as f64 / scale).round() as u32,
                height: (outer.height as f64 / scale).round() as u32,
            })
        })
        .flatten()
    }

    #[getter]
    #[serde]
    pub fn position(&self, state: &OpState) -> Option<WindowPosition> {
        self.with_entry(state, |entry| {
            let scale = entry.scale_factor()? as f64;
            let pos = entry.state.outer_position?;
            Some(WindowPosition {
                x: pos.x as f64 / scale,
                y: pos.y as f64 / scale,
            })
        })
        .flatten()
    }

    #[fast]
    pub fn setPosition(&self, state: &OpState, x: f64, y: f64) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        self.proxy_send(
            state,
            UserEvent::SetPosition {
                id: self.id,
                position: LogicalPosition::new(x, y),
            },
        )
    }

    #[getter]
    pub fn scaleFactor(&self, state: &OpState) -> Option<f32> {
        self.with_entry(state, |entry| entry.scale_factor())
            .flatten()
    }

    #[getter]
    #[string]
    pub fn theme(&self, state: &OpState) -> Option<String> {
        self.with_entry(state, |entry| {
            entry
                .state
                .theme
                .map(|t| WindowTheme::from_winit(t).as_str().to_string())
        })
        .flatten()
    }

    #[fast]
    #[setter]
    pub fn theme(&self, state: &OpState, #[string] theme: &str) -> bool {
        let theme = WindowTheme::from(theme).to_winit();
        let id = self.id;
        self.sync_and_send(state, move |entry| {
            entry.state.theme = theme;
            UserEvent::SetTheme { id, theme }
        })
    }

    #[getter]
    pub fn active(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| entry.state.focused)
    }

    #[fast]
    pub fn focus(&self, state: &OpState) -> bool {
        self.proxy_send(state, UserEvent::Focus { id: self.id })
    }

    #[fast]
    pub fn setAnimationFramePending(&self, state: &OpState, pending: bool) -> bool {
        self.proxy_send(
            state,
            UserEvent::AnimationFramePending {
                id: self.id,
                pending,
            },
        )
    }

    #[getter]
    pub fn contentProtected(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| entry.state.content_protected)
    }

    #[fast]
    pub fn setContentProtected(&self, state: &OpState, protected: bool) -> bool {
        let id = self.id;
        self.sync_and_send(state, move |entry| {
            entry.state.content_protected = protected;
            UserEvent::SetContentProtected { id, protected }
        })
    }

    #[getter]
    pub fn closable(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| {
            entry.state.enabled_buttons.contains(WindowButtons::CLOSE)
        })
    }

    #[fast]
    #[setter]
    pub fn closable(&self, state: &OpState, closable: bool) -> bool {
        self.toggle_button(state, WindowButtons::CLOSE, closable)
    }

    #[getter]
    pub fn minimizable(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| {
            entry
                .state
                .enabled_buttons
                .contains(WindowButtons::MINIMIZE)
        })
    }

    #[fast]
    #[setter]
    pub fn minimizable(&self, state: &OpState, minimizable: bool) -> bool {
        self.toggle_button(state, WindowButtons::MINIMIZE, minimizable)
    }

    #[getter]
    pub fn maximizable(&self, state: &OpState) -> Option<bool> {
        self.with_entry(state, |entry| {
            entry
                .state
                .enabled_buttons
                .contains(WindowButtons::MAXIMIZE)
        })
    }

    #[fast]
    #[setter]
    pub fn maximizable(&self, state: &OpState, maximizable: bool) -> bool {
        self.toggle_button(state, WindowButtons::MAXIMIZE, maximizable)
    }

    /// Set or remove a window var. `value = None` removes it; bound attrs
    /// fall back to their defaults. Triggers a redraw if anything changed.
    pub fn setVar(
        &self,
        state: &OpState,
        #[string] key: &str,
        #[string] value: Option<String>,
    ) -> bool {
        let js_state = state.borrow::<SharedJsState>().clone();
        with_state(&js_state, |s| {
            let Some(entry) = s.windows.get_mut(&self.id) else {
                return false;
            };
            entry.set_var(key, value);
            if let Some(window) = entry.window.as_ref() {
                window.request_redraw();
            }
            true
        })
    }

    #[getter]
    pub fn remBase(&self, state: &OpState) -> f32 {
        self.with_entry(state, |entry| entry.rem_base)
            .unwrap_or(16.0)
    }

    #[fast]
    #[setter]
    pub fn remBase(&self, state: &mut OpState, value: f64) {
        self.with_entry_mut(state, |entry| entry.rem_base = value as f32);
    }
}

#[cfg(test)]
mod tests {
    use super::{WindowOptions, WindowPosition, WindowTheme};
    use winit::window::{Fullscreen, Theme};

    fn base_options() -> WindowOptions {
        WindowOptions {
            width: 800,
            height: 600,
            title: "demo".to_string(),
            ..WindowOptions::default()
        }
    }

    #[test]
    fn create_options_map_common_attributes() {
        let mut options = base_options();
        options.visible = false;
        options.resizable = false;
        options.decorations = false;
        options.transparent = true;
        options.maximized = true;
        options.fullscreen = true;
        options.theme = Some(WindowTheme::Dark);
        options.active = Some(true);
        options.content_protected = true;

        let attributes = options.to_window_attributes();

        assert!(!attributes.visible);
        assert!(!attributes.resizable);
        assert!(!attributes.decorations);
        assert!(attributes.transparent);
        assert!(attributes.maximized);
        assert!(attributes.active);
        assert!(attributes.content_protected);
        assert_eq!(attributes.preferred_theme, Some(Theme::Dark));
        assert!(matches!(
            attributes.fullscreen,
            Some(Fullscreen::Borderless(None))
        ));
    }

    #[test]
    fn default_min_size_is_preserved_without_explicit_min_size() {
        let options = base_options();

        let attributes = options.to_window_attributes();

        assert!(attributes.min_inner_size.is_some());
    }

    #[test]
    fn explicit_window_level_maps_to_window_attributes() {
        let mut options = base_options();
        options.window_level = Some(super::UzWindowLevel::AlwaysOnBottom);

        assert_eq!(
            options.window_level(),
            winit::window::WindowLevel::AlwaysOnBottom
        );
    }

    #[test]
    fn flat_button_options_map_to_enabled_buttons() {
        let mut options = base_options();
        options.closable = false;
        options.minimizable = true;
        options.maximizable = false;

        let buttons = options.enabled_buttons();

        assert!(!buttons.contains(winit::window::WindowButtons::CLOSE));
        assert!(buttons.contains(winit::window::WindowButtons::MINIMIZE));
        assert!(!buttons.contains(winit::window::WindowButtons::MAXIMIZE));
    }

    #[test]
    fn window_position_round_trip() {
        let p = WindowPosition { x: 1.0, y: 2.0 };
        assert_eq!(p.x, 1.0);
        assert_eq!(p.y, 2.0);
    }
}
