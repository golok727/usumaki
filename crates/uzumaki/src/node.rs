use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};

use bitflags::bitflags;
use refineable::Refineable;

use crate::cursor::UzCursorIcon;
use crate::element::{ElementNode, ImageNode, TextContent};
use crate::input::InputState;
use crate::interactivity::{HitboxId, Interactivity, StyleSlot};
use crate::style::{Outline, TextSelectable, UzStyle, UzStyleRefinement};

pub type UzNodeId = usize;

/// Records that an attribute was set with a `$name` reference. Kept per-node so
/// `setVar` can re-apply just the affected attributes without rescanning the
/// whole DOM by string.
#[derive(Clone, Debug)]
pub struct VarBinding {
    pub attr_name: String,
    pub var_name: String,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct NodeFlags: u8 {
        const ANONYMOUS = 1 << 0;
        const INLINE_ROOT = 1 << 1;
    }
}

impl NodeFlags {
    pub fn reset_construction_flags(&mut self) {
        self.remove(Self::INLINE_ROOT);
    }

    pub fn is_anonymous(self) -> bool {
        self.contains(Self::ANONYMOUS)
    }

    pub fn is_inline_root(self) -> bool {
        self.contains(Self::INLINE_ROOT)
    }
}

/// Which axis a scroll operation targets. Used by drag/wheel routing and by
/// the unified scrollbar geometry helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollAxis {
    X,
    Y,
}

#[derive(Default)]
pub struct ScrollState {
    pub scroll_offset_x: f32,
    pub scroll_offset_y: f32,
}

impl ScrollState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn offset(&self, axis: ScrollAxis) -> f32 {
        match axis {
            ScrollAxis::X => self.scroll_offset_x,
            ScrollAxis::Y => self.scroll_offset_y,
        }
    }

    pub fn set_offset(&mut self, axis: ScrollAxis, value: f32) {
        match axis {
            ScrollAxis::X => self.scroll_offset_x = value,
            ScrollAxis::Y => self.scroll_offset_y = value,
        }
    }

    pub fn scroll_input_x(
        &mut self,
        cursor_left: f32,
        cursor_right: f32,
        natural_w: f32,
        visible_width: f32,
    ) {
        if visible_width <= 0.0 {
            return;
        }
        if cursor_left - self.scroll_offset_x < 0.0 {
            self.scroll_offset_x = cursor_left;
        } else if cursor_right - self.scroll_offset_x > visible_width {
            self.scroll_offset_x = cursor_right - visible_width;
        }
        let max_scroll = (natural_w - visible_width).max(0.0);
        self.scroll_offset_x = self.scroll_offset_x.clamp(0.0, max_scroll);
    }

    pub fn scroll_single_line_input_end(&mut self, natural_w: f32, visible_width: f32) {
        if visible_width <= 0.0 {
            return;
        }
        let max_scroll = (natural_w - visible_width).max(0.0);
        self.scroll_offset_x = max_scroll;
    }

    pub fn scroll_input_y(&mut self, cursor_y: f32, line_height: f32, visible_height: f32) {
        if visible_height <= 0.0 {
            return;
        }
        let cursor_bottom = cursor_y + line_height;
        if cursor_y < self.scroll_offset_y {
            self.scroll_offset_y = cursor_y;
        } else if cursor_bottom > self.scroll_offset_y + visible_height {
            self.scroll_offset_y = cursor_bottom - visible_height;
        }
        self.scroll_offset_y = self.scroll_offset_y.max(0.0);
    }
}

pub struct Node {
    pub parent: Option<UzNodeId>,

    pub children: Vec<UzNodeId>,

    default_style: UzStyle,

    pub data: NodeData,

    pub flags: NodeFlags,

    pub interactivity: Interactivity,

    pub hitbox_id: Option<HitboxId>,
    // nit: we can just use a point
    pub scroll_state: ScrollState,

    // cache the computed style for a frame
    computed_style: UzStyle,

    pub final_layout: taffy::Layout,
    pub unrounded_layout: taffy::Layout,
    pub cache: taffy::Cache,
    pub taffy_style: taffy::Style,
    /// Layout-tree parent. Equals `parent` for normal nodes; for the original
    /// inline children that were wrapped, points at the synthetic anonymous
    /// inline wrapper instead. Set by the construct phase each frame.
    pub layout_parent: Option<UzNodeId>,
    /// Layout-tree children. Rebuilt by the construct phase to splice
    /// anonymous inline wrappers around runs of inline-level children.
    pub layout_children: RefCell<Vec<UzNodeId>>,

