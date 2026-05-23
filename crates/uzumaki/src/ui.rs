use slab::Slab;

use crate::{
    cursor::UzCursorIcon,
    element::{ElementNode, ImageData, ImageNode, TextContent, TextRunEntry, TextSelectRun},
    input::InputState,
    interactivity::{HitTestState, HitboxStore},
    layout::TaffyLayoutExt,
    node::{Node, ScrollAxis, TextNode, UzNodeId},
    paint::{
        ScrollThumbRect,
        scroll::{self, ScrollAlign, ScrollIntoViewOptions},
    },
    selection::TextSelection,
    style::{Length, UzStyle},
    text::TextRenderer,
};

/// Active scroll-thumb drag. Stored on the dom (only one drag at a time).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollDragState {
    pub node_id: UzNodeId,
    pub axis: ScrollAxis,
    /// Mouse coordinate on `axis` at drag start (logical px).
    pub start_mouse_pos: f64,
    pub start_scroll_offset: f32,
    /// Distance the thumb can travel along `axis` (track length minus thumb
    /// length).
    pub track_range: f64,
    /// Maximum scroll offset along `axis` (content_size - visible_size).
    pub max_scroll: f32,
}

/// Short-lived wheel routing capture so a nested scroller keeps ownership
/// across a burst of wheel events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollWheelTarget {
    pub node_id: UzNodeId,
    pub axis: ScrollAxis,
    pub started_at: std::time::Instant,
}

/// Current drag owned by the UI. Persistent scroll position stays on nodes;
/// transient gesture ownership lives at the UI/document level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DragMode {
    None,
    ScrollbarThumb(ScrollDragState),
    InputSelection(UzNodeId),
    ViewSelection(UzNodeId),
    /// Reserved for future element drag / drag event work.
    Node(UzNodeId),
}

impl DragMode {
    pub fn node_id(&self) -> Option<UzNodeId> {
        match self {
            Self::None => None,
            Self::ScrollbarThumb(state) => Some(state.node_id),
            Self::InputSelection(node_id) | Self::ViewSelection(node_id) | Self::Node(node_id) => {
                Some(*node_id)
            }
        }
    }

    pub fn as_scrollbar_thumb(&self) -> Option<&ScrollDragState> {
        match self {
            Self::ScrollbarThumb(state) => Some(state),
            _ => None,
        }
    }
}

pub struct UIState {
    pub nodes: Slab<Node>,

    pub root: Option<UzNodeId>,
    pub hitbox_store: HitboxStore,

    pub hit_state: HitTestState,

    pub focused_node: Option<UzNodeId>,

    pub last_click_time: Option<std::time::Instant>,

    pub last_click_node: Option<UzNodeId>,

    pub click_count: u8,

    pub window_focused: bool,

    pub scroll_thumbs: Vec<ScrollThumbRect>,

    pub drag_mode: DragMode,

    pub wheel_capture: Option<ScrollWheelTarget>,
    /// Current text selection within a textSelect view. `root == None` means
    /// there is no active view selection
    pub text_selection: TextSelection,
    // TODO move this to renderer ?
    /// Text runs for textSelect subtrees, rebuilt each frame.
    pub selectable_text_runs: Vec<TextSelectRun>,

    pending_scroll_node_into_view: Option<(UzNodeId, ScrollIntoViewOptions)>,

    /// Set when something invalidates the hit tree (scroll, layout,
    /// mutation) and a fresh hit-test is needed before input dispatch.
    /// `hit_tree::rebuild` clears it.
    pub hit_tree_dirty: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self::new()
    }
}

impl UIState {
    pub fn new() -> Self {
        Self {
            nodes: Slab::new(),
            root: None,
            hitbox_store: HitboxStore::default(),
            hit_state: HitTestState::default(),
            focused_node: None,
            pending_scroll_node_into_view: None,
            last_click_time: None,
            last_click_node: None,
            click_count: 0,
            window_focused: true,
            scroll_thumbs: Vec::new(),
            drag_mode: DragMode::None,
            wheel_capture: None,
            text_selection: TextSelection::default(),
            hit_tree_dirty: true,
            selectable_text_runs: Vec::new(),
        }
    }

