use vello::Scene;
use vello::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii};
use vello::peniko::Fill;

use crate::node::{ScrollAxis, UzNodeId};
use crate::style::{Bounds, ScrollbarStyle};

pub const SCROLLBAR_END_MARGIN: f64 = 0.0; // gap at track start/end (along scroll axis)
pub const THUMB_MIN_LENGTH: f64 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAlign {
    Start,
    Center,
    End,
    Nearest,
}

#[derive(Clone, Copy, Debug)]
pub struct ScrollIntoViewOptions {
    pub block: ScrollAlign,
    pub inline: ScrollAlign,
    pub margin: f32,
}

impl Default for ScrollIntoViewOptions {
    fn default() -> Self {
        Self {
            block: ScrollAlign::Nearest,
            inline: ScrollAlign::Nearest,
            margin: 8.0,
        }
    }
}

/// A scrollable extent on one axis. Geometry-only — the owning element's
/// `ScrollState` holds the actual offset.
#[derive(Clone, Copy, Debug)]
pub struct ScrollAxisInfo {
    pub content_size: f64,
    pub visible_size: f64,
    pub offset: f64,
}

impl ScrollAxisInfo {
    pub fn max_scroll(&self) -> f64 {
        (self.content_size - self.visible_size).max(0.0)
    }

    pub fn overflows(&self) -> bool {
        self.content_size > self.visible_size
    }

    pub fn clamped_offset(&self) -> f64 {
        self.offset.clamp(0.0, self.max_scroll())
    }
}

/// Vertical extent available for scrolling content when a horizontal scrollbar
/// is shown along the bottom edge (matches thumb placement in `thumb_geometry`).
#[inline]
pub fn vertical_scroll_visible_height(
    layout_height: f32,
    layout_content_width: f32,
    layout_size_width: f32,
    overflow_x_scrollable: bool,
    scrollbar_width: f32,
) -> f32 {
    let mut h = layout_height;
    if overflow_x_scrollable && layout_content_width > layout_size_width + 0.5 {
        let reserve = scrollbar_width.max(0.0) as f64;
        h = ((h as f64) - reserve).max(1.0) as f32;
    }
    h
}

pub fn compute_scroll_offset(
    rel: f32,
    target_extent: f32,
    viewport_extent: f32,
    content_extent: f32,
    cur_offset: f32,
    align: ScrollAlign,
    margin: f32,
) -> Option<f32> {
    if !rel.is_finite()
        || !target_extent.is_finite()
        || !viewport_extent.is_finite()
        || !content_extent.is_finite()
        || !cur_offset.is_finite()
    {
        return None;
    }
    let max_scroll = (content_extent - viewport_extent).max(0.0);
    let target_start = rel;
    let target_end = rel + target_extent;
    let next = match align {
        ScrollAlign::Start => target_start - margin,
        ScrollAlign::End => target_end - viewport_extent + margin,
        ScrollAlign::Center => target_start + target_extent / 2.0 - viewport_extent / 2.0,
        ScrollAlign::Nearest => {
            let inner_usable = (viewport_extent - 2.0 * margin).max(0.0);
            if target_extent > inner_usable && inner_usable > 0.0 {
                target_start - margin
            } else {
                let inner_start = cur_offset + margin;
                let inner_end = cur_offset + viewport_extent - margin;
                if target_start < inner_start {
                    target_start - margin
                } else if target_end > inner_end {
                    target_end - viewport_extent + margin
                } else {
                    cur_offset
                }
            }
        }
    };
    Some(next.clamp(0.0, max_scroll))
}

/// Local-space rect of a scrollbar thumb in the view's transform space, plus
/// the movable range along the drag axis (used by drag handlers) and the
/// track rect (used by paint to draw the optional track background).
#[derive(Clone, Copy, Debug)]
pub struct ThumbGeometry {
    pub local_x: f64,
    pub local_y: f64,
    pub thumb_width: f64,
    pub thumb_height: f64,
    pub track_range: f64,
    // unused for now
    pub track_x: f64,
    pub track_y: f64,
    pub track_width: f64,
    pub track_height: f64,
}