    /// Attributes on this node authored as `$name` references. Re-resolved
    /// when `JsWindow::set_var` mutates the window var table.
    pub var_bindings: Vec<VarBinding>,
}

impl Node {
    pub fn new(default_style: UzStyle, data: impl Into<NodeData>) -> Self {
        // todo should we keep a base style to derive from ?
        let taffy_style = default_style.to_taffy();
        Self {
            parent: None,
            children: Vec::new(),
            default_style: default_style.clone(),
            computed_style: default_style,
            taffy_style,
            data: data.into(),
            interactivity: Interactivity::default(),
            hitbox_id: None,
            scroll_state: ScrollState::new(),
            final_layout: taffy::Layout::new(),
            unrounded_layout: taffy::Layout::new(),
            cache: taffy::Cache::new(),
            layout_parent: None,
            layout_children: RefCell::new(Vec::new()),
            flags: NodeFlags::empty(),
            var_bindings: Vec::new(),
        }
    }
}

impl Node {
    pub fn base_style(&mut self) -> &mut UzStyleRefinement {
        self.style_slot(StyleSlot::Base)
    }

    pub(crate) fn style_slot(&mut self, variant: StyleSlot) -> &mut UzStyleRefinement {
        self.interactivity.style_for(variant)
    }

    pub fn computed_style(&self) -> &UzStyle {
        &self.computed_style
    }

    pub fn compute_styles(
        &mut self,
        hover: bool,
        active: bool,
        focus: bool,
        parent_style: Option<&UzStyle>,
    ) {
        let mut style = self.default_style.clone();

        if let Some(parent_style) = parent_style {
            style.inherit_from(parent_style, &self.interactivity.base_style);
        }

        style.refine(&self.interactivity.base_style);

        if hover && let Some(refinement) = &self.interactivity.hover_style {
            style.refine(refinement);
        }
        if active && let Some(refinement) = &self.interactivity.active_style {
            style.refine(refinement);
        }
        if focus && let Some(refinement) = &self.interactivity.focus_style {
            style.refine(refinement);
        }
        if focus && style.outline.is_none() {
            style.outline = Some(Outline::FOCUS_RING);
        }

        self.taffy_style = style.to_taffy();
        self.computed_style = style;
    }

    #[inline]
    pub fn text_selectable(&self) -> TextSelectable {
        self.computed_style().text_selectable
    }

    pub fn is_text_selectable(&self) -> bool {
        self.computed_style().text_selectable.selectable()
    }

    /// Estimated total heap footprint of this node. Used to keep V8's
    /// external-memory accounting honest so the GC schedules collections
    /// based on real cost (image pixels, editor buffers) instead of a flat
    /// per-node constant.
    pub fn heap_bytes(&self) -> usize {
        let mut bytes = std::mem::size_of::<Self>();
        bytes += self.children.capacity() * std::mem::size_of::<UzNodeId>();
        bytes += self.layout_children.borrow().capacity() * std::mem::size_of::<UzNodeId>();
        if let Some(img) = self.as_image() {
            bytes += img.heap_bytes();
        }
        if let Some(input) = self.as_text_input() {
            bytes += input.heap_bytes();
        }
        if let Some(text) = self.get_text_content() {
            bytes += text.content.capacity();
        }
        bytes
    }

    pub fn set_text_selectable(&mut self, text_selectable: TextSelectable) {
        self.style_slot(StyleSlot::Base).text_selectable = Some(text_selectable);
    }

    pub fn as_text_input(&self) -> Option<&InputState> {
        self.data.as_text_input()
    }

    pub fn as_text_input_mut(&mut self) -> Option<&mut InputState> {
        self.data.as_text_input_mut()
    }

    pub fn as_checkbox_input(&self) -> Option<&bool> {
        self.data.as_checkbox_input()
    }

    pub fn as_checkbox_input_mut(&mut self) -> Option<&mut bool> {
        self.data.as_checkbox_input_mut()
    }

    pub fn as_element(&self) -> Option<&ElementNode> {
        self.data.as_element()
    }

    pub fn as_element_mut(&mut self) -> Option<&mut ElementNode> {
        self.data.as_element_mut()
    }

    pub fn get_text_content(&self) -> Option<&TextContent> {
        self.data.get_text_content()
    }

    pub fn text_content_mut(&mut self) -> Option<&mut TextContent> {
        self.data.text_content_mut()
    }