    pub fn has_focused_node(&self) -> bool {
        self.focused_node.is_some()
    }

    pub(crate) fn with_focused_node<R>(
        &mut self,
        update: impl FnOnce(&mut Node, UzNodeId) -> R,
    ) -> Option<R> {
        let focus = self.focused_node;
        focus.and_then(|id| self.nodes.get_mut(id).map(|node| update(node, id)))
    }

    pub fn get_node(&self, node_id: UzNodeId) -> Option<&Node> {
        self.nodes.get(node_id)
    }

    pub fn get_node_mut(&mut self, node_id: UzNodeId) -> Option<&mut Node> {
        self.nodes.get_mut(node_id)
    }

    /// Resolve the effective cursor for `node_id`.
    /// Precedence: inherited/authored style -> behavior default -> selectable
    /// text fallback.
    pub fn resolve_cursor(&self, node_id: UzNodeId) -> UzCursorIcon {
        let Some(node) = self.nodes.get(node_id) else {
            return UzCursorIcon::Default;
        };

        let style = node.computed_style();

        if let Some(c) = style.cursor {
            return c;
        }

        if let Some(c) = node.default_cursor() {
            return c;
        }

        if node.is_text_selectable() {
            return UzCursorIcon::Text;
        }

        UzCursorIcon::Default
    }

    /// Create a View element with a style.
    pub fn create_view(&mut self, style: UzStyle) -> UzNodeId {
        self.nodes.insert(Node::new(style, ElementNode::new_view()))
    }

    pub fn create_button(&mut self, style: UzStyle) -> UzNodeId {
        self.nodes
            .insert(Node::new(style, ElementNode::new_button()))
    }

    // leaf text node
    pub fn create_text_node(&mut self, content: String, style: UzStyle) -> UzNodeId {
        let text = TextContent {
            content: content.clone(),
        };
        self.nodes
            .insert(Node::new(style, TextNode::new(text.clone())))
    }

    /// Create a Text element. <text>
    pub fn create_text_element(&mut self, content: String, style: UzStyle) -> UzNodeId {
        let text = TextContent {
            content: content.clone(),
        };
        self.nodes
            .insert(Node::new(style, ElementNode::new_text(text.clone())))
    }

    /// Create an Input element.
    pub fn create_input(&mut self, style: UzStyle) -> UzNodeId {
        let is = InputState::new_single_line();

        let node_id = self
            .nodes
            .insert(Node::new(style, ElementNode::new_text_input(is)));
        self.nodes[node_id]
            .as_element_mut()
            .expect("input should be an element")
            .set_focussable(true);
        node_id
    }

    pub fn create_image(&mut self, style: UzStyle) -> UzNodeId {
        self.nodes.insert(Node::new(
            style,
            ElementNode::new_image(ImageNode::default()),
        ))
    }

    pub fn create_checkbox(&mut self, mut style: UzStyle) -> UzNodeId {
        if matches!(style.size.width, Length::Auto) {
            style.size.width = Length::Px(18.0);
        }
        if matches!(style.size.height, Length::Auto) {
            style.size.height = Length::Px(18.0);
        }

        let node_id = self
            .nodes
            .insert(Node::new(style, ElementNode::new_checkbox_input(false)));
        self.nodes[node_id]
            .as_element_mut()
            .expect("checkbox should be an element")
            .set_focussable(true);
        node_id
    }

    pub fn set_root(&mut self, node_id: UzNodeId) {
        self.root = Some(node_id);
    }

    pub fn append_child(&mut self, parent_id: UzNodeId, child_id: UzNodeId) {
        if parent_id == child_id {
            return;
        }
        if self.nodes[parent_id].get_text_content().is_some() {
            return;
        }

        self.detach_from_parent(child_id);

        self.nodes[child_id].parent = Some(parent_id);
        // Seed layout_parent to match — the construct phase reassigns it
        // every frame, but seeding here keeps direct DOM access (e.g.,
        // hit-test against a manually built tree) consistent before the
        // first layout pass runs.
        self.nodes[child_id].layout_parent = Some(parent_id);
        self.nodes[parent_id].children.push(child_id);
    }