pub fn thumb_geometry(
    axis: ScrollAxis,
    view: Bounds,
    info: ScrollAxisInfo,
    style: &ScrollbarStyle,
) -> ThumbGeometry {
    let thickness = (style.width as f64).max(0.0);
    let max_scroll = info.max_scroll();
    let scroll_ratio = if max_scroll > 0.0 {
        info.clamped_offset() / max_scroll
    } else {
        0.0
    };
    match axis {
        ScrollAxis::Y => {
            let track = view.height - SCROLLBAR_END_MARGIN - thickness;
            let length =
                (track * info.visible_size / info.content_size.max(1.0)).max(THUMB_MIN_LENGTH);
            let track_range = (track - length).max(0.0);
            let track_x = view.width - thickness;
            ThumbGeometry {
                local_x: track_x,
                local_y: SCROLLBAR_END_MARGIN + scroll_ratio * track_range,
                thumb_width: thickness,
                thumb_height: length,
                track_range,
                track_x,
                track_y: SCROLLBAR_END_MARGIN,
                track_width: thickness,
                track_height: track,
            }
        }
        ScrollAxis::X => {
            let track = view.width - SCROLLBAR_END_MARGIN - thickness;
            let length =
                (track * info.visible_size / info.content_size.max(1.0)).max(THUMB_MIN_LENGTH);
            let track_range = (track - length).max(0.0);
            let track_y = view.height - thickness;
            ThumbGeometry {
                local_x: SCROLLBAR_END_MARGIN + scroll_ratio * track_range,
                local_y: track_y,
                thumb_width: length,
                thumb_height: thickness,
                track_range,
                track_x: SCROLLBAR_END_MARGIN,
                track_y,
                track_width: track,
                track_height: thickness,
            }
        }
    }
}

pub fn paint_thumb(
    scene: &mut Scene,
    transform: Affine,
    geom: &ThumbGeometry,
    hovered: bool,
    active: bool,
    style: &ScrollbarStyle,
) {
    if geom.thumb_width <= 0.0 || geom.thumb_height <= 0.0 {
        return;
    }

    let color = if active {
        style.active_color
    } else if hovered {
        style.hover_color
    } else {
        style.color
    };
    if color.is_transparent() {
        return;
    }

    let radius = style
        .radius
        .map(|r| r as f64)
        .unwrap_or_else(|| geom.thumb_width.min(geom.thumb_height) / 2.0)
        .max(0.0);
    let rect = Rect::new(
        geom.local_x,
        geom.local_y,
        geom.local_x + geom.thumb_width,
        geom.local_y + geom.thumb_height,
    );
    let rounded = RoundedRect::from_rect(rect, RoundedRectRadii::from_single_radius(radius));
    scene.fill(Fill::NonZero, transform, color.to_vello(), None, &rounded);
}

/// Rendered thumb rect, rebuilt each paint pass for hit testing.
pub struct ScrollThumbRect {
    pub node_id: UzNodeId,
    pub axis: ScrollAxis,
    pub thumb_bounds: Bounds,
    pub view_bounds: Bounds,
    pub content_size: f32,
    pub visible_size: f32,
}

#[cfg(test)]
mod tests {
    use super::{ScrollAlign, compute_scroll_offset};

    const M: f32 = 8.0;

    #[test]
    fn nearest_no_op_when_target_in_inner_band() {
        let off = compute_scroll_offset(20.0, 20.0, 100.0, 500.0, 0.0, ScrollAlign::Nearest, M);
        assert_eq!(off, Some(0.0));
    }

    #[test]
    fn nearest_scrolls_down_when_target_below_band() {
        let off = compute_scroll_offset(200.0, 20.0, 100.0, 500.0, 0.0, ScrollAlign::Nearest, M);
        assert_eq!(off, Some(128.0));
    }

    #[test]
    fn nearest_scrolls_up_when_target_above_band() {
        let off = compute_scroll_offset(50.0, 20.0, 100.0, 500.0, 200.0, ScrollAlign::Nearest, M);
        assert_eq!(off, Some(42.0));
    }

    #[test]
    fn target_taller_than_inner_usable_aligns_to_start() {
        let off = compute_scroll_offset(50.0, 200.0, 100.0, 500.0, 0.0, ScrollAlign::Nearest, M);
        assert_eq!(off, Some(42.0));
    }

    #[test]
    fn align_start_clamps_at_zero() {
        let off = compute_scroll_offset(5.0, 20.0, 100.0, 500.0, 0.0, ScrollAlign::Start, M);
        assert_eq!(off, Some(0.0));
    }

    #[test]
    fn align_end_clamps_to_max_scroll() {
        let off = compute_scroll_offset(180.0, 20.0, 100.0, 200.0, 0.0, ScrollAlign::End, M);
        assert_eq!(off, Some(100.0));
    }

    #[test]
    fn align_center_centers_target_in_viewport() {
        let off = compute_scroll_offset(200.0, 20.0, 100.0, 500.0, 0.0, ScrollAlign::Center, M);
        assert_eq!(off, Some(160.0));
    }

    #[test]
    fn no_overflow_returns_zero() {
        let off = compute_scroll_offset(5.0, 20.0, 100.0, 80.0, 0.0, ScrollAlign::End, M);
        assert_eq!(off, Some(0.0));
    }

    #[test]
    fn non_finite_inputs_return_none() {
        let off = compute_scroll_offset(f32::NAN, 20.0, 100.0, 500.0, 0.0, ScrollAlign::Nearest, M);
        assert_eq!(off, None);
        let off = compute_scroll_offset(
            20.0,
            20.0,
            f32::INFINITY,
            500.0,
            0.0,
            ScrollAlign::Nearest,
            M,
        );
        assert_eq!(off, None);
    }
}
