//! Layout-tree construction. Walks the DOM each frame and computes
//! `layout_children` for every container, splicing synthetic anonymous
//! wrapper nodes around runs of inline-level children when those inline
//! children sit alongside block-level siblings.
//! adapted from https://github.com/DioxusLabs/blitz

use crate::element::{ElementNode, InlineLayoutKind, InlineTextEntry, TextLayout};
use crate::node::{Node, NodeData, NodeFlags, UzNodeId};
use crate::style::{Display, UzStyle};
use crate::ui::UIState;

enum ChildLayout {
    /// No inline children: every child becomes a layout child of the
    /// parent and is laid out as its own box.
    AllBlock,
    /// A single `<text>` element child: recurse into it directly so it
    /// owns its own inline-formatting context (preserving its style).
    SingleTextElement,
    /// All children are inline-level and the parent isn't a flex
    /// container the parent itself becomes the inline-formatting root.
    InlineRoot,
    /// Mixed inline/block children, or a flex container with bare text
    /// to wrap. `wrap_inline_elements = false` for flex parents (only
    /// bare text gets wrapped; `<text>` chips stay as flex items).
    Mixed { wrap_inline_elements: bool },
}

impl UIState {
    /// Tear down anonymous wrappers from the previous frame and rebuild the
    /// layout tree. Must be called before `layout_engine.compute_layout`.
    pub fn resolve_layout_children(&mut self) {
        self.tear_down_anonymous_nodes();

        let Some(root) = self.root else { return };
        self.build_layout_children(root);
    }

    fn tear_down_anonymous_nodes(&mut self) {
        self.nodes.retain(|_, node| !node.flags.is_anonymous());

        for (_, node) in self.nodes.iter_mut() {
            *node.layout_children.get_mut() = node.children.clone();
            node.layout_parent = node.parent;

            if let Some(element) = node.as_element_mut() {
                element.inline_layout = None;
            }

            node.flags.reset_construction_flags();
        }
    }

    /// Build the layout tree under `node_id`. Picks one of four strategies
    /// based on the kinds of children present and the parent's display
    /// mode, then recurses.
    fn build_layout_children(&mut self, node_id: UzNodeId) {
        let Some(node) = self.nodes.get(node_id) else {
            return;
        };
        let children = node.children.clone();
        *self.nodes[node_id].layout_children.borrow_mut() = children.clone();
        if children.is_empty() {
            return;
        }

        match self.classify_children(node_id, &children) {
            ChildLayout::AllBlock => {
                self.reparent_as_layout_children(node_id, &children);
                self.build_layout_children_recurse(&children);
            }
            ChildLayout::SingleTextElement => {
                let child_id = children[0];
                self.set_layout_parent(child_id, node_id);
                self.build_layout_children(child_id);
            }
            ChildLayout::InlineRoot => {
                self.become_inline_root(node_id, &children);
            }
            ChildLayout::Mixed {
                wrap_inline_elements,
            } => {
                let layout_children =
                    self.wrap_inline_runs(node_id, &children, wrap_inline_elements);
                self.build_layout_children_recurse(&layout_children);
                *self.nodes[node_id].layout_children.borrow_mut() = layout_children;
            }
        }
    }

    /// Decide how `node_id`'s children should be laid out.
    fn classify_children(&self, node_id: UzNodeId, children: &[UzNodeId]) -> ChildLayout {
        let parent_display = self.nodes[node_id].computed_style().display;

        let mut has_inline_nodes = false;
        let mut has_block_nodes = false;
        for &cid in children {
            let is_inline = self.nodes.get(cid).is_some_and(|n| n.is_inline_level());
            has_inline_nodes |= is_inline;
            has_block_nodes |= !is_inline;
            if has_inline_nodes && has_block_nodes {
                break;
            }
        }

        if !has_inline_nodes {
            return ChildLayout::AllBlock;
        }

        // A lone `<text>` child becomes its own layout box (so its style
        // can drive textAlign, etc.) and we recurse into it instead of
        // wrapping it in an inline-formatting context here.
        if children.len() == 1
            && self
                .nodes
                .get(children[0])
                .is_some_and(|n| n.is_text_element())
        {
            return ChildLayout::SingleTextElement;
        }

        // Pure-inline non-flex container becomes one inline-formatting
        // context owning all its children's text via a parley layout.
        if !has_block_nodes && parent_display != Display::Flex {
            return ChildLayout::InlineRoot;
        }

        // Mixed runs (or a flex container) need anonymous wrappers around
        // contiguous inline runs. In flex containers we only wrap bare
        // text nodes; `<text>` chips remain independent flex items, which
        // matches CSS spec for flex item generation.
        ChildLayout::Mixed {
            wrap_inline_elements: parent_display != Display::Flex,
        }
    }

