use serde::Serialize;

use super::types::{
    UzClipboardEvent, UzFocusEvent, UzInputEvent, UzKeyboardEvent, UzMouseEvent, UzResizeEvent,
    UzSelectionChangeEvent, UzThemeEvent, UzWindowEvent,
};
use crate::selection::TextSelection;

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
    #[serde(rename = "selectionChange")]
    SelectionChange(UzSelectionChangeEvent),
    #[serde(rename = "windowLoad")]
    WindowLoad(UzWindowEvent),
    #[serde(rename = "windowClose")]
    WindowClose(UzWindowEvent),
    #[serde(rename = "themeChanged")]
    ThemeChanged(UzThemeEvent),
    HotReload,
}

fn endpoints_match(a: &TextSelection, b: &TextSelection) -> bool {
    fn key(
        e: Option<crate::selection::SelectionEndpoint>,
    ) -> Option<(crate::node::UzNodeId, usize)> {
        e.map(|p| (p.node, p.offset))
    }
    key(a.anchor) == key(b.anchor) && key(a.focus) == key(b.focus)
}

pub fn selection_change_event(
    window_id: u32,
    before: &TextSelection,
    after: &TextSelection,
) -> Option<AppEvent> {
    if endpoints_match(before, after) {
        return None;
    }
    Some(AppEvent::SelectionChange(UzSelectionChangeEvent {
        window_id,
        anchor_node_id: after.anchor.map(|e| e.node),
        anchor_offset: after.anchor.map(|e| e.offset as u32).unwrap_or(0),
        focus_node_id: after.focus.map(|e| e.node),
        focus_offset: after.focus.map(|e| e.offset as u32).unwrap_or(0),
        is_collapsed: after.is_collapsed(),
    }))
}
