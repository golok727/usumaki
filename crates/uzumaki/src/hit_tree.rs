//! Hit-tree construction. Walks the layout tree from the current
//! `final_layout` + `scroll_state` and rebuilds the cache of node
//! hitboxes and scroll-thumb rects used by input dispatch.

use vello::kurbo::Affine;

use crate::interactivity::HitboxStore;
use crate::node::{ScrollAxis, UzNodeId};
use crate::paint::ScrollThumbRect;
use crate::paint::scroll::{self, ScrollAxisInfo};
use crate::style::{Bounds, Visibility};
use crate::text::{TextRenderer, apply_text_style_to_editor};
use crate::ui::UIState;

/// Recompute the hit tree from current state. Cheap relative to paint
/// (no scene work, no glyph layout) and idempotent. Safe to call
/// multiple times per frame.
pub fn rebuild(state: &mut UIState, text_renderer: &mut TextRenderer, _scale: f64) {
    state.hitbox_store.clear();
    state.scroll_thumbs.clear();
    for (_, node) in state.nodes.iter_mut() {
        node.hitbox_id = None;
    }

    let Some(root) = state.root else {
        state.hit_tree_dirty = false;
        return;
    };
    // Mouse coordinates are converted to logical pixels before hit-test,
    // so the hit transform starts at IDENTITY — never `Affine::scale`.
    // (Paint uses `Affine::scale(scale)` for the *paint* transform, which
    // is a separate axis.)
    walk(state, text_renderer, root, Affine::IDENTITY, None);
    state.hit_tree_dirty = false;
}

fn walk(
    state: &mut UIState,
    text_renderer: &mut TextRenderer,
    node_id: UzNodeId,
    parent_hit_transform: Affine,
    clip: Option<Bounds>,
) {
    let Some(node) = state.nodes.get(node_id) else {
        return;
    };
    let computed_style = node.computed_style().clone();

    if computed_style.visibility == Visibility::Hidden
        || computed_style.display == crate::style::Display::None
    {
        return;
    }

    let layout = node.final_layout;
    let border_box = Bounds::new(
        0.0,
        0.0,
        layout.size.width as f64,
        layout.size.height as f64,
    );
    let local_style_transform = computed_style
        .transform
        .to_affine(border_box.width, border_box.height);
    let local_translate = Affine::translate((layout.location.x as f64, layout.location.y as f64));
    let hit_transform = parent_hit_transform * local_translate * local_style_transform;

    // Screen-space AABB of this node. Used to cull this hitbox
    // against the ancestor scroll/clip region and extend the clip
    // for descendants if this node itself clips.
    let screen_aabb = transformed_view_bounds(border_box, hit_transform);
    let visible_aabb = match clip {
        Some(c) => intersect(screen_aabb, c),
        None => Some(screen_aabb),
    };

    if visible_aabb.is_none() {
        return;
    }

    let hitbox_id = state
        .hitbox_store
        .insert_transformed(node_id, border_box, hit_transform);
    state.nodes[node_id].hitbox_id = Some(hitbox_id);

    if state.nodes[node_id].is_text_input() {
        register_input_scrollbar(state, text_renderer, node_id, hit_transform);
        return;
    }

    let view_bounds = transformed_view_bounds(border_box, hit_transform);
    let (offset_x, offset_y) = register_view_scroll(state, node_id, &computed_style, view_bounds);

    let scroll_translate = if offset_x != 0.0 || offset_y != 0.0 {
        Affine::translate((-offset_x, -offset_y))
    } else {
        Affine::IDENTITY
    };
    let child_hit_transform = hit_transform * scroll_translate;

    // Extend the clip for descendants if this node clips its content.
    // `overflow: visible` lets children spill out and remain hittable.
    let child_clip = if computed_style.overflow_x.clips()
        || computed_style.overflow_y.clips()
        || computed_style.overflow_x.is_scrollable()
        || computed_style.overflow_y.is_scrollable()
    {
        let content_local = crate::layout::TaffyLayoutExt::content_box_bounds(&layout);
        let content_screen = transformed_view_bounds(content_local, hit_transform);
        Some(match clip {
            Some(c) => intersect(content_screen, c).unwrap_or(content_screen),
            None => content_screen,
        })
    } else {
        clip
    };

    let children = state.nodes[node_id].layout_children.borrow().clone();
    for child_id in children {
        walk(
            state,
            text_renderer,
            child_id,
            child_hit_transform,
            child_clip,
        );
    }
}

