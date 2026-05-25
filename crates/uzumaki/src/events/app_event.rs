use serde::Serialize;

use super::types::{
    UzClipboardEvent, UzFocusEvent, UzInputEvent, UzKeyboardEvent, UzMouseEvent, UzResizeEvent,
    UzThemeEvent, UzWindowEvent,
};

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AppEvent {
    Click(UzMouseEvent),
    MouseDown(UzMouseEvent),
    MouseUp(UzMouseEvent),
    MouseMove(UzMouseEvent),
    MouseEnter(UzMouseEvent),
    MouseLeave(UzMouseEvent),
    MouseOver(UzMouseEvent),
    MouseOut(UzMouseEvent),
    KeyDown(UzKeyboardEvent),
    KeyUp(UzKeyboardEvent),
    Resize(UzResizeEvent),
    Input(UzInputEvent),
    BeforeInput(UzInputEvent),
    Focus(UzFocusEvent),
    Blur(UzFocusEvent),
    Copy(UzClipboardEvent),
    Cut(UzClipboardEvent),
    Paste(UzClipboardEvent),
    #[serde(rename = "windowLoad")]
    WindowLoad(UzWindowEvent),
    #[serde(rename = "windowClose")]
    WindowClose(UzWindowEvent),
    #[serde(rename = "themeChanged")]
    ThemeChanged(UzThemeEvent),
    HotReload,
}
