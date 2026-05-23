use parley::BoundingBox;
use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color as VelloColor, Fill};

use crate::input::input_align_offset;
use crate::style::{Bounds, TextStyle, UzStyle};
use crate::text::TextRenderer;

const SELECTION_COLOR: VelloColor = VelloColor::from_rgba8(56, 121, 185, 128);
const PLACEHOLDER_COLOR: VelloColor = VelloColor::from_rgba8(128, 128, 128, 255);
const PREEDIT_BG_COLOR: VelloColor = VelloColor::from_rgba8(50, 50, 60, 180);
const PREEDIT_UNDERLINE_COLOR: VelloColor = VelloColor::from_rgba8(180, 180, 180, 255);
const CARET_COLOR: VelloColor = VelloColor::from_rgba8(115, 115, 115, 255);
const CARET_WIDTH: f64 = 2.0;

pub struct InputRenderInfo {
    pub display_text: String,
    pub placeholder: String,
    pub text_style: TextStyle,
    pub focused: bool,
    pub cursor_rect: Option<BoundingBox>,
    pub selection_rects: Vec<BoundingBox>,
    pub scroll_offset_x: f32,
    pub scroll_offset_y: f32,
    pub blink_visible: bool,
    pub multiline: bool,
    pub layout_height: f32,
    pub preedit: Option<PreeditRenderInfo>,
}

pub struct PreeditRenderInfo {
    pub text: String,
    pub cursor_x: f32,
    pub width: f32,
}

/// Paint an input: background/border come from the standard `UzStyle::paint`
/// pipeline (same as a view), then the text/selection/caret are painted into
/// the content box, clipped against it.
pub fn paint_input(
    scene: &mut Scene,
    text_renderer: &mut TextRenderer,
    bounds: Bounds,
    style: &UzStyle,
    content_box: Bounds,
    info: &InputRenderInfo,
    transform: Affine,
) {
    style.paint(bounds, scene, transform, |scene| {
        InputPainter {
            scene,
            text_renderer,
            content_box,
            info,
            transform,
        }
        .paint();
    });
}

struct InputPainter<'a> {
    scene: &'a mut Scene,
    text_renderer: &'a mut TextRenderer,
    content_box: Bounds,
    info: &'a InputRenderInfo,
    transform: Affine,
}

// todo replace with point
#[derive(Clone, Copy)]
struct LayoutOrigin {
    x: f64,
    y: f64,
}

impl InputPainter<'_> {
    fn paint(mut self) {
        let content = self.content_box;
        if content.width <= 0.0 || content.height <= 0.0 {
            return;
        }

        self.scene
            .push_clip_layer(Fill::NonZero, self.transform, &content.to_rect());

        let origin = self.layout_origin(content);
        let is_empty = self.info.display_text.is_empty();

        if is_empty && !self.info.placeholder.is_empty() {
            self.paint_placeholder(content);
        }

        if !is_empty {
            if self.info.focused {
                self.paint_selection(origin);
            }
            self.paint_text(origin, content);
        }

        if let Some(preedit) = &self.info.preedit
            && let Some(cursor) = &self.info.cursor_rect
        {
            self.paint_preedit(origin, preedit, cursor);
        }

        if self.should_paint_caret()
            && let Some(cursor) = &self.info.cursor_rect
        {
            self.paint_caret(origin, cursor);
        }

        self.scene.pop_layer();
    }

    fn line_height(&self) -> f32 {
        (self.info.text_style.font_size * self.info.text_style.line_height).round()
    }

    fn layout_origin(&mut self, content: Bounds) -> LayoutOrigin {
        if self.info.multiline {
            return LayoutOrigin {
                x: content.x,
                y: content.y - self.info.scroll_offset_y as f64,
            };
        }

        let line_h = self.line_height() as f64;
        let y = content.y + ((content.height - line_h) * 0.5).max(0.0);

        let (natural_w, _) = self.text_renderer.measure_text(
            &self.info.display_text,
            &self.info.text_style,
            None,
            None,
        );
        let align = input_align_offset(
            content.width as f32,
            natural_w,
            self.info.text_style.text_align,
        ) as f64;
        let x = content.x + align - self.info.scroll_offset_x as f64;

        LayoutOrigin { x, y }
    }

    fn paint_placeholder(&mut self, content: Bounds) {
        let line_h = self.line_height();
        let py = if self.info.multiline {
            content.y as f32
        } else {
            content.y as f32 + ((content.height as f32 - line_h) * 0.5).max(0.0)
        };
        // Placeholder respects text-align: pass the content width so parley's
        // alignment is applied in single-line and multiline alike.
        self.text_renderer.draw_text(
            self.scene,
            &self.info.placeholder,
            &self.info.text_style,
            Some(content.width as f32),
            (content.x as f32, py),
            PLACEHOLDER_COLOR,
            self.transform,
        );
    }

    fn paint_selection(&mut self, origin: LayoutOrigin) {
        for rect in &self.info.selection_rects {
            let r = Rect::new(
                origin.x + rect.x0,
                origin.y + rect.y0,
                origin.x + rect.x1,
                origin.y + rect.y1,
            );
            self.scene
                .fill(Fill::NonZero, self.transform, SELECTION_COLOR, None, &r);
        }
    }

    fn paint_text(&mut self, origin: LayoutOrigin, content: Bounds) {
        // Multiline keeps the editor's wrap width so alignment applies inside
        // the layout. Single-line draws with no wrap so the layout grows
        // naturally and we position via `origin.x`.
        let wrap = if self.info.multiline {
            Some(content.width as f32)
        } else {
            None
        };
        self.text_renderer.draw_text(
            self.scene,
            &self.info.display_text,
            &self.info.text_style,
            wrap,
            (origin.x as f32, origin.y as f32),
            self.info.text_style.color.to_vello(),
            self.transform,
        );
    }

    fn paint_preedit(
        &mut self,
        origin: LayoutOrigin,
        preedit: &PreeditRenderInfo,
        cursor: &BoundingBox,
    ) {
        let px = origin.x + cursor.x0;
        let py = origin.y + cursor.y0;
        let height = cursor.y1 - cursor.y0;
        let width = preedit.width as f64;

        let bg = Rect::new(px, py, px + width, py + height);
        self.scene
            .fill(Fill::NonZero, self.transform, PREEDIT_BG_COLOR, None, &bg);

        self.text_renderer.draw_text(
            self.scene,
            &preedit.text,
            &self.info.text_style,
            None,
            (px as f32, py as f32),
            self.info.text_style.color.to_vello(),
            self.transform,
        );

        let underline_y = py + height - 1.0;
        let underline = Rect::new(px, underline_y, px + width, underline_y + 1.0);
        self.scene.fill(
            Fill::NonZero,
            self.transform,
            PREEDIT_UNDERLINE_COLOR,
            None,
            &underline,
        );
    }

    fn paint_caret(&mut self, origin: LayoutOrigin, cursor: &BoundingBox) {
        let cx = origin.x + cursor.x0;
        let cy = origin.y + cursor.y0;
        let rect = Rect::new(cx, cy, cx + CARET_WIDTH, cy + (cursor.y1 - cursor.y0));
        self.scene
            .fill(Fill::NonZero, self.transform, CARET_COLOR, None, &rect);
    }

    fn should_paint_caret(&self) -> bool {
        self.info.focused && self.info.blink_visible && self.info.preedit.is_none()
    }
}
