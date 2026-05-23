use parley::Cluster;

use crate::element::{TextRunEntry, TextSelectRun};
use crate::node::{Node, UzNodeId};
use crate::selection::{Affinity, SelectionEndpoint, TextSelection};
use crate::text::{cluster_count, layout_byte_to_grapheme, layout_grapheme_to_byte};
use crate::ui::UIState;

impl UIState {
    /// Build text runs for all textSelect subtrees. Called each frame before render.
    pub fn build_text_select_runs(&mut self) {
        self.selectable_text_runs.clear();
        let Some(root) = self.root else { return };
        self.visit_text_select(root, None);
    }

    fn visit_text_select(&mut self, node_id: UzNodeId, run_idx: Option<usize>) {
        let resolved_text_sel = self.nodes[node_id]
            .computed_style()
            .text_selectable
            .selectable();

        // A node that explicitly enables textSelect when the parent scope
        // doesn't have it starts a new selection scope.
        let current_run = if resolved_text_sel && run_idx.is_none() {
            let idx = self.selectable_text_runs.len();
            self.selectable_text_runs.push(TextSelectRun {
                root_id: node_id,
                entries: Vec::new(),
                total_graphemes: 0,
            });
            Some(idx)
        } else if resolved_text_sel {
            run_idx
        } else {
            None
        };

        let is_inline_root = self.nodes[node_id].flags.is_inline_root();

        if let Some(idx) = current_run
            && let Some(inline) = self.nodes[node_id]
                .as_element()
                .and_then(|element| element.inline_layout.as_ref())
            && inline.is_inline_root()
        {
            let entries = inline.entries().to_vec();
            let run = &mut self.selectable_text_runs[idx];
            for inline_entry in entries {
                // Atomic inline-box entries (chips) carry no text in the
                // parent's parley layout — they're recorded so document
                // ordering is preserved, but the chip's own text is added
                // when we recurse into the chip below.
                if inline_entry.byte_len == 0 {
                    continue;
                }
                let flat_start = run.total_graphemes;
                let flat_byte_start = inline_entry.byte_start;
                let flat_byte_end = flat_byte_start + inline_entry.byte_len;
                let grapheme_start = layout_byte_to_grapheme(&inline.layout, flat_byte_start);
                let grapheme_end = layout_byte_to_grapheme(&inline.layout, flat_byte_end);
                let grapheme_count = grapheme_end.saturating_sub(grapheme_start);
                run.entries.push(TextRunEntry {
                    node_id: inline_entry.node_id,
                    layout_node_id: node_id,
                    flat_start,
                    flat_byte_start,
                    byte_len: inline_entry.byte_len,
                    grapheme_count,
                });
                run.total_graphemes += grapheme_count;
            }
        } else if let Some(idx) = current_run
            && self.nodes[node_id].get_text_content().is_some()
            && self.nodes[node_id]
                .as_element()
                .and_then(|element| element.inline_layout.as_ref())
                .is_some()
        {
            // Leaf text element (chip rendering its own text). Its parley
            // layout lives on the chip itself and starts at byte 0.
            let gc = self.nodes[node_id]
                .as_element()
                .and_then(|element| element.inline_layout.as_ref())
                .map(|inline| &inline.layout)
                .map(cluster_count)
                .unwrap_or(0);
            let run = &mut self.selectable_text_runs[idx];
            run.entries.push(TextRunEntry {
                node_id,
                layout_node_id: node_id,
                flat_start: run.total_graphemes,
                flat_byte_start: 0,
                byte_len: self.nodes[node_id]
                    .get_text_content()
                    .map(|text| text.content.len())
                    .unwrap_or(0),
                grapheme_count: gc,
            });
            run.total_graphemes += gc;
        }

        let layout_children = self.nodes[node_id].layout_children.borrow().clone();
        for cid in layout_children {
            // Bare text children of an inline-root parent are already
            // represented in the parent's entries; skip them so we don't
            // double-count.
            if is_inline_root
                && matches!(
                    self.nodes.get(cid).map(|n| &n.data),
                    Some(crate::node::NodeData::Text(_))
                )
            {
                continue;
            }
            self.visit_text_select(cid, current_run);
        }
    }

