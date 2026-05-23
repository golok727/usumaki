use std::cell::Ref;

use slab::Slab;
use taffy::{
    AvailableSpace, CacheTree, CollapsibleMarginSet, Layout, LayoutBlockContainer,
    LayoutFlexboxContainer, LayoutInput, LayoutOutput, LayoutPartialTree, NodeId, Point, Rect,
    RoundTree, RunMode, Size, TraversePartialTree, TraverseTree, compute_block_layout,
    compute_cached_layout, compute_flexbox_layout, compute_leaf_layout, compute_root_layout,
    round_layout,
};

use crate::element::{InlineLayoutKind, TextLayout};
use crate::node::{Node, NodeData, ScrollAxis, UzNodeId};
use crate::style::Bounds;
use crate::text::{InlineBox, LEAF_BRUSH_ID, TextRenderer};
use crate::ui::UIState;

pub trait TaffyLayoutExt {
    fn border_box_bounds(&self) -> Bounds;
    fn content_box_bounds(&self) -> Bounds;
    fn axis_location(&self, axis: ScrollAxis) -> f32;
    fn axis_size(&self, axis: ScrollAxis) -> f32;
    fn axis_content_box_size(&self, axis: ScrollAxis) -> f32;
    fn axis_scroll_overflow(&self, axis: ScrollAxis) -> f32;
    fn axis_scroll_content_size(&self, axis: ScrollAxis) -> f32;
}

impl TaffyLayoutExt for taffy::Layout {
    fn border_box_bounds(&self) -> Bounds {
        Bounds::new(0.0, 0.0, self.size.width as f64, self.size.height as f64)
    }

    fn content_box_bounds(&self) -> Bounds {
        Bounds::new(
            (self.border.left + self.padding.left) as f64,
            (self.border.top + self.padding.top) as f64,
            (self.content_box_width() - self.scrollbar_size.width).max(0.0) as f64,
            (self.content_box_height() - self.scrollbar_size.height).max(0.0) as f64,
        )
    }

    fn axis_location(&self, axis: ScrollAxis) -> f32 {
        match axis {
            ScrollAxis::X => self.location.x,
            ScrollAxis::Y => self.location.y,
        }
    }

    fn axis_size(&self, axis: ScrollAxis) -> f32 {
        match axis {
            ScrollAxis::X => self.size.width,
            ScrollAxis::Y => self.size.height,
        }
    }

    fn axis_content_box_size(&self, axis: ScrollAxis) -> f32 {
        match axis {
            ScrollAxis::X => self.content_box_bounds().width as f32,
            ScrollAxis::Y => self.content_box_bounds().height as f32,
        }
    }

    fn axis_scroll_overflow(&self, axis: ScrollAxis) -> f32 {
        match axis {
            ScrollAxis::X => self.scroll_width(),
            ScrollAxis::Y => self.scroll_height(),
        }
    }

    fn axis_scroll_content_size(&self, axis: ScrollAxis) -> f32 {
        self.axis_content_box_size(axis) + self.axis_scroll_overflow(axis)
    }
}

pub struct LayoutTree<'a> {
    pub state: &'a mut UIState,
    pub text: &'a mut TextRenderer,
}

impl<'a> LayoutTree<'a> {
    pub fn new(state: &'a mut UIState, text: &'a mut TextRenderer) -> Self {
        Self { state, text }
    }

    fn nodes(&self) -> &Slab<Node> {
        &self.state.nodes
    }

    fn nodes_mut(&mut self) -> &mut Slab<Node> {
        &mut self.state.nodes
    }

    /// Run a full layout pass starting at the document root. Clears every
    /// node's cache first so style/DOM changes from this frame are honored.
    pub fn run(state: &'a mut UIState, text: &'a mut TextRenderer, width: f32, height: f32) {
        let Some(root_id) = state.root else { return };
        for (_, node) in state.nodes.iter_mut() {
            node.cache.clear();
        }
        let mut tree = LayoutTree::new(state, text);
        let root = NodeId::from(root_id);
        compute_root_layout(
            &mut tree,
            root,
            Size {
                width: AvailableSpace::Definite(width),
                height: AvailableSpace::Definite(height),
            },
        );
        round_layout(&mut tree, root);
    }

