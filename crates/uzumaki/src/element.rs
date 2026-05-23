use std::sync::Arc;

use crate::cursor::UzCursorIcon;
use crate::input::InputState;
use crate::node::UzNodeId;
use crate::parse::parse_max_length;
use crate::prop_keys::AttrValue;
use crate::text::TextBrush;
use parley::{ContentWidths, Layout as ParleyLayout};
use serde_json::{Value, json};
use vello::peniko::Blob;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ElementKind {
    #[default]
    View,
    Text,
    Button,
    Input,
    Checkbox,
    Image,
}

impl ElementKind {
    pub fn is_keyboard_activatable(self) -> bool {
        matches!(self, Self::Button | Self::Checkbox)
    }
}

pub struct ElementNode {
    pub kind: ElementKind,
    pub is_focussable: bool,
    pub data: ElementData,
    pub inline_layout: Option<Box<TextLayout>>,
}

impl ElementNode {
    pub fn new(kind: ElementKind, data: ElementData) -> Self {
        Self {
            kind,
            is_focussable: false,
            data,
            inline_layout: None,
        }
    }

    pub fn new_view() -> Self {
        Self::new(ElementKind::View, ElementData::None)
    }

    pub fn new_button() -> Self {
        let mut element = Self::new(ElementKind::Button, ElementData::None);
        element.set_focussable(true);
        element
    }

    /**
     * Inline text element (for styling inline text) Hello <text> Something <text>
     *  Hello                    <text>Something</text>
     *   |                          |---------------|
     *NodeData::TextNode()   NodeData::ElementNode(ElementData::Text())
     */
    pub fn new_text(text: TextContent) -> Self {
        Self::new(ElementKind::Text, ElementData::Text(text))
    }

    pub fn new_text_input(state: InputState) -> Self {
        Self::new(ElementKind::Input, ElementData::TextInput(Box::new(state)))
    }

    pub fn new_checkbox_input(checked: bool) -> Self {
        Self::new(ElementKind::Checkbox, ElementData::CheckboxInput(checked))
    }

    pub fn new_image(state: ImageNode) -> Self {
        Self::new(ElementKind::Image, ElementData::Image(Box::new(state)))
    }

    pub fn is_text_input(&self) -> bool {
        self.data.is_text_input()
    }

    pub fn is_checkbox_input(&self) -> bool {
        self.data.is_checkbox_input()
    }

    pub fn is_image(&self) -> bool {
        self.data.is_image()
    }

    pub fn is_button(&self) -> bool {
        self.kind == ElementKind::Button
    }

    pub fn is_keyboard_activatable(&self) -> bool {
        self.kind.is_keyboard_activatable()
    }

    pub fn is_focussable(&self) -> bool {
        self.is_focussable
    }

    pub fn set_focussable(&mut self, focussable: bool) {
        self.is_focussable = focussable;
    }

    pub(crate) fn set_attr(&mut self, name: &str, value: AttrValue<'_>) -> bool {
        if name == "focusable" {
            let value = value.parse_bool();
            self.set_focussable(value);
        }

        self.data.set_attr(name, value)
    }

    pub fn clear_attr(&mut self, name: &str) -> bool {
        if name == "focusable" {
            self.set_focussable(false);
            return true;
        }
        self.data.clear_attr(name)
    }

    pub fn get_attr(&self, name: &str) -> Option<Value> {
        if name == "focusable" {
            return Some(json!(self.is_focussable()));
        }
        self.data.get_attr(name)
    }
}

#[derive(Clone, Debug)]
pub struct TextContent {
    pub content: String,
}