fn intersect(a: Bounds, b: Bounds) -> Option<Bounds> {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = (a.x + a.width).min(b.x + b.width);
    let y1 = (a.y + a.height).min(b.y + b.height);
    if x1 <= x0 || y1 <= y0 {
        None
    } else {
        Some(Bounds::new(x0, y0, x1 - x0, y1 - y0))
    }
}

/// Compute the hitbox's screen-space AABB.
fn transformed_view_bounds(local: Bounds, transform: Affine) -> Bounds {
    use vello::kurbo::Point;
    let pts = [
        transform * Point::new(local.x, local.y),
        transform * Point::new(local.x + local.width, local.y),
        transform * Point::new(local.x + local.width, local.y + local.height),
        transform * Point::new(local.x, local.y + local.height),
    ];
    let (mut min_x, mut min_y) = (pts[0].x, pts[0].y);
    let (mut max_x, mut max_y) = (pts[0].x, pts[0].y);
    for p in pts.iter().skip(1) {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    Bounds::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Clamp the node's scroll offsets to its current overflow and register
/// scroll-thumb hit rects for both axes if scrollable. Returns the
/// (clamped) offsets so the walker can apply them to the child
/// transform.
fn register_view_scroll(
    state: &mut UIState,
    node_id: UzNodeId,
    style: &crate::style::UzStyle,
    view_bounds: Bounds,
) -> (f64, f64) {
    let scroll_x = style.overflow_x.is_scrollable();
    let scroll_y = style.overflow_y.is_scrollable();
    if !scroll_x && !scroll_y {
        return (0.0, 0.0);
    }

    let layout = state.nodes[node_id].final_layout;
    let (shows_x, shows_y) = visible_scrollbars(style, &layout);
    let content_box = scroll_content_box(&layout, style, shows_x, shows_y);
    let visible_w = content_box.width;
    let visible_h = content_box.height;
    let content_w =
        crate::layout::TaffyLayoutExt::axis_scroll_content_size(&layout, ScrollAxis::X) as f64;
    let content_h =
        crate::layout::TaffyLayoutExt::axis_scroll_content_size(&layout, ScrollAxis::Y) as f64;
    let max_x = (content_w - visible_w).max(0.0);
    let max_y = (content_h - visible_h).max(0.0);

    let ss = &mut state.nodes[node_id].scroll_state;
    if ss.scroll_offset_x as f64 > max_x {
        ss.scroll_offset_x = max_x as f32;
    }
    if ss.scroll_offset_y as f64 > max_y {
        ss.scroll_offset_y = max_y as f32;
    }
    let offset_x = ss.scroll_offset_x as f64;
    let offset_y = ss.scroll_offset_y as f64;

    if shows_y {
        register_view_thumb(
            &mut state.hitbox_store,
            &mut state.scroll_thumbs,
            node_id,
            ScrollAxis::Y,
            view_bounds,
            content_h,
            visible_h,
            offset_y,
            &style.scrollbar,
        );
    }
    if shows_x {
        register_view_thumb(
            &mut state.hitbox_store,
            &mut state.scroll_thumbs,
            node_id,
            ScrollAxis::X,
            view_bounds,
            content_w,
            visible_w,
            offset_x,
            &style.scrollbar,
        );
    }

    (offset_x, offset_y)
}

#[allow(clippy::too_many_arguments)]
fn register_view_thumb(
    _hitbox_store: &mut HitboxStore,
    scroll_thumbs: &mut Vec<ScrollThumbRect>,
    node_id: UzNodeId,
    axis: ScrollAxis,
    view_bounds: Bounds,
    content: f64,
    visible: f64,
    offset: f64,
    scrollbar: &crate::style::ScrollbarStyle,
) {
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
    scroll_thumbs.push(ScrollThumbRect {
        node_id,
        axis,
        thumb_bounds,
        view_bounds,
        content_size: content as f32,
        visible_size: visible as f32,
    });
}

/// Multi-line text inputs have their own internal scroll. Refresh the
/// editor's parley layout so we can compute current overflow, then clamp
/// and register the thumb. Single-line inputs never overflow vertically.
fn register_input_scrollbar(
    state: &mut UIState,
    text_renderer: &mut TextRenderer,
    node_id: UzNodeId,
    hit_transform: Affine,
) {
    let layout = state.nodes[node_id].final_layout;
    let content_box = crate::layout::TaffyLayoutExt::content_box_bounds(&layout);
    let style = state.nodes[node_id].computed_style().clone();

    let (multiline, layout_height) = {
        let node = state.nodes.get_mut(node_id).unwrap();
        let is = node.as_text_input_mut().unwrap();
        let multiline = is.multiline;
        let text_w = layout.content_box_width().max(0.0);
        apply_text_style_to_editor(&mut is.editor, &style.text);
        is.editor
            .set_width(if multiline { Some(text_w) } else { None });
        is.editor
            .refresh_layout(&mut text_renderer.font_ctx, &mut text_renderer.layout_ctx);
        let h = is.editor.try_layout().map(|l| l.height()).unwrap_or(0.0);
        (multiline, h)
    };

    if !multiline {
        return;
    }

    let axis_info = ScrollAxisInfo {
        content_size: layout_height as f64,
        visible_size: content_box.height,
        offset: state.nodes[node_id].scroll_state.scroll_offset_y as f64,
    };
    if !axis_info.overflows() {
        return;
    }

    let max_y = axis_info.max_scroll() as f32;
    let ss = &mut state.nodes[node_id].scroll_state;
    if ss.scroll_offset_y > max_y {
        ss.scroll_offset_y = max_y;
    }

    let view_bounds = transformed_view_bounds(
        Bounds::new(
            0.0,
            0.0,
            layout.size.width as f64,
            layout.size.height as f64,
        ),
        hit_transform,
    );
    register_view_thumb(
        &mut state.hitbox_store,
        &mut state.scroll_thumbs,
        node_id,
        ScrollAxis::Y,
        view_bounds,
        axis_info.content_size,
        axis_info.visible_size,
        axis_info.offset,
        &style.scrollbar,
    );
}

fn visible_scrollbars(style: &crate::style::UzStyle, layout: &taffy::Layout) -> (bool, bool) {
    let content_w =
        crate::layout::TaffyLayoutExt::axis_scroll_content_size(layout, ScrollAxis::X) as f64;
    let content_h =
        crate::layout::TaffyLayoutExt::axis_scroll_content_size(layout, ScrollAxis::Y) as f64;
    let visible_w =
        crate::layout::TaffyLayoutExt::axis_content_box_size(layout, ScrollAxis::X) as f64;
    let visible_h =
        crate::layout::TaffyLayoutExt::axis_content_box_size(layout, ScrollAxis::Y) as f64;
    (
        scrollbar_visible(style.overflow_x, content_w, visible_w),
        scrollbar_visible(style.overflow_y, content_h, visible_h),
    )
}

fn scrollbar_visible(overflow: crate::style::Overflow, content: f64, visible: f64) -> bool {
    use crate::style::Overflow;
    match overflow {
        Overflow::Scroll => true,
        Overflow::Auto => content > visible + 0.5,
        Overflow::Visible | Overflow::Hidden => false,
    }
}

fn scroll_content_box(
    layout: &taffy::Layout,
    style: &crate::style::UzStyle,
    shows_x: bool,
    shows_y: bool,
) -> Bounds {
    let mut content_box = crate::layout::TaffyLayoutExt::content_box_bounds(layout);
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
    overflow: crate::style::Overflow,
    reserved_gutter: f32,
    visible: bool,
    gutter: f64,
) -> f64 {
    use crate::style::Overflow;
    if overflow == Overflow::Auto && visible {
        (gutter - reserved_gutter as f64).max(0.0)
    } else {
        0.0
    }
}