    /// All-block children: just point each at this parent and recurse.
    fn reparent_as_layout_children(&mut self, node_id: UzNodeId, children: &[UzNodeId]) {
        for &cid in children {
            self.set_layout_parent(cid, node_id);
        }
    }

    /// Convert `node_id` into an inline-formatting-context root: collect
    /// every descendant text fragment into the parent's parley layout,
    /// flag the parent as an inline root, and clear `layout_children`
    /// (they're represented by the parley layout, not by child boxes).
    fn become_inline_root(&mut self, node_id: UzNodeId, children: &[UzNodeId]) {
        let inline = self.collect_inline_layout(children);
        self.set_inline_layout(node_id, inline);
        self.nodes[node_id].flags.insert(NodeFlags::INLINE_ROOT);
        for &cid in children {
            self.set_layout_parent(cid, node_id);
        }
        self.nodes[node_id].layout_children.borrow_mut().clear();
    }

    /// Walk children left-to-right; each contiguous inline run becomes one
    /// anonymous block wrapper that owns an inline-formatting context.
    /// Block-level children pass through unchanged. Returns the new
    /// `layout_children` slice for `node_id`.
    fn wrap_inline_runs(
        &mut self,
        node_id: UzNodeId,
        children: &[UzNodeId],
        wrap_inline_elements: bool,
    ) -> Vec<UzNodeId> {
        let mut layout_children: Vec<UzNodeId> = Vec::with_capacity(children.len());
        let mut open_wrapper: Option<UzNodeId> = None;

        for &cid in children {
            let Some(child) = self.nodes.get(cid) else {
                continue;
            };
            let should_wrap = if wrap_inline_elements {
                child.is_inline_level()
            } else {
                child.is_text_node()
            };

            if should_wrap {
                let wrapper_id = match open_wrapper {
                    Some(id) => id,
                    None => {
                        let wrapper = self.create_anonymous_block(node_id);
                        layout_children.push(wrapper);
                        open_wrapper = Some(wrapper);
                        wrapper
                    }
                };
                self.set_layout_parent(cid, wrapper_id);
                self.nodes[wrapper_id].children.push(cid);
                self.append_inline_layout(wrapper_id, cid);
            } else {
                open_wrapper = None;
                self.set_layout_parent(cid, node_id);
                layout_children.push(cid);
            }
        }

        layout_children
    }

    fn set_layout_parent(&mut self, child_id: UzNodeId, parent_id: UzNodeId) {
        if let Some(child) = self.nodes.get_mut(child_id) {
            child.layout_parent = Some(parent_id);
        }
    }

    fn build_layout_children_recurse(&mut self, ids: &[UzNodeId]) {
        for &id in ids {
            let is_anon = self.nodes.get(id).is_some_and(Node::is_anonymous);
            if !is_anon {
                self.build_layout_children(id);
            }
        }
    }

    fn create_anonymous_block(&mut self, parent_id: UzNodeId) -> UzNodeId {
        let mut node = Node::new(
            UzStyle {
                display: Display::Block,
                ..Default::default()
            },
            NodeData::AnonymousBlock(ElementNode::new_view()),
        );
        node.flags.insert(NodeFlags::ANONYMOUS);
        node.flags.insert(NodeFlags::INLINE_ROOT);
        node.layout_parent = Some(parent_id);
        // Anonymous wrappers participate in the DOM tree for traversal
        // (e.g. selection-root walks) but are never DOM children of their
        // parent — they only live in `layout_children`.
        node.parent = Some(parent_id);
        let id = self.nodes.insert(node);
        // Cascade inheritable styles (text, color, visibility,
        // text_selectable, cursor) from the parent's already-resolved
        // style. The main style cascade pass runs before construct and
        // walks DOM `children`, so it never visits these synthetic
        // wrappers — we have to seed their computed_style here or every
        // consumer that reads it would see uncascaded defaults.
        let parent_style = self.nodes[parent_id].computed_style().clone();
        self.nodes[id].compute_styles(false, false, false, Some(&parent_style));
        id
    }