    pub fn as_image(&self) -> Option<&ImageNode> {
        self.data.as_image()
    }

    pub fn as_image_mut(&mut self) -> Option<&mut ImageNode> {
        self.data.as_image_mut()
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
        self.data.is_button()
    }

    pub fn is_keyboard_activatable(&self) -> bool {
        self.data.is_keyboard_activatable()
    }

    pub fn is_text_node(&self) -> bool {
        self.data.is_text_node()
    }

    pub fn is_text_element(&self) -> bool {
        matches!(
            &self.data,
            NodeData::Element(el) if matches!(el.kind, crate::element::ElementKind::Text)
        )
    }

    pub fn default_cursor(&self) -> Option<UzCursorIcon> {
        self.data.default_cursor()
    }

    /// Whether this node can receive keyboard focus.
    pub fn is_focusable(&self) -> bool {
        self.as_element()
            .map(|e| e.is_focussable())
            .unwrap_or(false)
    }

    /// Whether this node participates in an inline formatting context as a
    /// child (bare text node, or `<text>` element). Block-level elements
    /// (View, Button, Input, Image, Checkbox) are not inline.
    pub fn is_inline_level(&self) -> bool {
        match &self.data {
            NodeData::Text(_) => true,
            NodeData::Element(el) => matches!(el.kind, crate::element::ElementKind::Text),
            NodeData::AnonymousBlock(_) | NodeData::Root => false,
        }
    }

    pub fn is_anonymous(&self) -> bool {
        self.flags.is_anonymous()
    }
}

pub struct TextNode(TextContent);

impl TextNode {
    pub fn new(content: TextContent) -> Self {
        Self(content)
    }
}

impl Deref for TextNode {
    type Target = TextContent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TextNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<TextNode> for NodeData {
    fn from(value: TextNode) -> Self {
        Self::Text(value)
    }
}

pub enum NodeData {
    Root,
    // normal text nodes (cant add event listners etc just plain text)
    Text(TextNode),
    // element node
    Element(ElementNode),
    AnonymousBlock(ElementNode),
}

impl From<ElementNode> for NodeData {
    fn from(value: ElementNode) -> Self {
        Self::Element(value)
    }
}

impl NodeData {
    pub fn default_cursor(&self) -> Option<UzCursorIcon> {
        match self {
            Self::Element(element) => element.data.default_cursor(),
            // Plain text labels should inherit the cursor from their container.
            // Text cursor is handled separately for inputs and textSelect content.
            Self::Text(_) => None,
            Self::AnonymousBlock(_) | Self::Root => None,
        }
    }

    pub fn create_root() -> Self {
        Self::Root
    }

    pub fn get_text_content(&self) -> Option<&TextContent> {
        match self {
            Self::Text(text) => Some(&text.0),
            Self::Element(element) => element.data.get_text_content(),
            Self::AnonymousBlock(_) | Self::Root => None,
        }
    }

    pub fn text_content_mut(&mut self) -> Option<&mut TextContent> {
        match self {
            Self::Text(text) => Some(text),
            Self::Element(element) => element.data.text_content_mut(),
            Self::AnonymousBlock(_) | Self::Root => None,
        }
    }

    pub fn as_text_input(&self) -> Option<&InputState> {
        match self {
            Self::Element(element) => element.data.as_text_input(),
            Self::AnonymousBlock(_) | Self::Root | Self::Text(_) => None,
        }
    }

    pub fn as_text_input_mut(&mut self) -> Option<&mut InputState> {
        match self {
            Self::Element(element) => element.data.as_text_input_mut(),
            Self::AnonymousBlock(_) | Self::Root | Self::Text(_) => None,
        }
    }

    pub fn as_checkbox_input(&self) -> Option<&bool> {
        match self {
            Self::Element(element) => element.data.as_checkbox_input(),
            Self::AnonymousBlock(_) | Self::Root | Self::Text(_) => None,
        }
    }

    pub fn as_checkbox_input_mut(&mut self) -> Option<&mut bool> {
        match self {
            Self::Element(element) => element.data.as_checkbox_input_mut(),
            Self::AnonymousBlock(_) | Self::Root | Self::Text(_) => None,
        }
    }

    pub fn as_image(&self) -> Option<&ImageNode> {
        match self {
            Self::Element(element) => element.data.as_image(),
            Self::AnonymousBlock(_) | Self::Root | Self::Text(_) => None,
        }
    }

