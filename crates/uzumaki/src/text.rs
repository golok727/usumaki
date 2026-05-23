use parley::{
    Affinity as ParleyAffinity, Alignment as ParleyAlignment, AlignmentOptions, BoundingBox,
    Cluster, Cursor, FontContext, Layout, LayoutContext, Selection,
};
use vello::Scene;
use vello::kurbo::Affine;
use vello::peniko::{Brush, Color, Fill};

use crate::style::TextStyle;

pub(crate) use parley::{InlineBox, TreeBuilder};

impl From<crate::selection::Affinity> for ParleyAffinity {
    fn from(value: crate::selection::Affinity) -> Self {
        match value {
            crate::selection::Affinity::Downstream => Self::Downstream,
            crate::selection::Affinity::Upstream => Self::Upstream,
        }
    }
}

impl From<ParleyAffinity> for crate::selection::Affinity {
    fn from(value: ParleyAffinity) -> Self {
        match value {
            ParleyAffinity::Downstream => Self::Downstream,
            ParleyAffinity::Upstream => Self::Upstream,
        }
    }
}

/// Brush id used by `compute_text_leaf` for its standalone parley layout.
/// Sentinel meaning "no inline `<text>` element owns this run" — paint
/// code uses it to distinguish a text leaf's own glyphs from a real
/// inline span that should receive per-line chip painting.
pub const LEAF_BRUSH_ID: usize = usize::MAX;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextBrush {
    pub id: usize,
}

impl TextBrush {
    pub fn from_id(id: usize) -> Self {
        Self { id }
    }

    pub fn is_leaf(self) -> bool {
        self.id == LEAF_BRUSH_ID
    }
}