    /// Get the currently selected text content. Checks focused input first,
    /// then the active view selection.
    pub fn selected_text(&self) -> String {
        if let Some(fid) = self.focused_node
            && let Some(node) = self.nodes.get(fid)
            && let Some(is) = node.as_text_input()
        {
            return is.selected_text();
        }

        if self.text_selection.is_collapsed() {
            return String::new();
        }
        let Some((start, end)) = self.ordered_text_selection() else {
            return String::new();
        };
        let Some(run) = self.find_run_for_node(start.node) else {
            return String::new();
        };

        let mut out = String::new();
        let mut in_range = false;
        for entry in &run.entries {
            if entry.node_id == start.node {
                in_range = true;
            }
            if !in_range {
                continue;
            }

            let Some(text) = self
                .nodes
                .get(entry.node_id)
                .and_then(|n| n.get_text_content())
            else {
                continue;
            };
            let local_start = if entry.node_id == start.node {
                start.offset.min(text.content.len())
            } else {
                0
            };
            let local_end = if entry.node_id == end.node {
                end.offset.min(text.content.len())
            } else {
                text.content.len()
            };
            if local_start < local_end {
                out.push_str(&text.content[local_start..local_end]);
            }
            if entry.node_id == end.node {
                break;
            }
        }
        out
    }