    fn compute_node_layout(&mut self, node_id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        let raw_id: usize = node_id.into();

        if inputs.run_mode == RunMode::PerformHiddenLayout {
            return taffy::compute_hidden_layout(self, node_id);
        }

        let Some(node) = self.nodes().get(raw_id) else {
            return LayoutOutput::HIDDEN;
        };

        if node.flags.is_inline_root() {
            return self.compute_inline_layout(raw_id, inputs);
        }

        let display = node.taffy_style.display;
        if display == taffy::Display::None {
            return taffy::compute_hidden_layout(self, node_id);
        }

        // Text input is a measured leaf (cursor, scroll, etc. are paint-only).
        if node.is_text_input() {
            return self.compute_text_input_leaf(raw_id, inputs);
        }

        // Checkbox is a fixed-size leaf based on its style size.
        if node.is_checkbox_input() {
            return self.compute_checkbox_leaf(raw_id, inputs);
        }

        // Image is measured from natural size + style box.
        if node.as_image().is_some() {
            return self.compute_image_leaf(raw_id, inputs);
        }

        // Bare text leaf (no inline_root parent has wrapped it). Shouldn't
        // normally happen with the construct phase, but be defensive.
        if matches!(node.data, NodeData::Text(_)) {
            return self.compute_text_leaf(raw_id, inputs);
        }

        // Single `<text>` element that wasn't wrapped: behave like a text leaf.
        if node.is_text_element() && node.layout_children.borrow().is_empty() {
            return self.compute_text_leaf(raw_id, inputs);
        }

        match display {
            taffy::Display::Flex => compute_flexbox_layout(self, node_id, inputs),
            taffy::Display::Block => compute_block_layout(self, node_id, inputs),
            taffy::Display::Grid => compute_block_layout(self, node_id, inputs),
            taffy::Display::None => LayoutOutput::HIDDEN,
        }
    }

    fn compute_text_input_leaf(&mut self, node_id: UzNodeId, inputs: LayoutInput) -> LayoutOutput {
        let style = self.nodes()[node_id].computed_style().clone();
        let line_height = (style.text.font_size * style.text.line_height).round();
        let taffy_style = self.nodes()[node_id].taffy_style.clone();
        compute_leaf_layout(
            inputs,
            &taffy_style,
            |_, _| 0.0,
            |known, avail| {
                let width = known.width.unwrap_or_else(|| match avail.width {
                    AvailableSpace::Definite(v) => v.min(300.0),
                    AvailableSpace::MinContent => 0.0,
                    AvailableSpace::MaxContent => 300.0,
                });
                let height = known.height.unwrap_or(line_height);
                Size { width, height }
            },
        )
    }

    fn compute_checkbox_leaf(&mut self, node_id: UzNodeId, inputs: LayoutInput) -> LayoutOutput {
        let taffy_style = self.nodes()[node_id].taffy_style.clone();
        compute_leaf_layout(
            inputs,
            &taffy_style,
            |_, _| 0.0,
            |known, _| {
                // The style.size already encodes the 18x18 default (see UIState::create_checkbox).
                let w = known.width.unwrap_or(0.0);
                let h = known.height.unwrap_or(0.0);
                let s = w.max(h);
                Size {
                    width: s,
                    height: s,
                }
            },
        )
    }

    fn compute_image_leaf(&mut self, node_id: UzNodeId, inputs: LayoutInput) -> LayoutOutput {
        let taffy_style = self.nodes()[node_id].taffy_style.clone();
        let natural = self.nodes()[node_id]
            .as_image()
            .and_then(|img| img.data.natural_size());
        compute_leaf_layout(
            inputs,
            &taffy_style,
            |_, _| 0.0,
            |known, _| match natural {
                Some((nw, nh)) if nw > 0.0 && nh > 0.0 => {
                    let ratio = nw / nh;
                    let width = known
                        .width
                        .unwrap_or_else(|| known.height.map(|h| h * ratio).unwrap_or(nw));
                    let height = known
                        .height
                        .unwrap_or_else(|| known.width.map(|w| w / ratio).unwrap_or(nh));
                    Size { width, height }
                }
                _ => Size {
                    width: known.width.unwrap_or(0.0),
                    height: known.height.unwrap_or(0.0),
                },
            },
        )
    }