pub struct TextRenderer {
    pub font_ctx: FontContext,
    pub layout_ctx: LayoutContext<TextBrush>,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRenderer {
    pub fn new() -> Self {
        let font_ctx = FontContext::default();

        // todo allow registering custom fonts

        Self {
            font_ctx,
            layout_ctx: LayoutContext::new(),
        }
    }

    pub(crate) fn build_layout(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
    ) -> Layout<TextBrush> {
        let mut builder = self
            .layout_ctx
            .ranged_builder(&mut self.font_ctx, text, 1.0, true);
        for prop in style.to_parley_styles() {
            builder.push_default(prop);
        }
        let mut layout = builder.build(text);
        layout.break_all_lines(max_width);
        let alignment = style.text_align;
        // Only apply alignment when a frame is provided. Without one, callers
        // (cursor x-positions, hit testing, single-line input drawing) want
        // natural unaligned coordinates and apply alignment themselves.
        if alignment != ParleyAlignment::Start && max_width.is_some() {
            layout.align(max_width, alignment, AlignmentOptions::default());
        }
        layout
    }

    /// Build a multi-span inline parley layout. The caller drives the parley
    /// `TreeBuilder` via `build`, choosing which fragments to push (styled
    /// spans, inline boxes, padding spacers, etc.).
    pub(crate) fn build_inline_layout<F>(
        &mut self,
        root_style: &TextStyle,
        root_node_id: usize,
        max_width: Option<f32>,
        build: F,
    ) -> Layout<TextBrush>
    where
        F: FnOnce(&mut TreeBuilder<'_, TextBrush>),
    {
        let parley_root = root_style.to_parley_text_style(root_node_id);
        let mut builder = self
            .layout_ctx
            .tree_builder(&mut self.font_ctx, 1.0, true, &parley_root);
        build(&mut builder);
        let (mut layout, _) = builder.build();
        layout.break_all_lines(max_width);
        let alignment = root_style.text_align;
        if alignment != ParleyAlignment::Start && max_width.is_some() {
            layout.align(max_width, alignment, AlignmentOptions::default());
        }
        layout
    }

    fn grapheme_to_byte(boundaries: &[usize], grapheme_index: usize) -> usize {
        boundaries
            .get(grapheme_index)
            .copied()
            .unwrap_or_else(|| *boundaries.last().unwrap_or(&0))
    }

    fn byte_to_grapheme(boundaries: &[usize], byte_index: usize) -> usize {
        boundaries
            .partition_point(|&boundary| boundary <= byte_index)
            .saturating_sub(1)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        &mut self,
        scene: &mut Scene,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
        position: (f32, f32),
        color: Color,
        transform: Affine,
    ) {
        let layout = self.build_layout(text, style, width);
        draw_layout(scene, &layout, position, color, transform);
    }

    pub fn grapheme_x_positions(&mut self, text: &str, style: &TextStyle) -> Vec<f32> {
        if text.is_empty() {
            return vec![0.0];
        }

        let layout = self.build_layout(text, style, None);
        let layout_width = layout.width();
        let boundaries = cluster_byte_boundaries(&layout);

        let mut positions = Vec::with_capacity(boundaries.len());
        positions.push(0.0);
        for &byte_offset in boundaries.iter().skip(1) {
            let cursor = Cursor::from_byte_index(&layout, byte_offset, ParleyAffinity::Downstream);
            let geom = cursor.geometry(&layout, layout_width);
            positions.push(geom.x0 as f32);
        }

        positions
    }

    pub fn hit_to_grapheme(&mut self, text: &str, style: &TextStyle, x: f32) -> usize {
        self.hit_to_grapheme_2d(text, style, None, x, 0.0)
    }

    pub fn hit_to_grapheme_2d(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> usize {
        if text.is_empty() {
            return 0;
        }

        let layout = self.build_layout(text, style, wrap_width);
        let boundaries = cluster_byte_boundaries(&layout);
        let cursor = Cursor::from_point(&layout, x, y);
        Self::byte_to_grapheme(&boundaries, cursor.index())
    }

    pub fn hit_to_text_position(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, crate::selection::Affinity) {
        if text.is_empty() {
            return (0, crate::selection::Affinity::Downstream);
        }

        let layout = self.build_layout(text, style, wrap_width);
        let cursor = Cursor::from_point(&layout, x, y);
        (cursor.index().min(text.len()), cursor.affinity().into())
    }

    pub fn cursor_geometry(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        grapheme_index: usize,
    ) -> BoundingBox {
        let layout = self.build_layout(text, style, wrap_width);
        let boundaries = cluster_byte_boundaries(&layout);
        let byte_index = Self::grapheme_to_byte(&boundaries, grapheme_index);
        let cursor = Cursor::from_byte_index(&layout, byte_index, ParleyAffinity::Downstream);
        cursor.geometry(&layout, layout.width())
    }

    pub fn word_range_at_point(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let layout = self.build_layout(text, style, wrap_width);
        let boundaries = cluster_byte_boundaries(&layout);
        let selection = Selection::word_from_point(&layout, x, y);
        let range = selection.text_range();
        (
            Self::byte_to_grapheme(&boundaries, range.start),
            Self::byte_to_grapheme(&boundaries, range.end),
        )
    }

    pub fn word_byte_range_at_point(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let layout = self.build_layout(text, style, wrap_width);
        let range = Selection::word_from_point(&layout, x, y).text_range();
        (range.start.min(text.len()), range.end.min(text.len()))
    }

    pub fn line_range_at_point(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let layout = self.build_layout(text, style, wrap_width);
        let boundaries = cluster_byte_boundaries(&layout);
        let selection = Selection::line_from_point(&layout, x, y);
        let range = selection.text_range();
        (
            Self::byte_to_grapheme(&boundaries, range.start),
            Self::byte_to_grapheme(&boundaries, range.end),
        )
    }

    pub fn line_byte_range_at_point(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let layout = self.build_layout(text, style, wrap_width);
        let range = Selection::line_from_point(&layout, x, y).text_range();
        (range.start.min(text.len()), range.end.min(text.len()))
    }

    pub fn selection_rects(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        start: usize,
        end: usize,
    ) -> Vec<BoundingBox> {
        if text.is_empty() || start >= end {
            return Vec::new();
        }

        let layout = self.build_layout(text, style, wrap_width);
        let anchor = start.min(text.len());
        let focus = end.min(text.len());
        let selection = Selection::new(
            Cursor::from_byte_index(&layout, anchor, ParleyAffinity::Downstream),
            Cursor::from_byte_index(&layout, focus, ParleyAffinity::Upstream),
        );

        selection
            .geometry(&layout)
            .into_iter()
            .map(|(rect, _)| rect)
            .collect()
    }

    pub fn measure_text(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
        _max_height: Option<f32>,
    ) -> (f32, f32) {
        let layout = self.build_layout(text, style, max_width);

        let measured_width = layout.full_width();
        let measured_height = layout.height();
        let fallback_height = (style.font_size * style.line_height).round();

        let w = if measured_width == 0.0 {
            (text.len() as f32) * (style.font_size * 0.6)
        } else {
            measured_width
        };

        let h = if measured_height == 0.0 {
            fallback_height
        } else {
            measured_height
        };

        (w.ceil(), h.ceil())
    }
}

/// Byte offsets at parley cluster boundaries for the given layout. Always
/// starts with `0`; for a non-empty layout the last entry is the text length.
pub fn cluster_byte_boundaries(layout: &Layout<TextBrush>) -> Vec<usize> {
    let mut out = vec![0];
    let mut cur = Cluster::from_byte_index(layout, 0);
    while let Some(c) = cur {
        out.push(c.text_range().end);
        cur = c.next_logical();
    }
    out
}

/// Number of parley clusters (graphemes) in the layout.
pub fn cluster_count(layout: &Layout<TextBrush>) -> usize {
    let mut count = 0;
    let mut cur = Cluster::from_byte_index(layout, 0);
    while let Some(c) = cur {
        count += 1;
        cur = c.next_logical();
    }
    count
}

/// Convert a grapheme index into a byte offset within the layout's source
/// text. Out-of-range indices clamp to the end.
pub fn layout_grapheme_to_byte(layout: &Layout<TextBrush>, grapheme_index: usize) -> usize {
    let Some(mut cur) = Cluster::from_byte_index(layout, 0) else {
        return 0;
    };
    for _ in 0..grapheme_index {
        match cur.next_logical() {
            Some(next) => cur = next,
            None => return cur.text_range().end,
        }
    }
    cur.text_range().start
}

/// Convert a byte offset within the layout's source text into a grapheme
/// index. Offsets in the middle of a cluster round down to that cluster's
/// index.
pub fn layout_byte_to_grapheme(layout: &Layout<TextBrush>, byte: usize) -> usize {
    let mut count = 0;
    let mut cur = Cluster::from_byte_index(layout, 0);
    while let Some(c) = cur {
        if c.text_range().start >= byte {
            break;
        }
        count += 1;
        cur = c.next_logical();
    }
    count
}

/// Draw a parley layout that has already been built. Pure draw with no layout
/// rebuild. Used by the painter against an element's cached inline layout.
pub fn draw_layout(
    scene: &mut Scene,
    layout: &Layout<TextBrush>,
    position: (f32, f32),
    color: Color,
    transform: Affine,
) {
    draw_layout_with_brush(scene, layout, position, transform, |_| color);
}

pub fn draw_layout_with_brush(
    scene: &mut Scene,
    layout: &Layout<TextBrush>,
    position: (f32, f32),
    transform: Affine,
    mut color_for_brush: impl FnMut(TextBrush) -> Color,
) {
    let (px, py) = position;
    for line in layout.lines() {
        for item in line.items() {
            if let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                let run = glyph_run.run();
                let font = run.font().clone();
                let run_font_size = run.font_size();
                let synthesis = run.synthesis();
                let glyph_xform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));

                scene
                    .draw_glyphs(&font)
                    .font_size(run_font_size)
                    .transform(transform)
                    .glyph_transform(glyph_xform)
                    .brush(&Brush::Solid(color_for_brush(glyph_run.style().brush)))
                    .draw(
                        Fill::NonZero,
                        glyph_run.positioned_glyphs().map(|g| vello::Glyph {
                            id: g.id,
                            x: px + g.x,
                            y: py + g.y,
                        }),
                    );
            }
        }
    }
}