    pub fn insert_before(&mut self, parent_id: UzNodeId, child_id: UzNodeId, before_id: UzNodeId) {
        if child_id == before_id || parent_id == child_id {
            return;
        }
        if self.nodes[parent_id].get_text_content().is_some() {
            return;
        }
        self.detach_from_parent(child_id);
        self.nodes[child_id].parent = Some(parent_id);
        self.nodes[child_id].layout_parent = Some(parent_id);
        let Some(index) = self.nodes[parent_id]
            .children
            .iter()
            .position(|&id| id == before_id)
        else {
            self.nodes[child_id].parent = None;
            self.nodes[child_id].layout_parent = None;
            return;
        };
        self.nodes[parent_id].children.insert(index, child_id);
    }

    /// Walks the parent chain from `node_id` and returns true if it reaches
    /// the document root. Used by the pending-destroy drain to decide whether
    /// a wrapper-less native node still belongs to the tree.
    pub fn is_connected(&self, node_id: UzNodeId) -> bool {
        let root = self.root;
        let mut cur = Some(node_id);
        while let Some(id) = cur {
            if Some(id) == root {
                return true;
            }
            cur = self.nodes.get(id).and_then(|n| n.parent);
        }
        false
    }

    pub fn first_child(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        self.nodes
            .get(node_id)
            .and_then(|node| node.children.first().copied())
    }

    pub fn last_child(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        self.nodes
            .get(node_id)
            .and_then(|node| node.children.last().copied())
    }

    pub fn next_sibling(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        let node = self.nodes.get(node_id)?;
        let parent_id = node.parent?;
        let siblings = &self.nodes.get(parent_id)?.children;
        let index = siblings.iter().position(|&id| id == node_id)?;
        siblings.get(index + 1).copied()
    }

    pub fn prev_sibling(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        let node = self.nodes.get(node_id)?;
        let parent_id = node.parent?;
        let siblings = &self.nodes.get(parent_id)?.children;
        let index = siblings.iter().position(|&id| id == node_id)?;
        index
            .checked_sub(1)
            .and_then(|idx| siblings.get(idx).copied())
    }

    pub fn request_scroll_node_into_view(
        &mut self,
        node_id: UzNodeId,
        opts: ScrollIntoViewOptions,
    ) {
        self.pending_scroll_node_into_view = Some((node_id, opts));
    }

    /// Focus-flavored variant: text inputs scroll their parent (the wrapping
    /// view) instead of themselves, and we always use default options.
    pub fn request_scroll_focus_into_view(&mut self, node_id: UzNodeId) {
        let target = if self.nodes.get(node_id).is_some_and(|n| n.is_text_input()) {
            // Use the layout parent so an input wrapped by an anonymous
            // inline run still scrolls its real visual container.
            self.nodes
                .get(node_id)
                .and_then(|n| n.layout_parent)
                .unwrap_or(node_id)
        } else {
            node_id
        };
        self.request_scroll_node_into_view(target, ScrollIntoViewOptions::default());
    }

    pub fn scroll_node_into_view(&mut self, node_id: UzNodeId, opts: ScrollIntoViewOptions) {
        if !self.nodes.contains(node_id) {
            return;
        }
        self.scroll_axis_into_view(node_id, ScrollAxis::Y, opts.block, opts.margin);
        self.scroll_axis_into_view(node_id, ScrollAxis::X, opts.inline, opts.margin);
    }

    fn scroll_axis_into_view(
        &mut self,
        target_id: UzNodeId,
        axis: ScrollAxis,
        align: ScrollAlign,
        margin: f32,
    ) {
        let Some(scroller_id) = Self::nearest_overflow_scroller(&self.nodes, target_id, axis)
        else {
            return;
        };
        let Some((rel, scroller_abs)) =
            Self::accumulate_axis(&self.nodes, target_id, scroller_id, axis)
        else {
            return;
        };

        let Some(target_node) = self.nodes.get(target_id) else {
            return;
        };
        let Some(scroller_ref) = self.nodes.get(scroller_id) else {
            return;
        };

        let target_extent = target_node.final_layout.axis_size(axis);
        let content_extent = scroller_ref.final_layout.axis_scroll_content_size(axis);

        // Clamp viewport by the root rect so a scroller overflowing the window
        // doesn't think it has more visible space than the user can actually see.
        let root_extent = self
            .root
            .and_then(|r| self.nodes.get(r))
            .map(|n| n.final_layout.axis_size(axis))
            .unwrap_or(f32::MAX);
        let scroller_extent = scroller_ref.final_layout.axis_content_box_size(axis);
        let clipped_end = (scroller_abs + scroller_extent).min(root_extent);
        let true_viewport = (clipped_end - scroller_abs).max(0.0);
        let viewport_extent = true_viewport;

        let cur_offset = scroller_ref.scroll_state.offset(axis);

        let Some(next_offset) = scroll::compute_scroll_offset(
            rel,
            target_extent,
            viewport_extent,
            content_extent,
            cur_offset,
            align,
            margin,
        ) else {
            return;
        };

        self.nodes[scroller_id]
            .scroll_state
            .set_offset(axis, next_offset);
        self.hit_tree_dirty = true;
    }

