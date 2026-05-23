use std::collections::HashMap;

use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color as VelloColor, Fill};

use crate::layout::TaffyLayoutExt;
use crate::node::{ScrollAxis, UzNodeId};
use crate::paint::{
    checkbox::CheckboxRenderInfo,
    image::ImageRenderInfo,
    input::InputRenderInfo,
    scroll::{self, ScrollAxisInfo, ThumbGeometry},
};
use crate::style::{Bounds, Overflow, ScrollbarStyle, UzStyle, Visibility};
use crate::text::{
    LEAF_BRUSH_ID, TextBrush, TextRenderer, apply_text_style_to_editor, secure_cursor_geometry,
    secure_selection_geometry,
};
use crate::ui::UIState;

pub struct Painter<'a> {
    dom: &'a mut UIState,
    text_renderer: &'a mut TextRenderer,
    scale: f64,
}

impl<'a> Painter<'a> {
    pub fn new(dom: &'a mut UIState, text_renderer: &'a mut TextRenderer, scale: f64) -> Self {
        Self {
            dom,
            text_renderer,
            scale,
        }
    }

    pub fn paint(mut self, scene: &mut Scene) {
        // Hit tree (hitboxes + scroll-thumb hit rects) is now built by
        // `crate::hit_tree::rebuild` independently of paint, so it can be
        // refreshed eagerly on scroll/mutation. Paint just consumes
        // current scroll state to draw.
        self.dom.build_text_select_runs();

        let text_selections = self.compute_text_selections_map();

        let Some(root) = self.dom.root else {
            return;
        };

        self.render_node(
            root,
            0.0,
            0.0,
            Affine::scale(self.scale),
            Affine::IDENTITY,
            scene,
            &text_selections,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_node(
        &mut self,
        node_id: UzNodeId,
        parent_x: f64,
        parent_y: f64,
        parent_paint_transform: Affine,
        parent_hit_transform: Affine,
        scene: &mut Scene,
        text_selections: &HashMap<UzNodeId, (usize, usize)>,
    ) {
        let Some(node_ref) = self.dom.nodes.get(node_id) else {
            return;
        };
        let layout = node_ref.final_layout;

        let computed_style = node_ref.computed_style().clone();

        if computed_style.visibility == Visibility::Hidden
            || computed_style.display == crate::style::Display::None
        {
            return;
        }

        let border_box = layout.border_box_bounds();
        let x = parent_x + layout.location.x as f64;
        let y = parent_y + layout.location.y as f64;
        let w = border_box.width;
        let h = border_box.height;

        let local_style_transform = computed_style.transform.to_affine(w, h);
        let local_translate =
            Affine::translate((layout.location.x as f64, layout.location.y as f64));
        let transform = parent_paint_transform * local_translate * local_style_transform;
        let hit_transform = parent_hit_transform * local_translate * local_style_transform;

        self.paint_node(
            node_id,
            &layout,
            &computed_style,
            border_box,
            Bounds::new(x, y, w, h),
            transform,
            scene,
            text_selections,
        );

        let view_scroll = self.prepare_view_scroll(
            node_id,
            &computed_style,
            &layout,
            Bounds::new(x, y, w, h),
            transform,
        );

        let needs_clip = view_scroll.is_some()
            || computed_style.overflow_x.clips()
            || computed_style.overflow_y.clips();

        let (content_box, offset_x, offset_y, mouse_in_view, view_thumbs) = match view_scroll {
            Some(v) => (
                v.content_box,
                v.offset_x,
                v.offset_y,
                v.mouse_in_view,
                v.thumbs,
            ),
            None => (layout.content_box_bounds(), 0.0, 0.0, false, Vec::new()),
        };

        let scroll_translate = if offset_x != 0.0 || offset_y != 0.0 {
            Affine::translate((-offset_x, -offset_y))
        } else {
            Affine::IDENTITY
        };
        let child_paint_transform = transform * scroll_translate;
        let child_hit_transform = hit_transform * scroll_translate;

        if needs_clip {
            scene.push_clip_layer(Fill::NonZero, transform, &content_box.to_rect());
        }

        let children = self.dom.nodes[node_id].layout_children.borrow().clone();
        for child_id in children {
            self.render_node(
                child_id,
                x - offset_x,
                y - offset_y,
                child_paint_transform,
                child_hit_transform,
                scene,
                text_selections,
            );
        }

        if needs_clip {
            scene.pop_layer();
        }

        if mouse_in_view {
            for thumb in view_thumbs {
                scroll::paint_thumb(
                    scene,
                    thumb.transform,
                    &thumb.geom,
                    thumb.hovered,
                    thumb.active,
                    &thumb.style,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_node(
        &mut self,
        node_id: UzNodeId,
        layout: &taffy::Layout,
        style: &UzStyle,
        bounds: Bounds,      // local (0,0,w,h)
        view_bounds: Bounds, // absolute (x,y,w,h) for scrollbar placement
        transform: Affine,
        scene: &mut Scene,
        text_selections: &HashMap<UzNodeId, (usize, usize)>,
    ) {
        if self.dom.nodes[node_id].is_text_input() {
            // TODO: dont snapshot
            let info = self.build_input_render_info(node_id, style, layout);
            crate::paint::input::paint_input(
                scene,
                self.text_renderer,
                bounds,
                style,
                layout.content_box_bounds(),
                &info,
                transform,
            );
            // Input scrollbar paints outside the input's text clip.
            if let Some(thumb) = self.register_input_scrollbar(
                node_id,
                style,
                layout,
                &info,
                transform,
                view_bounds.x,
                view_bounds.y,
            ) {
                scroll::paint_thumb(
                    scene,
                    thumb.transform,
                    &thumb.geom,
                    thumb.hovered,
                    thumb.active,
                    &thumb.style,
                );
            }
            return;
        }

        let node = &self.dom.nodes[node_id];

        if let Some(&checked) = node.as_checkbox_input() {
            let info = CheckboxRenderInfo {
                checked,
                focused: self.dom.focused_node == Some(node_id),
            };
            crate::paint::checkbox::paint_checkbox(scene, bounds, style, &info, transform);
        } else if let Some(img) = node.as_image() {
            let info = ImageRenderInfo {
                data: img.data.clone(),
            };
            crate::paint::image::paint_image(scene, bounds, style, &info, transform);
        } else if let Some(inline) = node
            .as_element()
            .and_then(|element| element.inline_layout.as_ref())
        {
            // Either an inline-root parent (its parley layout holds a flat
            // run of bare text + atomic boxes for inline element children)
            // or a chip rendering its own text. In both cases parley already
            // has the glyphs; the per-node bg/border is drawn by paint_view
            // through the normal render_node recursion using final_layout.
            let is_inline_root = inline.is_inline_root();
            let sel = if is_inline_root {
                self.compute_inline_selection(node_id)
            } else {
                text_selections.get(&node_id).copied()
            };
            self.paint_text_node(
                scene,
                bounds,
                style,
                layout.content_box_bounds(),
                &inline.layout,
                inline.text_len,
                transform,
                sel,
                is_inline_root,
            );
        } else {
            crate::paint::view::paint_view(scene, bounds, style, transform, |_| {});
        }
    }

    /// Draw an inline-formatting-context node from its cached parley layout.
    /// Inline `<text>` element children contribute styled spans (identified by
    /// `TextBrush::id == node_id`); their bg/border/padding/corner-radii are
    /// painted here per line by reconstructing each span's bbox from parley
    /// glyph-run geometry, then routed through `UzStyle::paint` so they share
    /// every visual feature with regular block elements. Glyph color comes
    /// from each span's owning node so per-span `color` works.
    #[allow(clippy::too_many_arguments)]
    fn paint_text_node(
        &self,
        scene: &mut Scene,
        bounds: Bounds,
        style: &UzStyle,
        content_box: Bounds,
        layout: &parley::Layout<TextBrush>,
        text_len: usize,
        transform: Affine,
        selection: Option<(usize, usize)>,
        is_inline_root: bool,
    ) {
        style.paint(bounds, scene, transform, |scene| {
            let text_x = content_box.x;
            let text_y = content_box.y;

            // Per-span chip boxes only make sense for inline-root layouts where
            // distinct `<text>` element children contribute spans. Standalone
            // text leaves reuse the same parley layout but have no spans —
            // their box is already drawn by the outer `style.paint` above.
            if is_inline_root {
                self.paint_inline_span_boxes(scene, layout, text_x, text_y, transform);
            }

            if let Some((sel_start, sel_end)) = selection {
                let rects =
                    crate::text::selection_rects_from_layout(layout, text_len, sel_start, sel_end);
                let sel_color = VelloColor::from_rgba8(56, 121, 185, 128);
                for rect in rects {
                    scene.fill(
                        Fill::NonZero,
                        transform,
                        sel_color,
                        None,
                        &Rect::new(
                            rect.x0 + text_x,
                            rect.y0 + text_y,
                            rect.x1 + text_x,
                            rect.y1 + text_y,
                        ),
                    );
                }
            }

            let nodes = &self.dom.nodes;
            let parent_color = style.text.color.to_vello();
            crate::text::draw_layout_with_brush(
                scene,
                layout,
                (text_x as f32, text_y as f32),
                transform,
                |brush| {
                    nodes
                        .get(brush.id)
                        .map(|node| node.computed_style().text.color.to_vello())
                        .unwrap_or(parent_color)
                },
            );
        });
    }

    /// Per-span bg/border painting. For each line, gather contiguous glyph
    /// runs by brush.id and produce one bounds rect per (span, line). The
    /// vertical extent is the full line box (so adjacent lines of the same
    /// span visually merge); horizontal extent is the run's `offset..offset
    /// + advance`, expanded by the span node's left/right padding and border.
    /// Then `UzStyle::paint` draws the same bg/border/corner/shadow stack as any other element
    fn paint_inline_span_boxes(
        &self,
        scene: &mut Scene,
        layout: &parley::Layout<TextBrush>,
        text_x: f64,
        text_y: f64,
        transform: Affine,
    ) {
        use parley::PositionedLayoutItem;

        for line in layout.lines() {
            let metrics = line.metrics();
            let line_top = metrics.min_coord as f64;
            let line_bottom = metrics.max_coord as f64;
            let line_height = (line_bottom - line_top).max(0.0);

            let mut current_id: Option<usize> = None;
            let mut seg_start: f32 = 0.0;
            let mut seg_end: f32 = 0.0;
            let flush = |id: usize, start: f32, end: f32, scene: &mut Scene| {
                if id == LEAF_BRUSH_ID {
                    return;
                }
                let Some(node) = self.dom.nodes.get(id) else {
                    return;
                };
                if !node.is_text_element() {
                    return;
                }
                let style = node.computed_style();
                let has_box = style.background.is_some()
                    || style.border_widths.any_nonzero()
                    || style.box_shadow.is_some();
                if !has_box {
                    return;
                }
                let inset_l = (style.padding.left + style.border_widths.left) as f64;
                let inset_r = (style.padding.right + style.border_widths.right) as f64;
                let visual_h = (style.text.font_size * style.text.line_height
                    + style.padding.top
                    + style.padding.bottom
                    + style.border_widths.top
                    + style.border_widths.bottom) as f64;
                let bx = text_x + start as f64 - inset_l;
                let bh = line_height.max(visual_h);
                let by = text_y + line_top + (line_height - bh) * 0.5;
                let bw = (end - start) as f64 + inset_l + inset_r;
                let bounds = Bounds::new(bx, by, bw, bh);
                style.paint(bounds, scene, transform, |_| {});
            };

            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(run) = item else {
                    continue;
                };
                let id = run.style().brush.id;
                let run_start = run.offset();
                let run_end = run_start + run.advance();
                match current_id {
                    Some(cur) if cur == id => {
                        seg_end = run_end;
                    }
                    Some(cur) => {
                        flush(cur, seg_start, seg_end, scene);
                        current_id = Some(id);
                        seg_start = run_start;
                        seg_end = run_end;
                    }
                    None => {
                        current_id = Some(id);
                        seg_start = run_start;
                        seg_end = run_end;
                    }
                }
            }
            if let Some(id) = current_id {
                flush(id, seg_start, seg_end, scene);
            }
        }
    }

    fn build_input_render_info(
        &mut self,
        node_id: UzNodeId,
        style: &UzStyle,
        layout: &taffy::Layout,
    ) -> InputRenderInfo {
        let text_w = layout.content_box_width().max(0.0);
        let text_style = style.text.clone();

        // Grab seed values first so we can drop the immutable borrow.
        let (multiline, secure, has_preedit) = {
            let is = self.dom.nodes[node_id].as_text_input().unwrap();
            (is.multiline, is.secure, is.preedit.is_some())
        };

        let focused = self.dom.focused_node == Some(node_id);

        let node_mut = &mut self.dom.nodes[node_id];
        let is = node_mut.as_text_input_mut().unwrap();

        apply_text_style_to_editor(&mut is.editor, &text_style);
        is.editor
            .set_width(if multiline { Some(text_w) } else { None });
        is.editor.refresh_layout(
            &mut self.text_renderer.font_ctx,
            &mut self.text_renderer.layout_ctx,
        );

        let blink_visible = is.blink_visible(focused, self.dom.window_focused);

        let cursor_rect = if blink_visible || has_preedit {
            if secure {
                secure_cursor_geometry(&is.editor, 1.5, &text_style, self.text_renderer)
            } else {
                is.editor.cursor_geometry(1.5)
            }
        } else {
            None
        };

        let selection_rects = if secure {
            secure_selection_geometry(&is.editor, &text_style, self.text_renderer)
        } else {
            is.editor
                .selection_geometry()
                .into_iter()
                .map(|(bb, _)| bb)
                .collect()
        };

        let layout_height = is.editor.try_layout().map(|l| l.height()).unwrap_or(0.0);

        let preedit = is.preedit.clone().map(|ps| {
            let positions = self
                .text_renderer
                .grapheme_x_positions(&ps.text, &text_style);
            let width = *positions.last().unwrap_or(&0.0);
            crate::paint::input::PreeditRenderInfo {
                text: ps.text,
                cursor_x: ps
                    .cursor
                    .map(|(start, _)| {
                        if start < positions.len() {
                            positions[start]
                        } else {
                            width
                        }
                    })
                    .unwrap_or(width),
                width,
            }
        });

        let display_text = is.display_text();
        let placeholder = is.placeholder.clone();
        let scroll_offset_x = node_mut.scroll_state.scroll_offset_x;
        let scroll_offset_y = node_mut.scroll_state.scroll_offset_y;

        InputRenderInfo {
            display_text,
            placeholder,
            text_style,
            focused,
            cursor_rect,
            selection_rects,
            scroll_offset_x,
            scroll_offset_y,
            blink_visible,
            multiline,
            layout_height,
            preedit,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn register_input_scrollbar(
        &mut self,
        node_id: UzNodeId,
        style: &UzStyle,
        layout: &taffy::Layout,
        info: &InputRenderInfo,
        transform: Affine,
        view_x: f64,
        view_y: f64,
    ) -> Option<ScrollbarPaint> {
        if !info.multiline {
            return None;
        }
        let border_box = layout.border_box_bounds();
        let content_box = layout.content_box_bounds();
        let axis_info = ScrollAxisInfo {
            content_size: info.layout_height as f64,
            visible_size: content_box.height,
            offset: info.scroll_offset_y as f64,
        };
        if !axis_info.overflows() {
            return None;
        }
        // Scroll-state clamping and `scroll_thumbs` registration are
        // owned by `hit_tree::rebuild` — we just compute the visual geom
        // here for drawing.
        let view_local = border_box;
        let geom = scroll::thumb_geometry(ScrollAxis::Y, view_local, axis_info, &style.scrollbar);
        let view_bounds = Bounds::new(view_x, view_y, border_box.width, border_box.height);
        let thumb_bounds = Bounds::new(
            view_x + geom.local_x,
            view_y + geom.local_y,
            geom.thumb_width,
            geom.thumb_height,
        );

        let mouse_in = self.is_active_drag(node_id)
            || self
                .dom
                .hit_state
                .mouse_position
                .is_some_and(|(mx, my)| view_bounds.contains(mx, my));
        if !mouse_in {
            return None;
        }

        let active = self.is_active_drag_axis(node_id, ScrollAxis::Y);
        let hovered = !active
            && self
                .dom
                .hit_state
                .mouse_position
                .is_some_and(|(mx, my)| thumb_bounds.contains(mx, my));

        Some(ScrollbarPaint {
            transform,
            geom,
            hovered,
            active,
            style: style.scrollbar,
        })
    }

    fn prepare_view_scroll(
        &mut self,
        node_id: UzNodeId,
        style: &UzStyle,
        layout: &taffy::Layout,
        view_bounds: Bounds,
        transform: Affine,
    ) -> Option<ViewScroll> {
        let scroll_x = style.overflow_x.is_scrollable();
        let scroll_y = style.overflow_y.is_scrollable();
        if !scroll_x && !scroll_y {
            return None;
        }

        let (shows_x, shows_y) = visible_scrollbars(style, layout);
        let content_box = scroll_content_box(layout, style, shows_x, shows_y);
        let visible_w = content_box.width;
        let visible_h = content_box.height;
        let content_w = layout.axis_scroll_content_size(ScrollAxis::X) as f64;
        let content_h = layout.axis_scroll_content_size(ScrollAxis::Y) as f64;

        // Scroll offsets were already clamped by `hit_tree::rebuild`;
        // just read them here.
        let ss = &self.dom.nodes[node_id].scroll_state;
        let offset_x = ss.scroll_offset_x as f64;
        let offset_y = ss.scroll_offset_y as f64;

        let mouse_in_view = self.is_active_drag(node_id)
            || self
                .dom
                .hit_state
                .mouse_position
                .is_some_and(|(mx, my)| view_bounds.contains(mx, my));

        let mut thumbs = Vec::new();
        if shows_y {
            thumbs.push(self.register_view_thumb(
                node_id,
                ScrollAxis::Y,
                view_bounds,
                content_h,
                visible_h,
                offset_y,
                transform,
                &style.scrollbar,
            ));
        }
        if shows_x {
            thumbs.push(self.register_view_thumb(
                node_id,
                ScrollAxis::X,
                view_bounds,
                content_w,
                visible_w,
                offset_x,
                transform,
                &style.scrollbar,
            ));
        }

        Some(ViewScroll {
            content_box,
            offset_x,
            offset_y,
            mouse_in_view,
            thumbs,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn register_view_thumb(
        &mut self,
        node_id: UzNodeId,
        axis: ScrollAxis,
        view_bounds: Bounds,
        content: f64,
        visible: f64,
        offset: f64,
        transform: Affine,
        scrollbar: &ScrollbarStyle,
    ) -> ScrollbarPaint {
        // `hit_tree::rebuild` registered the thumb hit rect; we just
        // compute the visual geometry to draw.
        let view_local = Bounds::new(0.0, 0.0, view_bounds.width, view_bounds.height);
        let geom = scroll::thumb_geometry(
            axis,
            view_local,
            ScrollAxisInfo {
                content_size: content,
                visible_size: visible,
                offset,
            },
            scrollbar,
        );
        let thumb_bounds = Bounds::new(
            view_bounds.x + geom.local_x,
            view_bounds.y + geom.local_y,
            geom.thumb_width,
            geom.thumb_height,
        );

        let active = self.is_active_drag_axis(node_id, axis);
        let hovered = !active
            && self
                .dom
                .hit_state
                .mouse_position
                .is_some_and(|(mx, my)| thumb_bounds.contains(mx, my));

        ScrollbarPaint {
            transform,
            geom,
            hovered,
            active,
            style: *scrollbar,
        }
    }

    fn is_active_drag(&self, node_id: UzNodeId) -> bool {
        self.dom
            .drag_mode
            .as_scrollbar_thumb()
            .is_some_and(|d| d.node_id == node_id)
    }

    fn is_active_drag_axis(&self, node_id: UzNodeId, axis: ScrollAxis) -> bool {
        self.dom
            .drag_mode
            .as_scrollbar_thumb()
            .is_some_and(|d| d.node_id == node_id && d.axis == axis)
    }

    fn compute_inline_selection(&self, layout_node_id: UzNodeId) -> Option<(usize, usize)> {
        let sel = self.dom.text_selection;
        if sel.is_collapsed() {
            return None;
        }
        let (start, end) = self.dom.ordered_text_selection()?;
        let run = self.dom.find_run_for_node(start.node)?;
        let mut range_start = None;
        let mut range_end = None;
        let mut in_range = false;
        for entry in &run.entries {
            if entry.node_id == start.node {
                in_range = true;
            }
            if !in_range {
                continue;
            }
            if entry.layout_node_id != layout_node_id {
                if entry.node_id == end.node {
                    break;
                }
                continue;
            }
            let local_start = if entry.node_id == start.node {
                start.offset.min(entry.byte_len)
            } else {
                0
            };
            let local_end = if entry.node_id == end.node {
                end.offset.min(entry.byte_len)
            } else {
                entry.byte_len
            };
            let byte_start = entry.flat_byte_start + local_start;
            let byte_end = entry.flat_byte_start + local_end;
            range_start = Some(range_start.map_or(byte_start, |v: usize| v.min(byte_start)));
            range_end = Some(range_end.map_or(byte_end, |v: usize| v.max(byte_end)));
            if entry.node_id == end.node {
                break;
            }
        }
        Some((range_start?, range_end?))
    }

    fn compute_text_selections_map(&self) -> HashMap<UzNodeId, (usize, usize)> {
        let mut map = HashMap::new();
        let sel = self.dom.text_selection;
        if sel.is_collapsed() {
            return map;
        }
        let Some((start, end)) = self.dom.ordered_text_selection() else {
            return map;
        };
        let Some(run) = self.dom.find_run_for_node(start.node) else {
            return map;
        };
        let mut in_range = false;
        for entry in &run.entries {
            if entry.node_id == start.node {
                in_range = true;
            }
            if !in_range {
                continue;
            }
            let Some(text) = self
                .dom
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
                map.insert(entry.node_id, (local_start, local_end));
            }
            if entry.node_id == end.node {
                break;
            }
        }
        map
    }
}

fn visible_scrollbars(style: &UzStyle, layout: &taffy::Layout) -> (bool, bool) {
    let content_w = layout.axis_scroll_content_size(ScrollAxis::X) as f64;
    let content_h = layout.axis_scroll_content_size(ScrollAxis::Y) as f64;
    let visible_w = layout.axis_content_box_size(ScrollAxis::X) as f64;
    let visible_h = layout.axis_content_box_size(ScrollAxis::Y) as f64;
    let shows_x = scrollbar_visible(style.overflow_x, content_w, visible_w);
    let shows_y = scrollbar_visible(style.overflow_y, content_h, visible_h);

    (shows_x, shows_y)
}

fn scrollbar_visible(overflow: Overflow, content: f64, visible: f64) -> bool {
    match overflow {
        Overflow::Scroll => true,
        Overflow::Auto => content > visible + 0.5,
        Overflow::Visible | Overflow::Hidden => false,
    }
}

fn scroll_content_box(
    layout: &taffy::Layout,
    style: &UzStyle,
    shows_x: bool,
    shows_y: bool,
) -> Bounds {
    let mut content_box = layout.content_box_bounds();
    let gutter = style.scrollbar.gutter_width() as f64;
    content_box.width = (content_box.width
        - auto_scrollbar_gutter(
            style.overflow_y,
            layout.scrollbar_size.width,
            shows_y,
            gutter,
        ))
    .max(0.0);
    content_box.height = (content_box.height
        - auto_scrollbar_gutter(
            style.overflow_x,
            layout.scrollbar_size.height,
            shows_x,
            gutter,
        ))
    .max(0.0);
    content_box
}

fn auto_scrollbar_gutter(
    overflow: Overflow,
    reserved_gutter: f32,
    visible: bool,
    gutter: f64,
) -> f64 {
    if overflow == Overflow::Auto && visible {
        (gutter - reserved_gutter as f64).max(0.0)
    } else {
        0.0
    }
}

struct ViewScroll {
    content_box: Bounds,
    offset_x: f64,
    offset_y: f64,
    mouse_in_view: bool,
    thumbs: Vec<ScrollbarPaint>,
}

#[derive(Clone)]
struct ScrollbarPaint {
    transform: Affine,
    geom: ThumbGeometry,
    hovered: bool,
    active: bool,
    style: ScrollbarStyle,
}

#[cfg(test)]
mod tests {

    // todo add more tests with for the new layout engine
    use super::{scroll_content_box, visible_scrollbars};
    use crate::style::{Overflow, UzStyle};

    fn scroll_layout(
        width: f32,
        height: f32,
        content_width: f32,
        content_height: f32,
    ) -> taffy::Layout {
        let mut layout = taffy::Layout::new();
        layout.size.width = width;
        layout.size.height = height;
        layout.content_size.width = content_width;
        layout.content_size.height = content_height;
        layout
    }

    fn auto_scroll_style() -> UzStyle {
        // Gutter mode reserves a lane for the scrollbar — these tests
        // verify the gutter math. The default Overlay mode reserves no
        // space and would make the assertions trivial.
        UzStyle {
            overflow_x: Overflow::Auto,
            overflow_y: Overflow::Auto,
            scrollbar: crate::style::ScrollbarStyle {
                mode: crate::style::ScrollbarMode::Gutter,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn gutter_scrollbar() -> crate::style::ScrollbarStyle {
        crate::style::ScrollbarStyle {
            mode: crate::style::ScrollbarMode::Gutter,
            ..Default::default()
        }
    }

    #[test]
    fn auto_scroll_has_no_gutter_when_content_fits() {
        let style = auto_scroll_style();
        let mut layout = scroll_layout(100.0, 100.0, 92.0, 92.0);
        layout.scrollbar_size.width = style.scrollbar.gutter_width();
        layout.scrollbar_size.height = style.scrollbar.gutter_width();
        let (shows_x, shows_y) = visible_scrollbars(&style, &layout);
        let content_box = scroll_content_box(&layout, &style, shows_x, shows_y);

        assert_eq!((shows_x, shows_y), (false, false));
        assert_eq!(content_box.width, 92.0);
        assert_eq!(content_box.height, 92.0);
    }

    #[test]
    fn auto_scroll_vertical_overflow_adds_only_vertical_gutter() {
        let style = UzStyle {
            overflow_y: Overflow::Auto,
            scrollbar: gutter_scrollbar(),
            ..Default::default()
        };
        let mut layout = scroll_layout(100.0, 100.0, 100.0, 150.0);
        layout.scrollbar_size.width = style.scrollbar.gutter_width();
        let (shows_x, shows_y) = visible_scrollbars(&style, &layout);
        let content_box = scroll_content_box(&layout, &style, shows_x, shows_y);

        assert_eq!((shows_x, shows_y), (false, true));
        assert_eq!(content_box.width, 92.0);
        assert_eq!(content_box.height, 100.0);
    }

    #[test]
    fn auto_scroll_horizontal_overflow_adds_only_horizontal_gutter() {
        let style = UzStyle {
            overflow_x: Overflow::Auto,
            scrollbar: gutter_scrollbar(),
            ..Default::default()
        };
        let mut layout = scroll_layout(100.0, 100.0, 150.0, 100.0);
        layout.scrollbar_size.height = style.scrollbar.gutter_width();
        let (shows_x, shows_y) = visible_scrollbars(&style, &layout);
        let content_box = scroll_content_box(&layout, &style, shows_x, shows_y);

        assert_eq!((shows_x, shows_y), (true, false));
        assert_eq!(content_box.width, 100.0);
        assert_eq!(content_box.height, 92.0);
    }

    // #[test]
    // fn image_measure_uses_natural_size_when_unconstrained() {
    //     let mut renderer = TextRenderer::new();
    //     let (nodes, mut ctx) = image_nodes(320, 180);
    //     let size = measure(
    //         &mut renderer,
    //         &nodes,
    //         taffy::Size {
    //             width: None,
    //             height: None,
    //         },
    //         taffy::Size {
    //             width: taffy::AvailableSpace::MaxContent,
    //             height: taffy::AvailableSpace::MaxContent,
    //         },
    //         Some(&mut ctx),
    //     );
    //     assert_eq!(size.width, 320.0);
    //     assert_eq!(size.height, 180.0);
    // }

    // #[test]
    // fn image_measure_preserves_aspect_ratio_with_width_only() {
    //     let mut renderer = TextRenderer::new();
    //     let (nodes, mut ctx) = image_nodes(400, 200);
    //     let size = measure(
    //         &mut renderer,
    //         &nodes,
    //         taffy::Size {
    //             width: Some(160.0),
    //             height: None,
    //         },
    //         taffy::Size {
    //             width: taffy::AvailableSpace::MaxContent,
    //             height: taffy::AvailableSpace::MaxContent,
    //         },
    //         Some(&mut ctx),
    //     );
    //     assert_eq!(size.width, 160.0);
    //     assert_eq!(size.height, 80.0);
    // }

    // #[test]
    // fn image_measure_preserves_aspect_ratio_with_height_only() {
    //     let mut renderer = TextRenderer::new();
    //     let (nodes, mut ctx) = image_nodes(200, 400);
    //     let size = measure(
    //         &mut renderer,
    //         &nodes,
    //         taffy::Size {
    //             width: None,
    //             height: Some(100.0),
    //         },
    //         taffy::Size {
    //             width: taffy::AvailableSpace::MaxContent,
    //             height: taffy::AvailableSpace::MaxContent,
    //         },
    //         Some(&mut ctx),
    //     );
    //     assert_eq!(size.width, 50.0);
    //     assert_eq!(size.height, 100.0);
    // }

    // #[test]
    // fn image_measure_uses_explicit_box_when_both_dimensions_are_known() {
    //     let mut renderer = TextRenderer::new();
    //     let (nodes, mut ctx) = image_nodes(320, 180);
    //     let size = measure(
    //         &mut renderer,
    //         &nodes,
    //         taffy::Size {
    //             width: Some(512.0),
    //             height: Some(128.0),
    //         },
    //         taffy::Size {
    //             width: taffy::AvailableSpace::MaxContent,
    //             height: taffy::AvailableSpace::MaxContent,
    //         },
    //         Some(&mut ctx),
    //     );
    //     assert_eq!(size.width, 512.0);
    //     assert_eq!(size.height, 128.0);
    // }

    // #[test]
    // fn image_measure_without_bitmap_returns_default_size() {
    //     let mut renderer = TextRenderer::new();
    //     let (nodes, mut ctx) = empty_image_nodes();
    //     let size = measure(
    //         &mut renderer,
    //         &nodes,
    //         taffy::Size {
    //             width: None,
    //             height: None,
    //         },
    //         taffy::Size {
    //             width: taffy::AvailableSpace::MaxContent,
    //             height: taffy::AvailableSpace::MaxContent,
    //         },
    //         Some(&mut ctx),
    //     );
    //     assert_eq!(size.width, 0.0);
    //     assert_eq!(size.height, 0.0);
    // }
}