impl TextContent {
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

#[derive(Clone, Debug)]
pub struct ImageMeasureInfo {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RasterImageData {
    pub width: u32,
    pub height: u32,
    pub data: Blob<u8>,
}

impl RasterImageData {
    pub fn new(width: u32, height: u32, data: Arc<Vec<u8>>) -> Self {
        Self {
            width,
            height,
            data: Blob::new(data),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum ImageData {
    Raster(RasterImageData),
    Svg {
        tree: Arc<usvg::Tree>,
        uses_current_color: bool,
    },
    #[default]
    None,
}

impl ImageData {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Estimated bytes held on the heap by the decoded image. Reported to V8
    /// via `adjust_amount_of_external_allocated_memory` so a 4K image
    /// pressures GC accordingly instead of looking like a 1KB node.
    pub fn heap_bytes(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Raster(r) => (r.width as usize) * (r.height as usize) * 4,
            // Rough estimate for the parsed usvg tree; real cost depends on
            // node count but a few KB is closer than zero.
            Self::Svg { .. } => 4 * 1024,
        }
    }

    pub fn natural_size(&self) -> Option<(f32, f32)> {
        match self {
            Self::Raster(r) => Some((r.width as f32, r.height as f32)),
            Self::Svg { tree, .. } => {
                let s = tree.size();
                Some((s.width(), s.height()))
            }
            Self::None => None,
        }
    }
}

impl From<RasterImageData> for ImageData {
    fn from(value: RasterImageData) -> Self {
        Self::Raster(value)
    }
}

impl From<usvg::Tree> for ImageData {
    fn from(value: usvg::Tree) -> Self {
        Self::Svg {
            tree: Arc::new(value),
            uses_current_color: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ImageNode {
    pub data: ImageData,
}

impl ImageNode {
    pub fn clear(&mut self) {
        self.data = ImageData::None;
    }

    pub fn heap_bytes(&self) -> usize {
        self.data.heap_bytes()
    }
}

/// One text node's contribution to a textSelect run.
pub struct TextRunEntry {
    pub node_id: UzNodeId,
    pub layout_node_id: UzNodeId,
    /// Start grapheme index of this node in the flat run.
    pub flat_start: usize,
    pub flat_byte_start: usize,
    pub byte_len: usize,
    pub grapheme_count: usize,
}

#[derive(Clone, Debug)]
pub struct InlineTextEntry {
    /// Style owner — drives glyph color/font, and per-line chip
    /// bg/border/padding when this entry is a styled inline element.
    pub node_id: UzNodeId,
    /// Source node whose `get_text_content()` provides this entry's text.
    /// Usually equals `node_id`; differs only for nested cases like a bare
    /// text child of a `<text>` element, where the outer element is the
    /// style owner but the inner text node is the content source.
    pub content_source: UzNodeId,
    /// Cumulative byte offset within the parley layout's flat text.
    pub byte_start: usize,
    /// Bytes contributed by this entry.
    pub byte_len: usize,
}

#[derive(Clone, Default)]
pub enum InlineLayoutKind {
    #[default]
    Leaf,
    /// Inline-formatting context: multiple inline children contribute
    /// styled spans. Entries map flat byte ranges back to source nodes
    /// for selection and per-span box painting.
    InlineRoot { entries: Vec<InlineTextEntry> },
}

#[derive(Clone, Default)]
pub struct TextLayout {
    pub layout: ParleyLayout<TextBrush>,
    /// Total bytes in `layout`'s text (equals what would be `text.len()`
    /// if we materialized the concatenation — we don't, since no consumer
    /// reads the actual content past construct time).
    pub text_len: usize,
    pub content_widths: Option<ContentWidths>,
    pub kind: InlineLayoutKind,
}

impl TextLayout {
    pub fn entries(&self) -> &[InlineTextEntry] {
        match &self.kind {
            InlineLayoutKind::InlineRoot { entries } => entries,
            InlineLayoutKind::Leaf => &[],
        }
    }

    pub fn is_inline_root(&self) -> bool {
        matches!(self.kind, InlineLayoutKind::InlineRoot { .. })
    }
}

/// The complete text run for a textSelect subtree.
/// Built each frame; maps between flat grapheme offsets and per-node positions.
pub struct TextSelectRun {
    pub root_id: UzNodeId,
    pub entries: Vec<TextRunEntry>,
    pub total_graphemes: usize,
}

#[derive(Default)]
pub enum ElementData {
    // this is text Element <text>
    Text(TextContent),
    TextInput(Box<InputState>),
    CheckboxInput(bool),
    Image(Box<ImageNode>),
    // for view nodes
    #[default]
    None,
}

impl ElementData {
    pub fn default_cursor(&self) -> Option<UzCursorIcon> {
        match self {
            Self::TextInput(_) => Some(UzCursorIcon::Text),
            Self::CheckboxInput(_) => Some(UzCursorIcon::Pointer),
            _ => None,
        }
    }

    pub fn is_text_input(&self) -> bool {
        matches!(self, Self::TextInput(_))
    }

    pub fn is_checkbox_input(&self) -> bool {
        matches!(self, Self::CheckboxInput(_))
    }

    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }

    pub fn get_text_content(&self) -> Option<&TextContent> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn text_content_mut(&mut self) -> Option<&mut TextContent> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_text_input(&self) -> Option<&InputState> {
        match self {
            Self::TextInput(state) => Some(state),
            _ => None,
        }
    }

    pub fn as_text_input_mut(&mut self) -> Option<&mut InputState> {
        match self {
            Self::TextInput(state) => Some(state),
            _ => None,
        }
    }

    pub fn as_checkbox_input(&self) -> Option<&bool> {
        match self {
            Self::CheckboxInput(checked) => Some(checked),
            _ => None,
        }
    }

    pub fn as_checkbox_input_mut(&mut self) -> Option<&mut bool> {
        match self {
            Self::CheckboxInput(checked) => Some(checked),
            _ => None,
        }
    }

    pub fn as_image(&self) -> Option<&ImageNode> {
        match self {
            Self::Image(image) => Some(image),
            _ => None,
        }
    }

    pub fn as_image_mut(&mut self) -> Option<&mut ImageNode> {
        match self {
            Self::Image(image) => Some(image),
            _ => None,
        }
    }

    pub(crate) fn set_attr(&mut self, name: &str, value: AttrValue<'_>) -> bool {
        match self {
            Self::TextInput(input) => match name {
                "value" => {
                    input.set_value(&value);
                    true
                }
                "placeholder" => {
                    input.placeholder = value.to_string();
                    true
                }
                "maxLength" => {
                    let n = value.parse::<f32>().ok();
                    let Some(n) = n else {
                        return false;
                    };
                    input.max_length = parse_max_length(n);
                    true
                }
                "disabled" => {
                    input.disabled = value.parse_bool();
                    true
                }
                "multiline" => {
                    input.multiline = value.parse_bool();
                    true
                }
                "secure" => {
                    input.secure = value.parse_bool();
                    true
                }
                _ => false,
            },
            Self::CheckboxInput(checked) => match name {
                "checked" => {
                    let value = value.parse_bool();
                    *checked = value;
                    true
                }
                _ => false,
            },
            Self::Text(_) | Self::Image(_) | Self::None => false,
        }
    }

    pub fn clear_attr(&mut self, name: &str) -> bool {
        match self {
            Self::TextInput(input) => match name {
                "value" => {
                    input.set_value("");
                    true
                }
                "placeholder" => {
                    input.placeholder.clear();
                    true
                }
                "maxLength" => {
                    input.max_length = None;
                    true
                }
                "disabled" => {
                    input.disabled = false;
                    true
                }
                "multiline" => {
                    input.multiline = false;
                    true
                }
                "secure" => {
                    input.secure = false;
                    true
                }
                _ => false,
            },
            Self::CheckboxInput(checked) => match name {
                "checked" => {
                    *checked = false;
                    true
                }
                _ => false,
            },
            Self::Text(_) | Self::Image(_) | Self::None => false,
        }
    }

    pub fn get_attr(&self, name: &str) -> Option<Value> {
        match self {
            Self::TextInput(input) => match name {
                "value" => Some(json!(input.text())),
                "placeholder" => Some(json!(input.placeholder)),
                "disabled" => Some(json!(input.disabled)),
                "maxLength" => Some(input.max_length.map_or(Value::Null, |m| json!(m))),
                "multiline" => Some(json!(input.multiline)),
                "secure" => Some(json!(input.secure)),
                _ => None,
            },
            Self::CheckboxInput(checked) => match name {
                "checked" => Some(json!(checked)),
                _ => None,
            },
            Self::Text(_) | Self::Image(_) | Self::None => None,
        }
    }
}