    fn nearest_overflow_scroller(
        nodes: &Slab<Node>,
        target_id: UzNodeId,
        axis: ScrollAxis,
    ) -> Option<UzNodeId> {
        // Walk the layout tree — `overflow` is a layout/visual property
        // and the scroll container that paints around `target_id` may be
        // separated by anonymous wrappers in the layout tree.
        let mut ancestor = nodes.get(target_id).and_then(|n| n.layout_parent)?;
        loop {
            let node = nodes.get(ancestor)?;
            let scrollable = match axis {
                ScrollAxis::Y => node.computed_style().overflow_y.is_scrollable(),
                ScrollAxis::X => node.computed_style().overflow_x.is_scrollable(),
            };
            if scrollable {
                return Some(ancestor);
            }
            ancestor = node.layout_parent?;
        }
    }

    /// Walks target -> root once, returning (rel of target inside scroller,
    /// layout-absolute position of scroller) along the given axis.
    fn accumulate_axis(
        nodes: &Slab<Node>,
        target_id: UzNodeId,
        scroller_id: UzNodeId,
        axis: ScrollAxis,
    ) -> Option<(f32, f32)> {
        let mut cur = target_id;
        let mut rel = 0.0f32;
        let mut scroller_abs = 0.0f32;
        let mut past_scroller = false;
        loop {
            let node = nodes.get(cur)?;
            let loc = node.final_layout.axis_location(axis);
            if past_scroller {
                scroller_abs += loc;
            } else {
                rel += loc;
            }
            // `final_layout.location` is positioned relative to the
            // layout parent, so accumulate via `layout_parent`.
            match node.layout_parent {
                Some(pid) if !past_scroller && pid == scroller_id => {
                    past_scroller = true;
                    cur = pid;
                }
                Some(pid) => cur = pid,
                None => {
                    if past_scroller {
                        break;
                    } else {
                        return None;
                    }
                }
            }
        }
        Some((rel, scroller_abs))
    }

    fn remove_child_ref(&mut self, parent_id: UzNodeId, child_id: UzNodeId) -> bool {
        let Some(index) = self.nodes[parent_id]
            .children
            .iter()
            .position(|&id| id == child_id)
        else {
            return false;
        };
        self.nodes[parent_id].children.remove(index);
        true
    }

    fn detach_from_parent(&mut self, child_id: UzNodeId) {
        let Some(parent_id) = self.nodes[child_id].parent else {
            return;
        };

        self.remove_child_ref(parent_id, child_id);
        self.nodes[child_id].parent = None;
        self.nodes[child_id].layout_parent = None;
    }