/// Map a point (in layout-local coordinates) to a `(byte_offset, affinity)`
/// against a prebuilt parley layout.
pub fn hit_to_text_position_from_layout(
    layout: &Layout<TextBrush>,
    text_len: usize,
    x: f32,
    y: f32,
) -> (usize, crate::selection::Affinity) {
    if text_len == 0 {
        return (0, crate::selection::Affinity::Downstream);
    }
    let cursor = Cursor::from_point(layout, x, y);
    (cursor.index().min(text_len), cursor.affinity().into())
}

/// Word-range byte offsets at a point against a prebuilt parley layout.
pub fn word_byte_range_at_point_from_layout(
    layout: &Layout<TextBrush>,
    text_len: usize,
    x: f32,
    y: f32,
) -> (usize, usize) {
    if text_len == 0 {
        return (0, 0);
    }
    let range = Selection::word_from_point(layout, x, y).text_range();
    (range.start.min(text_len), range.end.min(text_len))
}

/// Line-range byte offsets at a point against a prebuilt parley layout.
pub fn line_byte_range_at_point_from_layout(
    layout: &Layout<TextBrush>,
    text_len: usize,
    x: f32,
    y: f32,
) -> (usize, usize) {
    if text_len == 0 {
        return (0, 0);
    }
    let range = Selection::line_from_point(layout, x, y).text_range();
    (range.start.min(text_len), range.end.min(text_len))
}