    /// Text leaf measure. Builds the parley layout at the resolved width and
    /// stashes it on the node's element so paint can reuse it without
    /// rebuilding.
    fn compute_text_leaf(&mut self, node_id: UzNodeId, inputs: LayoutInput) -> LayoutOutput {
        let text = self.nodes()[node_id]
            .get_text_content()
            .map(|t| t.content.clone())
            .unwrap_or_default();
        let style = self.nodes()[node_id].computed_style().clone();
        let taffy_style = self.nodes()[node_id].taffy_style.clone();

        // We need the parley layout outside the leaf measure closure so we can
        // stash it. Run the measure inline.
        let mut stashed_layout: Option<parley::Layout<crate::text::TextBrush>> = None;
        let output = compute_leaf_layout(
            inputs,
            &taffy_style,
            |_, _| 0.0,
            |known, avail| {
                // Translate taffy's available_space into a parley wrap
                // width. MinContent must wrap *maximally* (longest word
                // is the minimum width) — passing `None` gives parley a
                // single line and reports max-content as the minimum,
                // which makes flex layout think the item can't shrink
                // and prevents wrapping in fixed-width cells.
                let max_w = known.width.or(match avail.width {
                    AvailableSpace::Definite(v) => Some(v),
                    AvailableSpace::MinContent => Some(0.0),
                    AvailableSpace::MaxContent => None,
                });
                // Use the leaf-brush sentinel so per-span paint code can't
                // mistake this leaf's own glyphs for an inline `<text>` chip
                // span (which would double-paint the leaf's box).
                let layout =
                    self.text
                        .build_inline_layout(&style.text, LEAF_BRUSH_ID, max_w, |builder| {
                            builder.push_text(&text);
                        });
                let w = known.width.unwrap_or_else(|| layout.full_width().ceil());
                let h = known.height.unwrap_or_else(|| layout.height().ceil());
                stashed_layout = Some(layout);
                Size {
                    width: w,
                    height: h,
                }
            },
        );

        if let Some(layout) = stashed_layout
            && let Some(element) = self.nodes_mut()[node_id].as_element_mut()
        {
            let text_len = text.len();
            element.inline_layout = Some(Box::new(TextLayout {
                layout,
                text_len,
                kind: InlineLayoutKind::Leaf,
                ..TextLayout::default()
            }));
        }
        output
    }