    /// Single source of truth for clearing stale NodeId references when a node
    /// is about to be freed. With plain `usize` NodeIds (slab), any long-lived
    /// field holding a removed id would silently retarget to whatever node
    /// reuses the slot. Every removal path MUST funnel through here.
    ///
    /// When adding a new long-lived `NodeId` field to `Dom`, register it here.
    fn on_node_removed(&mut self, id: UzNodeId) {
        if self.focused_node == Some(id) {
            self.focused_node = None;
        }
        if matches!(self.pending_scroll_node_into_view, Some((nid, _)) if nid == id) {
            self.pending_scroll_node_into_view = None;
        }
        if self.last_click_node == Some(id) {
            self.last_click_node = None;
            self.click_count = 0;
            self.last_click_time = None;
        }
        if self.drag_mode.node_id() == Some(id) {
            self.drag_mode = DragMode::None;
        }
        if self
            .wheel_capture
            .as_ref()
            .is_some_and(|capture| capture.node_id == id)
        {
            self.wheel_capture = None;
        }
        if self
            .text_selection
            .anchor
            .is_some_and(|endpoint| endpoint.node == id)
            || self
                .text_selection
                .focus
                .is_some_and(|endpoint| endpoint.node == id)
            || self.selection_root(&self.text_selection) == Some(id)
        {
            self.text_selection.clear();
        }

        self.hit_state.hovered_nodes.retain(|&n| n != id);
        if self.hit_state.top_node == Some(id) {
            self.hit_state.top_node = None;
        }
        if self.hit_state.active_node == Some(id) {
            self.hit_state.active_node = None;
        }

        self.scroll_thumbs.retain(|t| t.node_id != id);
        self.hitbox_store.retain_by_node(|n| n != id);

        // Selectable text runs reference nodes as both roots and entries.
        self.selectable_text_runs
            .retain(|r| r.root_id != id && !r.entries.iter().any(|e| e.node_id == id));
    }

    pub fn remove_child(&mut self, parent_id: UzNodeId, child_id: UzNodeId) {
        if !self.remove_child_ref(parent_id, child_id) {
            return;
        }
        self.nodes[child_id].parent = None;
        self.nodes[child_id].layout_parent = None;
        // The node lives on in the slab until its JS wrapper is collected, but
        // every long-lived NodeId field (selection, focus, hit-state, scroll
        // locks…) refers to nodes by their place in the tree. Once detached
        // those references are stale, so scrub them now — same contract as
        // before the GC refactor.
        self.on_detached_subtree(child_id);
    }

    pub fn destroy_node(&mut self, node_id: UzNodeId) {
        if !self.nodes.contains(node_id) || self.root == Some(node_id) {
            return;
        }

        // eprintln!("[uzumaki] removing native node {node_id}",);

        if let Some(parent_id) = self.nodes[node_id].parent
            && self.nodes.contains(parent_id)
        {
            self.remove_child_ref(parent_id, node_id);
        }

        let children = std::mem::take(&mut self.nodes[node_id].children);
        for child_id in children {
            if let Some(child) = self.nodes.get_mut(child_id)
                && child.parent == Some(node_id)
            {
                child.parent = None;
                child.layout_parent = None;
            }
        }

        self.on_node_removed(node_id);
        self.nodes.remove(node_id);
    }

    /// Update a text node's content.
    pub fn set_text_content(&mut self, node_id: UzNodeId, text: String) {
        let node = &mut self.nodes[node_id];
        if let Some(text_node) = node.text_content_mut() {
            text_node.content = text;
        }
    }