/// Selection rects from a prebuilt parley layout. `start`/`end` are byte
/// offsets into the layout's source text.
pub fn selection_rects_from_layout(
    layout: &Layout<TextBrush>,
    text_len: usize,
    start: usize,
    end: usize,
) -> Vec<BoundingBox> {
    if start >= end {
        return Vec::new();
    }
    let anchor = start.min(text_len);
    let focus = end.min(text_len);
    let selection = Selection::new(
        Cursor::from_byte_index(layout, anchor, ParleyAffinity::Downstream),
        Cursor::from_byte_index(layout, focus, ParleyAffinity::Upstream),
    );
    selection
        .geometry(layout)
        .into_iter()
        .map(|(rect, _)| rect)
        .collect()
}

pub fn apply_text_style_to_editor(editor: &mut parley::PlainEditor<TextBrush>, style: &TextStyle) {
    let styles = editor.edit_styles();
    for prop in style.to_parley_styles() {
        styles.insert(prop);
    }
    editor.set_alignment(style.text_align);
}

pub fn secure_cursor_geometry(
    editor: &parley::PlainEditor<TextBrush>,
    width: f32,
    style: &TextStyle,
    text_renderer: &mut TextRenderer,
) -> Option<BoundingBox> {
    let sel = editor.raw_selection();
    let byte_idx = sel.focus().index();
    let char_count = editor.raw_text()[..byte_idx].chars().count();
    let masked = "\u{2022}".repeat(char_count);
    let positions = text_renderer.grapheme_x_positions(&masked, style);
    let cursor_x = *positions.last().unwrap_or(&0.0);
    let line_height = (style.font_size * style.line_height).round();
    Some(BoundingBox {
        x0: cursor_x as f64,
        y0: 0.0,
        x1: (cursor_x + width) as f64,
        y1: line_height as f64,
    })
}