    fn set_inline_layout(&mut self, node_id: UzNodeId, inline: TextLayout) {
        if let Some(element) = self.nodes[node_id].as_element_mut() {
            element.inline_layout = Some(Box::new(inline));
        }
    }

    fn collect_inline_layout(&self, children: &[UzNodeId]) -> TextLayout {
        let mut entries: Vec<InlineTextEntry> = Vec::new();
        let mut text_len: usize = 0;
        self.collect_inline_text_into(children, &mut entries, &mut text_len, None);
        TextLayout {
            text_len,
            kind: InlineLayoutKind::InlineRoot { entries },
            ..TextLayout::default()
        }
    }

    fn append_inline_layout(&mut self, wrapper_id: UzNodeId, child_id: UzNodeId) {
        let mut new_entries: Vec<InlineTextEntry> = Vec::new();
        let mut new_text_len: usize = 0;
        self.collect_inline_text_into(&[child_id], &mut new_entries, &mut new_text_len, None);

        let Some(element) = self.nodes[wrapper_id].as_element_mut() else {
            return;
        };
        let inline = element.inline_layout.get_or_insert_with(|| {
            Box::new(TextLayout {
                kind: InlineLayoutKind::InlineRoot {
                    entries: Vec::new(),
                },
                ..TextLayout::default()
            })
        });
        let offset = inline.text_len;
        inline.text_len += new_text_len;
        let InlineLayoutKind::InlineRoot { entries } = &mut inline.kind else {
            // Wrapper was just created above as InlineRoot; this branch is
            // only reachable if a caller mutated `kind` out from under us.
            return;
        };
        entries.extend(new_entries.into_iter().map(|mut entry| {
            entry.byte_start += offset;
            entry
        }));
    }

    fn collect_inline_text_into(
        &self,
        children: &[UzNodeId],
        entries: &mut Vec<InlineTextEntry>,
        text_len: &mut usize,
        style_owner: Option<UzNodeId>,
    ) {
        for &node_id in children {
            let Some(node) = self.nodes.get(node_id) else {
                continue;
            };
            match &node.data {
                NodeData::Text(text_node) => {
                    let owner = style_owner.unwrap_or(node_id);
                    push_inline_text(entries, text_len, owner, node_id, text_node.content.len());
                }
                NodeData::Element(el) if node.is_inline_level() => {
                    if let Some(content_len) = el
                        .data
                        .get_text_content()
                        .map(|content| content.content.len())
                        .filter(|len| *len > 0)
                    {
                        push_inline_text(entries, text_len, node_id, node_id, content_len);
                    }
                    let children = node.children.clone();
                    self.collect_inline_text_into(&children, entries, text_len, Some(node_id));
                }
                _ => {}
            }
        }
    }
}