    pub fn as_image_mut(&mut self) -> Option<&mut ImageNode> {
        match self {
            Self::Element(element) => element.data.as_image_mut(),
            Self::AnonymousBlock(_) | Self::Root | Self::Text(_) => None,
        }
    }

    pub fn is_text_node(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    pub fn is_text_input(&self) -> bool {
        match self {
            Self::Element(element) => element.data.is_text_input(),
            Self::AnonymousBlock(_) | Self::Root | Self::Text(_) => false,
        }
    }

    pub fn is_checkbox_input(&self) -> bool {
        match self {
            Self::Element(element) => element.data.is_checkbox_input(),
            Self::AnonymousBlock(_) | Self::Root | Self::Text(_) => false,
        }
    }

    pub fn is_image(&self) -> bool {
        match self {
            Self::Element(element) => element.data.is_image(),
            Self::AnonymousBlock(_) | Self::Root | Self::Text(_) => false,
        }
    }

    pub fn is_button(&self) -> bool {
        match self {
            Self::Element(element) => element.is_button(),
            Self::AnonymousBlock(_) | Self::Root | Self::Text(_) => false,
        }
    }

    pub fn is_keyboard_activatable(&self) -> bool {
        match self {
            Self::Element(element) => element.is_keyboard_activatable(),
            Self::AnonymousBlock(_) | Self::Root | Self::Text(_) => false,
        }
    }

    pub fn is_element(&self) -> bool {
        matches!(self, Self::Element(_))
    }

    pub fn is_root(&self) -> bool {
        matches!(self, Self::Root)
    }

    pub fn as_element(&self) -> Option<&ElementNode> {
        match self {
            Self::AnonymousBlock(element) | Self::Element(element) => Some(element),
            Self::Root | Self::Text(_) => None,
        }
    }

    pub fn as_element_mut(&mut self) -> Option<&mut ElementNode> {
        match self {
            Self::AnonymousBlock(element) | Self::Element(element) => Some(element),
            Self::Root | Self::Text(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ScrollState;

    #[test]
    fn input_scroll_scrolls_right() {
        let mut scroll = ScrollState::new();
        scroll.scroll_input_x(250.0, 251.5, 300.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 51.5);
    }

    #[test]
    fn input_scroll_scrolls_left() {
        let mut scroll = ScrollState {
            scroll_offset_x: 100.0,
            scroll_offset_y: 0.0,
        };
        scroll.scroll_input_x(50.0, 51.5, 300.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 50.0);
    }

    #[test]
    fn input_scroll_no_negative() {
        let mut scroll = ScrollState {
            scroll_offset_x: -10.0,
            scroll_offset_y: 0.0,
        };
        scroll.scroll_input_x(50.0, 51.5, 300.0, 200.0);
        assert!(scroll.scroll_offset_x >= 0.0);
    }

    #[test]
    fn input_scroll_clamps_to_natural_width() {
        let mut scroll = ScrollState {
            scroll_offset_x: 200.0,
            scroll_offset_y: 0.0,
        };
        scroll.scroll_input_x(30.0, 31.5, 30.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 0.0);
    }

    #[test]
    fn input_scroll_keeps_full_caret_visible() {
        let mut scroll = ScrollState::new();
        scroll.scroll_input_x(198.5, 200.0, 200.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 0.0);
    }

    #[test]
    fn input_scroll_does_not_scroll_past_natural_width_for_caret_width() {
        let mut scroll = ScrollState::new();
        scroll.scroll_input_x(200.0, 201.5, 200.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 0.0);
    }

    #[test]
    fn single_line_input_end_scrolls_to_natural_width() {
        let mut scroll = ScrollState::new();
        scroll.scroll_single_line_input_end(300.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 100.0);
    }

    #[test]
    fn single_line_input_end_does_not_scroll_when_text_fits() {
        let mut scroll = ScrollState {
            scroll_offset_x: 20.0,
            scroll_offset_y: 0.0,
        };
        scroll.scroll_single_line_input_end(180.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 0.0);
    }

    #[test]
    fn input_scroll_y_scrolls_down() {
        let mut scroll = ScrollState::new();
        scroll.scroll_input_y(250.0, 20.0, 200.0);
        assert_eq!(scroll.scroll_offset_y, 70.0);
    }

    #[test]
    fn input_scroll_y_scrolls_up() {
        let mut scroll = ScrollState {
            scroll_offset_x: 0.0,
            scroll_offset_y: 100.0,
        };
        scroll.scroll_input_y(50.0, 20.0, 200.0);
        assert_eq!(scroll.scroll_offset_y, 50.0);
    }
}