    pub fn set_image_data(&mut self, node_id: UzNodeId, data: ImageData) {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return;
        };
        let Some(image_node) = node.as_image_mut() else {
            return;
        };
        image_node.data = data;
    }

    pub fn clear_image_data(&mut self, node_id: UzNodeId) {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return;
        };
        let Some(image_node) = node.as_image_mut() else {
            return;
        };
        image_node.clear();
    }

    /// Detach all children from `parent_id`. Nodes themselves stay alive in
    /// the slab;
    pub fn clear_children(&mut self, parent_id: UzNodeId) {
        let children = std::mem::take(&mut self.nodes[parent_id].children);
        for child_id in children {
            self.nodes[child_id].parent = None;
            self.on_detached_subtree(child_id);
        }
    }

    /// Walk the subtree rooted at `id` (still wired via `children` since we
    /// only detach the root from its parent) and run `on_node_removed` on
    /// every entry. The slab nodes themselves stay alive.
    fn on_detached_subtree(&mut self, id: UzNodeId) {
        let children: Vec<UzNodeId> = self
            .nodes
            .get(id)
            .map(|node| node.children.clone())
            .unwrap_or_default();
        for child in children {
            self.on_detached_subtree(child);
        }
        self.on_node_removed(id);
    }

    pub fn compute_layout(
        &mut self,
        width: f32,
        height: f32,
        text_renderer: &mut TextRenderer,
        scale: f64,
    ) {
        self.compute_styles();
        self.resolve_layout_children();
        crate::layout::LayoutTree::run(self, text_renderer, width, height);
        if let Some((nid, opts)) = self.pending_scroll_node_into_view.take() {
            self.scroll_node_into_view(nid, opts);
        }
        // Layout invalidates the hit tree — refresh it now so input
        // dispatched between this frame and the next paint operates on
        // current geometry.
        crate::hit_tree::rebuild(self, text_renderer, scale);
    }

    /// Rebuild the hit tree if any state mutation has flagged it dirty
    /// (scroll, mutation, etc.). Cheap when nothing has changed.
    pub fn ensure_hit_tree_fresh(&mut self, text_renderer: &mut TextRenderer, scale: f64) {
        if self.hit_tree_dirty {
            crate::hit_tree::rebuild(self, text_renderer, scale);
        }
    }

    fn compute_styles(&mut self) {
        let Some(root) = self.root else { return };
        self.compute_styles_at(root, None);
    }

    fn compute_styles_at(&mut self, node_id: UzNodeId, parent_style: Option<UzStyle>) {
        let hover = self.hit_state.is_hovered(node_id);
        let active = self.hit_state.is_active(node_id);
        let focus = self.focused_node == Some(node_id);

        self.nodes[node_id].compute_styles(hover, active, focus, parent_style.as_ref());

        let computed = self.nodes[node_id].computed_style().clone();
        let children = self.nodes[node_id].children.clone();
        for child_id in children {
            if self.nodes.contains(child_id) {
                self.compute_styles_at(child_id, Some(computed.clone()));
            }
        }
    }

    /// Run hit test at the given mouse position and update hit_state.
    pub fn update_hit_test(&mut self, x: f64, y: f64) {
        let active = self.hit_state.active_node;
        let mut hit_state = self.hitbox_store.hit_test(x, y);
        if let Some(top) = hit_state.top_node {
            hit_state.hovered_nodes = self.hit_path(top);
        }
        hit_state.active_node = active;
        self.hit_state = hit_state;
    }

    fn hit_path(&self, top: UzNodeId) -> Vec<UzNodeId> {
        // The hit-test top node lives in the painted layout tree — walk
        // `layout_parent` so anonymous wrappers between the hit and a
        // hovered ancestor stay in the path.
        let mut path = Vec::new();
        let mut current = Some(top);
        while let Some(id) = current {
            if self.nodes.get(id).is_some() {
                path.push(id);
                current = self.nodes[id].layout_parent;
            } else {
                break;
            }
        }
        path.reverse();
        path
    }

    /// Refresh hit-testing using the current pointer position after layout or
    /// paint invalidates the previous frame's hitboxes.
    pub fn refresh_hit_test(&mut self) -> bool {
        let Some((x, y)) = self.hit_state.mouse_position else {
            return false;
        };

        let old_top = self.hit_state.top_node;
        let old_hovered = self.hit_state.hovered_nodes.clone();
        self.update_hit_test(x, y);

        old_top != self.hit_state.top_node || old_hovered != self.hit_state.hovered_nodes
    }

    /// Set the active node (mouse down on an element).
    pub fn set_active(&mut self, node_id: Option<UzNodeId>) {
        self.hit_state.active_node = node_id;
    }

    pub fn nearest_button_ancestor(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        self.nearest_ancestor_matching(node_id, |node| node.is_button())
    }

    fn nearest_ancestor_matching(
        &self,
        node_id: UzNodeId,
        mut matches: impl FnMut(&Node) -> bool,
    ) -> Option<UzNodeId> {
        // Used by `nearest_button_ancestor` and similar — the question is
        // "what interactive ancestor visually contains this hit?", which
        // is a layout-tree question.
        let mut current = Some(node_id);
        while let Some(id) = current {
            let node = self.nodes.get(id)?;
            if matches(node) {
                return Some(id);
            }
            current = node.layout_parent;
        }
        None
    }

    /// Find the text run that contains a given text node.
    pub fn find_run_for_node(&self, node_id: UzNodeId) -> Option<&TextSelectRun> {
        self.selectable_text_runs
            .iter()
            .find(|run| run.entries.iter().any(|e| e.node_id == node_id))
    }

    /// Find the text run entry for a given text node.
    pub fn find_run_entry_for_node(
        &self,
        node_id: UzNodeId,
    ) -> Option<(&TextSelectRun, &TextRunEntry)> {
        for run in &self.selectable_text_runs {
            for entry in &run.entries {
                if entry.node_id == node_id {
                    return Some((run, entry));
                }
            }
        }
        None
    }

    /// Check whether a node is a text node inside an active textSelect scope.
    pub fn is_text_selectable(&self, node_id: UzNodeId) -> bool {
        self.selectable_text_runs
            .iter()
            .any(|run| run.entries.iter().any(|e| e.node_id == node_id))
    }

    /// Walk up from `node_id` and return the root of the enclosing
    /// text-selectable run, if any. This lets a click anywhere inside a
    /// `selectable` container ,  not just on a text node , start selection
    pub fn containing_text_run_root(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        let mut cur = Some(node_id);
        while let Some(id) = cur {
            if self
                .selectable_text_runs
                .iter()
                .any(|run| run.root_id == id)
            {
                return Some(id);
            }
            // Layout walk: a hit on an anonymous wrapper inside a
            // selectable container should still find that container as
            // the run root.
            cur = self.nodes.get(id).and_then(|n| n.layout_parent);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::UIState;
    use crate::{
        cursor::UzCursorIcon,
        style::{Bounds, Length, Size, UzStyle},
        text::TextRenderer,
    };

    #[test]
    fn refresh_hit_test_retargets_stationary_pointer_after_hitboxes_change() {
        let mut dom = UIState::new();
        let first = dom.create_view(Default::default());
        let second = dom.create_view(Default::default());

        dom.hitbox_store.insert(
            first,
            Bounds {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        );
        dom.update_hit_test(10.0, 10.0);
        dom.set_active(Some(first));

        dom.hitbox_store.clear();
        dom.hitbox_store.insert(
            second,
            Bounds {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        );

        assert!(dom.refresh_hit_test());
        assert_eq!(dom.hit_state.top_node, Some(second));
        assert_eq!(dom.hit_state.active_node, Some(first));
    }

    #[test]
    fn plain_text_inherits_cursor_from_parent() {
        let mut dom = UIState::new();
        let parent = dom.create_view(Default::default());
        let child = dom.create_text_element("pointer".into(), Default::default());

        dom.append_child(parent, child);
        dom.nodes[parent].base_style().cursor = Some(UzCursorIcon::Pointer);

        // Drive the cascade so `cursor` actually reaches the child's
        // computed_style. `compute_styles_at` is private and the full
        // pipeline needs a TextRenderer, so do the two-node walk inline.
        dom.nodes[parent].compute_styles(false, false, false, None);
        let parent_style = dom.nodes[parent].computed_style().clone();
        dom.nodes[child].compute_styles(false, false, false, Some(&parent_style));

        assert_eq!(dom.resolve_cursor(child), UzCursorIcon::Pointer);
    }

    #[test]
    fn button_child_press_activates_button_ancestor() {
        let mut dom = UIState::new();
        let button = dom.create_button(Default::default());
        let label = dom.create_text_element("Press".into(), Default::default());

        dom.append_child(button, label);
        dom.hitbox_store
            .insert(button, Bounds::new(0.0, 0.0, 100.0, 40.0));
        dom.hitbox_store
            .insert(label, Bounds::new(20.0, 10.0, 60.0, 20.0));

        dom.update_hit_test(30.0, 20.0);
        assert_eq!(dom.hit_state.top_node, Some(label));
        assert_eq!(dom.nearest_button_ancestor(label), Some(button));

        dom.set_active(dom.nearest_button_ancestor(label).or(Some(label)));
        assert!(dom.hit_state.is_active(button));
    }

    #[test]
    fn non_button_press_keeps_hit_node_active() {
        let mut dom = UIState::new();
        let view = dom.create_view(Default::default());

        dom.hitbox_store
            .insert(view, Bounds::new(0.0, 0.0, 100.0, 40.0));
        dom.update_hit_test(30.0, 20.0);

        let active = dom
            .hit_state
            .top_node
            .and_then(|nid| dom.nearest_button_ancestor(nid).or(Some(nid)));
        dom.set_active(active);

        assert!(dom.hit_state.is_active(view));
    }

    #[test]
    fn keyboard_activation_is_limited_to_element_kind() {
        let mut dom = UIState::new();
        let focusable_view = dom.create_view(Default::default());
        let button = dom.create_button(Default::default());

        dom.nodes[focusable_view]
            .as_element_mut()
            .unwrap()
            .set_focussable(true);

        assert!(dom.nodes[focusable_view].is_focusable());
        assert!(!dom.nodes[focusable_view].is_keyboard_activatable());
        assert!(dom.nodes[button].is_keyboard_activatable());
    }

    #[test]
    fn append_child_to_text_node_is_noop() {
        let mut dom = UIState::new();
        let parent = dom.create_view(Default::default());
        let text = dom.create_text_element("leaf".into(), Default::default());
        let child = dom.create_view(Default::default());

        dom.append_child(parent, child);
        dom.append_child(text, child);

        assert!(dom.nodes[text].children.is_empty());
        assert_eq!(dom.nodes[child].parent, Some(parent));
    }

    #[test]
    fn insert_before_into_text_node_is_noop() {
        let mut dom = UIState::new();
        let parent = dom.create_view(Default::default());
        let text = dom.create_text_element("leaf".into(), Default::default());
        let child = dom.create_view(Default::default());
        let before = dom.create_view(Default::default());

        dom.append_child(parent, child);
        dom.insert_before(text, child, before);

        assert!(dom.nodes[text].children.is_empty());
        assert_eq!(dom.nodes[child].parent, Some(parent));
    }

    #[test]
    fn remove_node_detaches_links_and_clears_long_lived_refs() {
        let mut dom = UIState::new();
        let parent = dom.create_view(Default::default());
        let child = dom.create_view(Default::default());
        let grandchild = dom.create_view(Default::default());

        dom.append_child(parent, child);
        dom.append_child(child, grandchild);
        dom.focused_node = Some(child);
        dom.hit_state.top_node = Some(child);

        dom.destroy_node(child);

        assert!(!dom.nodes.contains(child));
        assert!(dom.nodes[parent].children.is_empty());
        assert_eq!(dom.nodes[grandchild].parent, None);
        assert_eq!(dom.focused_node, None);
        assert_eq!(dom.hit_state.top_node, None);
    }

    #[test]
    fn view_default_stacks_child_views_vertically() {
        let mut dom = UIState::new();
        let mut renderer = TextRenderer::new();

        let mut parent_style = UzStyle::default_for_element("view");
        parent_style.size = Size {
            width: Length::Px(100.0),
            height: Length::Px(100.0),
        };

        let mut child_style = UzStyle::default_for_element("view");
        child_style.size = Size {
            width: Length::Px(20.0),
            height: Length::Px(10.0),
        };

        let parent = dom.create_view(parent_style);
        let first = dom.create_view(child_style.clone());
        let second = dom.create_view(child_style);

        dom.set_root(parent);
        dom.append_child(parent, first);
        dom.append_child(parent, second);
        dom.compute_layout(100.0, 100.0, &mut renderer, 1.0);

        let first_layout = &dom.nodes[first].final_layout;
        let second_layout = &dom.nodes[second].final_layout;

        assert_eq!(first_layout.location.y, 0.0);
        assert_eq!(second_layout.location.y, first_layout.size.height);
        assert_eq!(second_layout.location.x, 0.0);
    }

    #[test]
    fn transformed_hitboxes_test_points_in_node_local_space() {
        use vello::kurbo::Affine;

        let mut dom = UIState::new();
        let node = dom.create_view(Default::default());
        dom.hitbox_store.insert_transformed(
            node,
            Bounds::new(0.0, 0.0, 10.0, 10.0),
            Affine::translate((50.0, 50.0))
                * Affine::rotate(std::f64::consts::FRAC_PI_4)
                * Affine::translate((-5.0, -5.0)),
        );

        dom.update_hit_test(50.0, 50.0);
        assert_eq!(dom.hit_state.top_node, Some(node));

        dom.update_hit_test(50.0, 42.0);
        assert_eq!(dom.hit_state.top_node, None);
    }
}
