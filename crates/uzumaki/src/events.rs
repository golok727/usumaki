//! Window input handling, decoupled from any particular runtime.
//!
//! The handlers in these submodules translate raw winit input into
//! [`AppEvent`]s and apply default actions against [`UIState`](crate::ui::UIState).
//! They never reach into the JS runtime: events are returned to the caller,
//! which decides how to dispatch them. This keeps the layer reusable for a
//! future pure-Rust framework built on the same renderer.

mod app_event;
mod clipboard;
mod keyboard;
mod mouse;
mod text_input;
mod types;
mod wheel;

pub use app_event::AppEvent;
pub use clipboard::{
    ClipboardCommand, ClipboardTarget, apply_clipboard_command, build_clipboard_command,
    clipboard_command_to_event,
};
pub use keyboard::{
    TabFocusOutcome, build_key_event, handle_key_for_button, handle_key_for_checkbox,
    handle_key_for_input, handle_key_for_view_selection, handle_tab_focus,
};
pub use mouse::{handle_cursor_moved, handle_mouse_input};
pub use text_input::{
    FocusedInputLayoutMeta, input_layout_meta, scroll_input_to_cursor, update_ime_cursor_area,
};
pub use types::{
    KeyModifiers, MouseButtons, UzClipboardEvent, UzFocusEvent, UzInputEvent, UzKeyboardEvent,
    UzMouseEvent, UzResizeEvent, UzThemeEvent, UzWindowEvent,
};
pub use wheel::handle_mouse_wheel;