pub fn secure_selection_geometry(
    editor: &parley::PlainEditor<TextBrush>,
    style: &TextStyle,
    text_renderer: &mut TextRenderer,
) -> Vec<BoundingBox> {
    let sel = editor.raw_selection();
    if sel.is_collapsed() {
        return vec![];
    }
    let text = editor.raw_text();
    let anchor_chars = text[..sel.anchor().index()].chars().count();
    let focus_chars = text[..sel.focus().index()].chars().count();
    let (start, end) = if anchor_chars < focus_chars {
        (anchor_chars, focus_chars)
    } else {
        (focus_chars, anchor_chars)
    };
    let full_masked = "\u{2022}".repeat(end);
    let positions = text_renderer.grapheme_x_positions(&full_masked, style);
    let x0 = if start < positions.len() {
        positions[start]
    } else {
        0.0
    };
    let x1 = if end < positions.len() {
        positions[end]
    } else {
        *positions.last().unwrap_or(&0.0)
    };
    let line_height = (style.font_size * style.line_height).round();
    vec![BoundingBox {
        x0: x0 as f64,
        y0: 0.0,
        x1: x1 as f64,
        y1: line_height as f64,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer() -> TextRenderer {
        TextRenderer::new()
    }

    fn default_style() -> TextStyle {
        TextStyle::default()
    }

    #[test]
    fn hit_2d_start_of_text() {
        let mut r = renderer();
        let idx = r.hit_to_grapheme_2d("abc\ndef", &default_style(), None, 0.0, 0.0);
        assert_eq!(idx, 0);
    }

    #[test]
    fn hit_2d_second_line() {
        let mut r = renderer();
        let idx = r.hit_to_grapheme_2d("abc\ndef", &default_style(), None, 0.0, 24.0);
        assert_eq!(
            idx, 4,
            "clicking at start of line 1 should give index 4 (after \\n)"
        );
    }

    #[test]
    fn hit_2d_past_end_snaps_to_last() {
        let mut r = renderer();
        let style = default_style();
        let pos = r.grapheme_x_positions("abc", &style);
        let last_x = *pos.last().unwrap();

        let idx = r.hit_to_grapheme_2d("abc", &style, None, last_x + 100.0, 0.0);
        assert_eq!(idx, 3, "clicking past end should give last position");
    }

    #[test]
    fn x_positions_count() {
        let mut r = renderer();
        let pos = r.grapheme_x_positions("hello", &default_style());
        assert_eq!(pos.len(), 6, "5 graphemes + 1 = 6 boundaries");
    }

    #[test]
    fn x_positions_start_at_zero() {
        let mut r = renderer();
        let pos = r.grapheme_x_positions("abc", &default_style());
        assert!((pos[0] - 0.0).abs() < 0.01, "first position should be 0");
    }

    #[test]
    fn x_positions_monotonic() {
        let mut r = renderer();
        let pos = r.grapheme_x_positions("hello world", &default_style());
        for w in pos.windows(2) {
            assert!(
                w[1] >= w[0] - 0.01,
                "x should increase: {} >= {}",
                w[1],
                w[0]
            );
        }
    }

    #[test]
    fn word_range_uses_layout_boundaries() {
        let mut r = renderer();
        let (start, end) = r.word_range_at_point("hello world", &default_style(), None, 2.0, 0.0);
        assert_eq!((start, end), (0, 5));
    }

    #[test]
    fn line_range_tracks_visual_line() {
        let mut r = renderer();
        let (start, end) = r.line_range_at_point("abc\ndef", &default_style(), None, 0.0, 24.0);
        assert_eq!((start, end), (4, 7));
    }

    #[test]
    fn selection_rects_split_across_lines() {
        let mut r = renderer();
        let rects = r.selection_rects("ab\ncd", &default_style(), None, 1, 4);
        assert_eq!(rects.len(), 2);
        assert!(rects[0].x1 > rects[0].x0);
        assert!(rects[1].y0 > rects[0].y0);
    }
}
