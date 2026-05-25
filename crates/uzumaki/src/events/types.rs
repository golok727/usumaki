use bitflags::bitflags;
use serde::Serialize;

use crate::node::UzNodeId;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct KeyModifiers: u32 {
        const CTRL  = 1 << 0;
        const ALT   = 1 << 1;
        const SHIFT = 1 << 2;
        const SUPER = 1 << 3;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct MouseButtons: u8 {
        const LEFT   = 1 << 0;
        const RIGHT  = 1 << 1;
        const MIDDLE = 1 << 2;
    }
}

impl Serialize for KeyModifiers {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u32(self.bits())
    }
}

impl Serialize for MouseButtons {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.bits())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UzMouseEvent {
    pub window_id: u32,
    pub node_id: UzNodeId,
    pub x: f32,
    pub y: f32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub button: u8,
    pub buttons: MouseButtons,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UzKeyboardEvent {
    pub window_id: u32,
    pub node_id: Option<UzNodeId>,
    pub key: String,
    pub code: String,
    pub key_code: u32,
    pub modifiers: KeyModifiers,
    pub repeat: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UzWindowEvent {
    pub window_id: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UzThemeEvent {
    pub window_id: u32,
    pub theme: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UzResizeEvent {
    pub window_id: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UzInputEvent {
    pub window_id: u32,
    pub node_id: UzNodeId,
    pub input_type: String,
    pub data: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UzFocusEvent {
    pub window_id: u32,
    pub node_id: UzNodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UzClipboardEvent {
    pub window_id: u32,
    pub node_id: Option<UzNodeId>,
    pub selection_text: Option<String>,
    pub clipboard_text: Option<String>,
}