fn push_inline_text(
    entries: &mut Vec<InlineTextEntry>,
    text_len: &mut usize,
    node_id: UzNodeId,
    content_source: UzNodeId,
    content_len: usize,
) {
    if content_len == 0 {
        return;
    }
    let byte_start = *text_len;
    *text_len += content_len;
    // Coalesce with previous entry only when the previous entry came from
    // the exact same content source — otherwise we'd lose the source-node
    // mapping that layout uses to fetch the actual text per fragment.
    if let Some(last) = entries.last_mut()
        && last.node_id == node_id
        && last.content_source == content_source
        && last.byte_start + last.byte_len == byte_start
    {
        last.byte_len += content_len;
        return;
    }
    entries.push(InlineTextEntry {
        node_id,
        content_source,
        byte_start,
        byte_len: content_len,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::UzStyle;

    #[test]
    fn all_inline_children_make_parent_inline_root() {
        let mut dom = UIState::new();
        let parent = dom.create_view(UzStyle::default_for_element("view"));
        let first = dom.create_text_node("hello ".into(), UzStyle::default_for_element("#text"));
        let second = dom.create_text_element("world".into(), UzStyle::default_for_element("text"));

        dom.set_root(parent);
        dom.append_child(parent, first);
        dom.append_child(parent, second);
        dom.resolve_layout_children();

        assert!(dom.nodes[parent].flags.is_inline_root());
        assert!(dom.nodes[parent].layout_children.borrow().is_empty());
        let inline = dom.nodes[parent]
            .as_element()
            .and_then(|element| element.inline_layout.as_ref())
            .expect("parent should own inline layout");
        assert_eq!(inline.text_len, "hello world".len());
        assert_eq!(inline.entries().len(), 2);
        assert_eq!(inline.entries()[0].content_source, first);
        assert_eq!(inline.entries()[1].content_source, second);
    }

    #[test]
    fn mixed_inline_and_block_children_create_anonymous_block_runs() {
        let mut dom = UIState::new();
        let parent = dom.create_view(UzStyle::default_for_element("view"));
        let first = dom.create_text_node("hello".into(), UzStyle::default_for_element("#text"));
        let block = dom.create_view(UzStyle::default_for_element("view"));
        let second = dom.create_text_node("world".into(), UzStyle::default_for_element("#text"));

        dom.set_root(parent);
        dom.append_child(parent, first);
        dom.append_child(parent, block);
        dom.append_child(parent, second);
        dom.resolve_layout_children();

        let layout_children = dom.nodes[parent].layout_children.borrow();
        assert_eq!(layout_children.len(), 3);
        assert!(dom.nodes[layout_children[0]].flags.is_anonymous());
        assert_eq!(layout_children[1], block);
        assert!(dom.nodes[layout_children[2]].flags.is_anonymous());
    }

    #[test]
    fn flex_parent_does_not_become_inline_root() {
        let mut dom = UIState::new();
        let mut flex = UzStyle::default_for_element("view");
        flex.display = Display::Flex;
        let parent = dom.create_view(flex);
        let bare_text = dom.create_text_node("hello".into(), UzStyle::default_for_element("#text"));
        let text_element =
            dom.create_text_element("world".into(), UzStyle::default_for_element("text"));

        dom.set_root(parent);
        dom.append_child(parent, bare_text);
        dom.append_child(parent, text_element);
        dom.resolve_layout_children();

        let layout_children = dom.nodes[parent].layout_children.borrow();
        assert!(!dom.nodes[parent].flags.is_inline_root());
        assert!(dom.nodes[layout_children[0]].flags.is_anonymous());
        assert_eq!(layout_children[1], text_element);
    }

    #[test]
    fn inline_root_entries_track_style_owner_and_content_source() {
        let mut dom = UIState::new();
        let parent = dom.create_view(UzStyle::default_for_element("view"));
        let bare = dom.create_text_node("hi ".into(), UzStyle::default_for_element("#text"));
        let chip = dom.create_text_element("chip".into(), UzStyle::default_for_element("text"));

        dom.set_root(parent);
        dom.append_child(parent, bare);
        dom.append_child(parent, chip);
        dom.resolve_layout_children();

        let inline = dom.nodes[parent]
            .as_element()
            .and_then(|el| el.inline_layout.as_ref())
            .expect("parent should own inline layout");

        assert_eq!(inline.entries().len(), 2);
        let bare_entry = &inline.entries()[0];
        assert_eq!(bare_entry.node_id, bare);
        assert_eq!(bare_entry.content_source, bare);
        let chip_entry = &inline.entries()[1];
        assert_eq!(chip_entry.node_id, chip);
        assert_eq!(chip_entry.content_source, chip);
    }

    #[test]
    fn single_text_element_stays_as_its_own_layout_box() {
        let mut dom = UIState::new();
        let parent = dom.create_view(UzStyle::default_for_element("view"));
        let text = dom.create_text_element("aligned".into(), UzStyle::default_for_element("text"));

        dom.set_root(parent);
        dom.append_child(parent, text);
        dom.resolve_layout_children();

        assert!(!dom.nodes[parent].flags.is_inline_root());
        assert_eq!(dom.nodes[text].layout_parent, Some(parent));
        assert_eq!(
            dom.nodes[parent].layout_children.borrow().as_slice(),
            &[text]
        );
    }
}