    /// Custom inline formatting context. Inline-root nodes own a parley run
    /// where bare text nodes and `<text>` element children contribute styled
    /// spans (each text node's `computed_style` already encodes the inherited
    /// cascade). `<text>` element box styling is painted from span geometry.
    fn compute_inline_layout(&mut self, node_id: UzNodeId, inputs: LayoutInput) -> LayoutOutput {
        let LayoutInput {
            known_dimensions,
            parent_size,
            available_space,
            run_mode,
            sizing_mode,
            ..
        } = inputs;

        let taffy_style = self.nodes()[node_id].taffy_style.clone();
        let computed_style = self.nodes()[node_id].computed_style().clone();

        // Resolve padding/border/margin in pixels (no calc support — we only
        // emit pixel values from to_taffy() today, so this is exact).
        let padding = resolve_rect(&taffy_style.padding, parent_size.width);
        let border = resolve_rect(&taffy_style.border, parent_size.width);
        let pad_border = sum_rect(padding) + sum_rect(border);
        let scrollbar = Size {
            width: if taffy_style.overflow.y == taffy::Overflow::Scroll {
                taffy_style.scrollbar_width
            } else {
                0.0
            },
            height: if taffy_style.overflow.x == taffy::Overflow::Scroll {
                taffy_style.scrollbar_width
            } else {
                0.0
            },
        };
        let inset_w = pad_border.width + scrollbar.width;
        let inset_h = pad_border.height + scrollbar.height;

        // Resolve style-based size constraints (used when known_dimensions are
        // absent).
        let style_size = resolve_dim_size(taffy_style.size, parent_size);
        let style_min = resolve_dim_size(taffy_style.min_size, parent_size);
        let style_max = resolve_dim_size(taffy_style.max_size, parent_size);

        // Pull the inline-root entries built by the construct phase. The
        // text content per fragment is fetched from each entry's source
        // node below — we never materialize a concatenated string.
        let entries = {
            let node = &self.nodes()[node_id];
            node.as_element()
                .and_then(|el| el.inline_layout.as_ref())
                .map(|i| i.entries().to_vec())
                .unwrap_or_default()
        };

        // Available content width — used as the parley wrap width.
        // MinContent must wrap at the longest word so flex containers
        // can shrink us; MaxContent intentionally leaves None so parley
        // reports our full single-line width.
        let available_width_f32 = known_dimensions
            .width
            .map(|w| (w - inset_w).max(0.0))
            .or_else(|| match available_space.width {
                AvailableSpace::Definite(v) => Some((v - inset_w).max(0.0)),
                AvailableSpace::MinContent => Some(0.0),
                AvailableSpace::MaxContent => None,
            })
            .or_else(|| style_size.width.map(|w| (w - inset_w).max(0.0)));

        // Resolve each entry's text slice and styled-span style up front so
        // the build closure (which receives parley by &mut) doesn't need to
        // borrow `self.nodes` while we hold a mut borrow of `self.text`.
        struct InlineFragment {
            node_id: UzNodeId,
            text: String,
            style: crate::style::TextStyle,
            pad_left: f32,
            pad_right: f32,
            line_height: Option<f32>,
        }
        let mut fragments: Vec<InlineFragment> = Vec::with_capacity(entries.len());
        for entry in &entries {
            // Pull the text directly from the entry's source node — the
            // concatenated string the construct phase used to build no
            // longer exists.
            let source_text = self
                .nodes()
                .get(entry.content_source)
                .and_then(|n| n.get_text_content())
                .map(|t| t.content.as_str())
                .unwrap_or("");
            let end = entry.byte_len.min(source_text.len());
            let slice = &source_text[..end];
            let entry_node = &self.nodes()[entry.node_id];
            let entry_style = entry_node.computed_style();
            let (pad_l, pad_r, line_height) = if entry_node.is_text_node() {
                (0.0, 0.0, None)
            } else {
                (
                    entry_style.padding.left + entry_style.border_widths.left,
                    entry_style.padding.right + entry_style.border_widths.right,
                    Some(
                        (entry_style.text.font_size * entry_style.text.line_height).ceil()
                            + entry_style.padding.top
                            + entry_style.padding.bottom
                            + entry_style.border_widths.top
                            + entry_style.border_widths.bottom,
                    ),
                )
            };
            fragments.push(InlineFragment {
                node_id: entry.node_id,
                text: slice.to_owned(),
                style: entry_style.text.clone(),
                pad_left: pad_l,
                pad_right: pad_r,
                line_height,
            });
        }

        let parley_layout = if fragments.is_empty() {
            self.text
                .build_layout("", &computed_style.text, available_width_f32)
        } else {
            self.text.build_inline_layout(
                &computed_style.text,
                node_id,
                available_width_f32,
                |builder| {
                    for frag in &fragments {
                        if frag.pad_left > 0.0 {
                            builder.push_inline_box(InlineBox {
                                id: 0,
                                index: 0,
                                width: frag.pad_left,
                                height: 0.0,
                            });
                        }
                        let mut span_style = frag.style.to_parley_text_style(frag.node_id);
                        if let Some(line_height) = frag.line_height {
                            span_style.line_height = parley::LineHeight::Absolute(line_height);
                        }
                        builder.push_style_span(span_style);
                        builder.push_text(&frag.text);
                        builder.pop_style_span();
                        if frag.pad_right > 0.0 {
                            builder.push_inline_box(InlineBox {
                                id: 0,
                                index: 0,
                                width: frag.pad_right,
                                height: 0.0,
                            });
                        }
                    }
                },
            )
        };

        let measured_w = parley_layout.full_width().ceil();
        let measured_h = parley_layout.height().ceil();

        // Stash the parent's parley layout for paint / hit-test / selection.
        // Construct already populated `kind = InlineRoot { entries }` and
        // `text_len`; we just slot in the freshly built parley layout.
        if let Some(element) = self.nodes_mut()[node_id].as_element_mut()
            && let Some(inline) = element.inline_layout.as_mut()
        {
            inline.layout = parley_layout;
        }

        // Final outer size: include padding/border. Honor known_dimensions /
        // style sizes when present.
        let outer_w = known_dimensions
            .width
            .or(style_size.width)
            .unwrap_or(measured_w + inset_w);
        let outer_h = known_dimensions
            .height
            .or(style_size.height)
            .unwrap_or(measured_h + inset_h);
        let size = Size {
            width: outer_w
                .max(style_min.width.unwrap_or(0.0))
                .min(style_max.width.unwrap_or(f32::INFINITY))
                .max(pad_border.width),
            height: outer_h
                .max(style_min.height.unwrap_or(0.0))
                .min(style_max.height.unwrap_or(f32::INFINITY))
                .max(pad_border.height),
        };

        let _ = run_mode;
        let _ = sizing_mode;

        LayoutOutput {
            size,
            content_size: Size {
                width: measured_w + sum_rect(padding).width,
                height: measured_h + sum_rect(padding).height,
            },
            first_baselines: Point::NONE,
            top_margin: CollapsibleMarginSet::ZERO,
            bottom_margin: CollapsibleMarginSet::ZERO,
            margins_can_collapse_through: false,
        }
    }
}

fn sum_rect(r: Rect<f32>) -> Size<f32> {
    Size {
        width: r.left + r.right,
        height: r.top + r.bottom,
    }
}

fn calc_zero(_: *const (), _: f32) -> f32 {
    0.0
}

fn resolve_rect(r: &Rect<taffy::LengthPercentage>, basis: Option<f32>) -> Rect<f32> {
    use taffy::ResolveOrZero;
    r.resolve_or_zero(basis, calc_zero)
}