    /// Get the current selection range as flat grapheme offsets.
    /// Returns (start, end) where start <= end.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let sel = self.get_selection()?;
        if sel.is_collapsed() {
            return None;
        }
        let (start, end) = self.selection_flat_range(&sel)?;
        Some((start, end))
    }

    /// Unified selection. Prefers the focused input; falls back to the view
    /// selection. Returns `None` if neither is set.
    pub fn get_selection(&self) -> Option<TextSelection> {
        if let Some(fid) = self.focused_node
            && let Some(node) = self.nodes.get(fid)
            && let Some(is) = node.as_text_input()
        {
            let sel = is.editor.raw_selection();
            return Some(TextSelection::new(
                SelectionEndpoint::new(fid, sel.anchor().index(), sel.anchor().affinity().into()),
                SelectionEndpoint::new(fid, sel.focus().index(), sel.focus().affinity().into()),
            ));
        }
        self.get_text_selection()
    }

    /// Active view selection, if any. Returns `None` if `root` is unset.
    pub fn get_text_selection(&self) -> Option<TextSelection> {
        self.text_selection.is_set().then_some(self.text_selection)
    }

    /// Set the active view selection. Clears any focused input.
    pub fn set_selection(&mut self, selection: TextSelection) {
        if selection.is_set() {
            self.focused_node = None;
        }
        self.text_selection = selection;
    }

    pub fn selection_root(&self, selection: &TextSelection) -> Option<UzNodeId> {
        let anchor = selection.anchor?;
        let focus = selection.focus?;
        let anchor_root = self.find_run_for_node(anchor.node)?.root_id;
        let focus_root = self.find_run_for_node(focus.node)?.root_id;
        (anchor_root == focus_root).then_some(anchor_root)
    }

    pub fn ordered_text_selection(&self) -> Option<(SelectionEndpoint, SelectionEndpoint)> {
        let (start, end) = self.text_selection.ordered_with(|a, b| {
            if a.node == b.node {
                return a.offset <= b.offset;
            }
            let Some(run) = self.find_run_for_node(a.node) else {
                return true;
            };
            let a_pos = run.entries.iter().position(|entry| entry.node_id == a.node);
            let b_pos = run.entries.iter().position(|entry| entry.node_id == b.node);
            a_pos <= b_pos
        })?;
        if self.selection_root(&self.text_selection).is_some() {
            Some((*start, *end))
        } else {
            None
        }
    }

    pub fn endpoint_from_flat_index(
        &self,
        root_id: UzNodeId,
        flat_index: usize,
        affinity: Affinity,
    ) -> Option<SelectionEndpoint> {
        let run = self
            .selectable_text_runs
            .iter()
            .find(|r| r.root_id == root_id)?;
        for entry in &run.entries {
            let entry_end = entry.flat_start + entry.grapheme_count;
            if flat_index <= entry_end {
                let local_grapheme = flat_index.saturating_sub(entry.flat_start);
                let layout = &self
                    .nodes
                    .get(entry.layout_node_id)?
                    .as_element()?
                    .inline_layout
                    .as_ref()?
                    .layout;
                let entry_grapheme_start = layout_byte_to_grapheme(layout, entry.flat_byte_start);
                let offset = layout_grapheme_to_byte(layout, entry_grapheme_start + local_grapheme)
                    .saturating_sub(entry.flat_byte_start)
                    .min(entry.byte_len);
                return Some(SelectionEndpoint::new(entry.node_id, offset, affinity));
            }
        }
        let entry = run.entries.last()?;
        let text = self.nodes.get(entry.node_id)?.get_text_content()?;
        Some(SelectionEndpoint::new(
            entry.node_id,
            text.content.len(),
            affinity,
        ))
    }

    pub fn flat_index_for_endpoint(&self, endpoint: SelectionEndpoint) -> Option<usize> {
        let (_run, entry) = self.find_run_entry_for_node(endpoint.node)?;
        let layout = &self
            .nodes
            .get(entry.layout_node_id)?
            .as_element()?
            .inline_layout
            .as_ref()?
            .layout;
        let entry_grapheme_start = layout_byte_to_grapheme(layout, entry.flat_byte_start);
        let endpoint_grapheme = layout_byte_to_grapheme(
            layout,
            entry.flat_byte_start + endpoint.offset.min(entry.byte_len),
        );
        Some(entry.flat_start + endpoint_grapheme.saturating_sub(entry_grapheme_start))
    }

    /// Walk forward from `flat_idx` to the next word boundary inside the
    /// textSelect run rooted at `root_id`. Word boundaries are decided by
    /// parley (UAX#29-aware) on each entry's cached layout. Returns
    /// `total_graphemes` when there's no further boundary.
    pub fn next_word_boundary_in_run(&self, root_id: UzNodeId, flat_idx: usize) -> usize {
        let Some(run) = self
            .selectable_text_runs
            .iter()
            .find(|r| r.root_id == root_id)
        else {
            return flat_idx;
        };
        let total = run.total_graphemes;
        if flat_idx >= total {
            return total;
        }
        let Some((mut entry_i, mut local)) = locate_in_run(run, flat_idx) else {
            return total;
        };
        let mut crossed_entry = false;
        loop {
            let entry = &run.entries[entry_i];
            if let Some(layout) = self
                .nodes
                .get(entry.layout_node_id)
                .and_then(|n| n.as_element())
                .and_then(|element| element.inline_layout.as_ref())
                .map(|inline| &inline.layout)
            {
                if crossed_entry
                    && let Some(first) = Cluster::from_byte_index(layout, 0)
                    && first.is_word_boundary()
                {
                    return entry.flat_start;
                }
                if local < entry.grapheme_count {
                    let entry_grapheme_start =
                        layout_byte_to_grapheme(layout, entry.flat_byte_start);
                    let byte = layout_grapheme_to_byte(layout, entry_grapheme_start + local);
                    if let Some(cur) = Cluster::from_byte_index(layout, byte)
                        && let Some(next) = cur.next_logical_word()
                    {
                        let g = layout_byte_to_grapheme(layout, next.text_range().start)
                            .saturating_sub(entry_grapheme_start);
                        return entry.flat_start + g.min(entry.grapheme_count);
                    }
                }
            }
            entry_i += 1;
            if entry_i >= run.entries.len() {
                return total;
            }
            local = 0;
            crossed_entry = true;
        }
    }

    /// Walk backward from `flat_idx` to the previous word boundary in the run.
    /// Returns `0` when there's no previous boundary.
    pub fn prev_word_boundary_in_run(&self, root_id: UzNodeId, flat_idx: usize) -> usize {
        let Some(run) = self
            .selectable_text_runs
            .iter()
            .find(|r| r.root_id == root_id)
        else {
            return flat_idx;
        };
        if flat_idx == 0 {
            return 0;
        }
        let Some((mut entry_i, mut local)) = locate_in_run(run, flat_idx) else {
            return 0;
        };
        loop {
            let entry = &run.entries[entry_i];
            if let Some(layout) = self
                .nodes
                .get(entry.layout_node_id)
                .and_then(|n| n.as_element())
                .and_then(|element| element.inline_layout.as_ref())
                .map(|inline| &inline.layout)
            {
                let entry_grapheme_start = layout_byte_to_grapheme(layout, entry.flat_byte_start);
                let cur = if local >= entry.grapheme_count && entry.grapheme_count > 0 {
                    let last_byte = layout_grapheme_to_byte(
                        layout,
                        entry_grapheme_start + entry.grapheme_count - 1,
                    );
                    let last = Cluster::from_byte_index(layout, last_byte);
                    if let Some(c) = &last
                        && c.is_word_boundary()
                    {
                        return entry.flat_start + entry.grapheme_count - 1;
                    }
                    last
                } else if local > 0 {
                    let byte = layout_grapheme_to_byte(layout, entry_grapheme_start + local);
                    Cluster::from_byte_index(layout, byte)
                } else {
                    None
                };
                if let Some(c) = cur
                    && let Some(prev) = c.previous_logical_word()
                {
                    let g = layout_byte_to_grapheme(layout, prev.text_range().start)
                        .saturating_sub(entry_grapheme_start);
                    return entry.flat_start + g.min(entry.grapheme_count);
                }
            }
            if entry_i == 0 {
                return 0;
            }
            entry_i -= 1;
            local = run.entries[entry_i].grapheme_count;
        }
    }

    pub fn selection_flat_range(&self, selection: &TextSelection) -> Option<(usize, usize)> {
        let (start, end) = selection.ordered_with(|a, b| {
            if a.node == b.node {
                return a.offset <= b.offset;
            }
            let Some(run) = self.find_run_for_node(a.node) else {
                return true;
            };
            let a_pos = run.entries.iter().position(|entry| entry.node_id == a.node);
            let b_pos = run.entries.iter().position(|entry| entry.node_id == b.node);
            a_pos <= b_pos
        })?;
        Some((
            self.flat_index_for_endpoint(*start)?,
            self.flat_index_for_endpoint(*end)?,
        ))
    }

    /// Focus an element node. Clears any active view selection and blurs the
    /// previously focused input.
    pub fn focus_element(&mut self, node_id: UzNodeId) {
        self.text_selection.clear();
        self.focused_node = Some(node_id);
        if let Some(node) = self.nodes.get_mut(node_id)
            && let Some(is) = node.as_text_input_mut()
        {
            is.reset_blink();
        }
    }

    /// Clear the view selection (does not touch focused input).
    pub fn clear_selection(&mut self) {
        self.text_selection.clear();
    }

    /// Walk the DOM in document order from `start_id`, returning the next node
    /// for which `filter` returns true. Wraps to the root once and stops if the
    /// traversal returns to `start_id` without a match.
    pub fn next_node(
        &self,
        start_id: UzNodeId,
        mut filter: impl FnMut(&Node) -> bool,
    ) -> Option<UzNodeId> {
        let mut node_id = start_id;
        let mut look_in_children = true;
        loop {
            let cur = self.nodes.get(node_id)?;
            let next_id = if look_in_children && let Some(first) = cur.children.first().copied() {
                first
            } else if let Some(parent_id) = cur.parent {
                if let Some(sibling) = self.next_sibling(node_id) {
                    look_in_children = true;
                    sibling
                } else {
                    look_in_children = false;
                    node_id = parent_id;
                    continue;
                }
            } else {
                look_in_children = true;
                self.root?
            };

            let next = self.nodes.get(next_id)?;
            if filter(next) {
                return Some(next_id);
            }
            if next_id == start_id {
                return None;
            }
            node_id = next_id;
        }
    }

    /// Walk the DOM in reverse document order. At each step go to the previous
    /// sibling's deepest-last descendant, or up to the parent. Wraps to the
    /// deepest-last descendant of root.
    pub fn prev_node(
        &self,
        start_id: UzNodeId,
        mut filter: impl FnMut(&Node) -> bool,
    ) -> Option<UzNodeId> {
        let mut node_id = start_id;
        loop {
            let cur = self.nodes.get(node_id)?;
            let next_id = if let Some(prev) = self.prev_sibling(node_id) {
                self.deepest_last(prev)
            } else if let Some(parent) = cur.parent {
                parent
            } else {
                self.deepest_last(self.root?)
            };

            let next = self.nodes.get(next_id)?;
            if filter(next) {
                return Some(next_id);
            }
            if next_id == start_id {
                return None;
            }
            node_id = next_id;
        }
    }

    fn deepest_last(&self, mut id: UzNodeId) -> UzNodeId {
        while let Some(last) = self.nodes.get(id).and_then(|n| n.children.last().copied()) {
            id = last;
        }
        id
    }

    /// Move focus to the next focusable element in document order.
    pub fn focus_next_node(&mut self) -> Option<FocusChange> {
        self.focus_step(false)
    }

    /// Move focus to the previous focusable element in document order.
    pub fn focus_prev_node(&mut self) -> Option<FocusChange> {
        self.focus_step(true)
    }

    fn focus_step(&mut self, backward: bool) -> Option<FocusChange> {
        let start_id = self.focused_node.or(self.root)?;
        let new_id = if backward {
            self.prev_node(start_id, |n| n.is_focusable())?
        } else {
            self.next_node(start_id, |n| n.is_focusable())?
        };

        let old = self.focused_node;
        if old == Some(new_id) {
            return None;
        }

        let is_input = self
            .nodes
            .get(new_id)
            .map(|n| n.is_text_input())
            .unwrap_or(false);
        if is_input {
            self.focus_element(new_id);
        } else {
            self.clear_selection();
            self.focused_node = Some(new_id);
        }

        Some(FocusChange { old, new: new_id })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FocusChange {
    pub old: Option<UzNodeId>,
    pub new: UzNodeId,
}

/// Resolve a flat grapheme index to `(entry index, local grapheme index in that entry)`.
/// Indices at the end of the run map to the last non-empty entry's past-the-end.
fn locate_in_run(run: &TextSelectRun, flat_idx: usize) -> Option<(usize, usize)> {
    for (i, entry) in run.entries.iter().enumerate() {
        let end = entry.flat_start + entry.grapheme_count;
        if flat_idx < end {
            return Some((i, flat_idx - entry.flat_start));
        }
    }
    let last = run.entries.iter().rposition(|e| e.grapheme_count > 0)?;
    Some((last, run.entries[last].grapheme_count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Display, TextSelectable, UzStyle};

    fn selectable_style() -> UzStyle {
        // Block display so the construct phase folds the inline children
        // into a single inline-formatting context with populated entries —
        // selection-run building consumes those entries directly without
        // needing a glyph layout pass to run first.
        UzStyle {
            display: Display::Block,
            text_selectable: TextSelectable::True,
            ..Default::default()
        }
    }

    fn prepare_for_select_runs(dom: &mut UIState) {
        let Some(root) = dom.root else { return };
        fn cascade(dom: &mut UIState, id: UzNodeId, parent: Option<UzStyle>) {
            dom.nodes[id].compute_styles(false, false, false, parent.as_ref());
            let style = dom.nodes[id].computed_style().clone();
            let children = dom.nodes[id].children.clone();
            for child in children {
                cascade(dom, child, Some(style.clone()));
            }
        }
        cascade(dom, root, None);
        dom.resolve_layout_children();
    }

    #[test]
    fn selected_text_spans_sibling_text_nodes_with_byte_offsets() {
        let mut dom = UIState::new();
        let root = dom.create_view(selectable_style());
        let first = dom.create_text_element("hello".into(), Default::default());
        let second = dom.create_text_element(" world".into(), Default::default());
        dom.set_root(root);
        dom.append_child(root, first);
        dom.append_child(root, second);
        prepare_for_select_runs(&mut dom);
        dom.build_text_select_runs();

        dom.set_selection(TextSelection::new(
            SelectionEndpoint::new(first, 2, Affinity::Downstream),
            SelectionEndpoint::new(second, 4, Affinity::Upstream),
        ));

        assert_eq!(dom.selected_text(), "llo wor");
    }

    #[test]
    fn removing_node_that_holds_endpoint_clears_selection() {
        let mut dom = UIState::new();
        let root = dom.create_view(selectable_style());
        let first = dom.create_text_element("hello".into(), Default::default());
        let second = dom.create_text_element(" world".into(), Default::default());
        dom.set_root(root);
        dom.append_child(root, first);
        dom.append_child(root, second);
        prepare_for_select_runs(&mut dom);
        dom.build_text_select_runs();
        dom.set_selection(TextSelection::new(
            SelectionEndpoint::new(first, 1, Affinity::Downstream),
            SelectionEndpoint::new(second, 2, Affinity::Upstream),
        ));

        dom.remove_child(root, second);

        assert!(dom.get_text_selection().is_none());
    }
}