fn resolve_dim_size(s: Size<taffy::Dimension>, parent: Size<Option<f32>>) -> Size<Option<f32>> {
    use taffy::MaybeResolve;
    s.maybe_resolve(parent, calc_zero)
}

pub struct RefCellChildIter<'a> {
    items: Ref<'a, [UzNodeId]>,
    idx: usize,
}

impl<'a> RefCellChildIter<'a> {
    fn new(items: Ref<'a, [UzNodeId]>) -> Self {
        Self { items, idx: 0 }
    }
}

impl Iterator for RefCellChildIter<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let v = self.items.get(self.idx)?;
        self.idx += 1;
        Some(NodeId::from(*v))
    }
}

impl<'tree> TraversePartialTree for LayoutTree<'tree> {
    type ChildIter<'a>
        = RefCellChildIter<'a>
    where
        Self: 'a;

    fn child_ids(&self, parent_node_id: NodeId) -> Self::ChildIter<'_> {
        let id: usize = parent_node_id.into();
        let r = self.nodes()[id].layout_children.borrow();
        RefCellChildIter::new(Ref::map(r, |v| v.as_slice()))
    }

    fn child_count(&self, parent_node_id: NodeId) -> usize {
        let id: usize = parent_node_id.into();
        self.nodes()[id].layout_children.borrow().len()
    }

    fn get_child_id(&self, parent_node_id: NodeId, child_index: usize) -> NodeId {
        let id: usize = parent_node_id.into();
        NodeId::from(self.nodes()[id].layout_children.borrow()[child_index])
    }
}

impl<'tree> TraverseTree for LayoutTree<'tree> {}

impl<'tree> LayoutPartialTree for LayoutTree<'tree> {
    type CoreContainerStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;
    type CustomIdent = <taffy::Style as taffy::CoreStyle>::CustomIdent;

    fn get_core_container_style(&self, node_id: NodeId) -> Self::CoreContainerStyle<'_> {
        let id: usize = node_id.into();
        &self.nodes()[id].taffy_style
    }

    fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
        let id: usize = node_id.into();
        self.nodes_mut()[id].unrounded_layout = *layout;
    }

    fn compute_child_layout(&mut self, node_id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
            tree.compute_node_layout(node_id, inputs)
        })
    }
}

impl<'tree> LayoutBlockContainer for LayoutTree<'tree> {
    type BlockContainerStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;
    type BlockItemStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;

    fn get_block_container_style(&self, node_id: NodeId) -> Self::BlockContainerStyle<'_> {
        let id: usize = node_id.into();
        &self.nodes()[id].taffy_style
    }

    fn get_block_child_style(&self, child_node_id: NodeId) -> Self::BlockItemStyle<'_> {
        let id: usize = child_node_id.into();
        &self.nodes()[id].taffy_style
    }
}

impl<'tree> LayoutFlexboxContainer for LayoutTree<'tree> {
    type FlexboxContainerStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;
    type FlexboxItemStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, node_id: NodeId) -> Self::FlexboxContainerStyle<'_> {
        let id: usize = node_id.into();
        &self.nodes()[id].taffy_style
    }

    fn get_flexbox_child_style(&self, child_node_id: NodeId) -> Self::FlexboxItemStyle<'_> {
        let id: usize = child_node_id.into();
        &self.nodes()[id].taffy_style
    }
}

impl<'tree> CacheTree for LayoutTree<'tree> {
    fn cache_get(
        &self,
        node_id: NodeId,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
        run_mode: RunMode,
    ) -> Option<LayoutOutput> {
        let id: usize = node_id.into();
        self.nodes()[id]
            .cache
            .get(known_dimensions, available_space, run_mode)
    }

    fn cache_store(
        &mut self,
        node_id: NodeId,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
        run_mode: RunMode,
        layout_output: LayoutOutput,
    ) {
        let id: usize = node_id.into();
        self.nodes_mut()[id].cache.store(
            known_dimensions,
            available_space,
            run_mode,
            layout_output,
        );
    }

    fn cache_clear(&mut self, node_id: NodeId) {
        let id: usize = node_id.into();
        self.nodes_mut()[id].cache.clear();
    }
}

impl<'tree> RoundTree for LayoutTree<'tree> {
    fn get_unrounded_layout(&self, node_id: NodeId) -> Layout {
        let id: usize = node_id.into();
        self.nodes()[id].unrounded_layout
    }

    fn set_final_layout(&mut self, node_id: NodeId, layout: &Layout) {
        let id: usize = node_id.into();
        self.nodes_mut()[id].final_layout = *layout;
    }
}
