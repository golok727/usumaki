use parley::{FontFamily, LineHeight, StyleProperty};
use refineable::Refineable;
use vello::Scene;
use vello::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Stroke};
use vello::peniko::Color as VelloColor;

use crate::SharedString;
use crate::cursor::UzCursorIcon;
use crate::text::TextBrush;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn to_vello(self) -> VelloColor {
        VelloColor::from_rgba8(self.r, self.g, self.b, self.a)
    }

    pub fn with_opacity(self, opacity: f32) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: (self.a as f32 * opacity.clamp(0.0, 1.0)) as u8,
        }
    }

    pub fn is_transparent(self) -> bool {
        self.a == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bounds {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
    }

    pub fn to_rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.x + self.width, self.y + self.height)
    }

    /// Inset by an `Edges` (e.g. `style.padding`) to produce the inner box.
    /// Width/height clamp at zero.
    pub fn inset_by(&self, edges: &Edges) -> Bounds {
        Bounds::new(
            self.x + edges.left as f64,
            self.y + edges.top as f64,
            (self.width - edges.horizontal() as f64).max(0.0),
            (self.height - edges.vertical() as f64).max(0.0),
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub fn all(val: f32) -> Self {
        Self {
            top: val,
            right: val,
            bottom: val,
            left: val,
        }
    }

    pub fn any_nonzero(&self) -> bool {
        self.top > 0.0 || self.right > 0.0 || self.bottom > 0.0 || self.left > 0.0
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct Inset {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Corners {
    pub fn uniform(val: f32) -> Self {
        Self {
            top_left: val,
            top_right: val,
            bottom_right: val,
            bottom_left: val,
        }
    }

    pub fn any_nonzero(&self) -> bool {
        self.top_left > 0.0
            || self.top_right > 0.0
            || self.bottom_right > 0.0
            || self.bottom_left > 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    pub color: Color,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
}

/// Focus/decoration outline. Painted outside the element bounds, doesn't affect
/// layout. `offset` is the gap between border edge and outline; the outline is
/// stroked at `width` outside that.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outline {
    pub color: Color,
    pub width: f32,
    pub offset: f32,
}

impl Outline {
    /// Default focus ring used when a focused element has no explicit outline.
    pub const FOCUS_RING: Self = Self {
        color: Color::rgba(86, 156, 214, 255),
        width: 2.0,
        offset: 0.0,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}

impl Length {
    pub fn px(val: f32) -> Self {
        Length::Px(val)
    }

    pub fn percent(val: f32) -> Self {
        Length::Percent(val)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DefiniteLength {
    Px(f32),
    Percent(f32),
}

impl Default for DefiniteLength {
    fn default() -> Self {
        DefiniteLength::Px(0.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Display {
    None,
    #[default]
    Flex,
    Block,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AlignItems {
    FlexStart,
    FlexEnd,
    Center,
    #[default]
    Stretch,
    Baseline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AlignSelf {
    #[default]
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    Baseline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl Overflow {
    /// True when content that exceeds the box should produce a scrollbar
    /// (covers explicit `Scroll` and the `Auto` case where content overflows).
    pub fn is_scrollable(self) -> bool {
        matches!(self, Self::Scroll | Self::Auto)
    }

    /// True when overflowing content must be clipped to the box (everything
    /// except `Visible`).
    pub fn clips(self) -> bool {
        !matches!(self, Self::Visible)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct Size {
    pub width: Length,
    pub height: Length,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct GapSize {
    pub width: DefiniteLength,
    pub height: DefiniteLength,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Regular,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl FontWeight {
    pub fn to_parley(self) -> parley::FontWeight {
        match self {
            Self::Thin => parley::FontWeight::THIN,
            Self::ExtraLight => parley::FontWeight::EXTRA_LIGHT,
            Self::Light => parley::FontWeight::LIGHT,
            Self::Regular => parley::FontWeight::NORMAL,
            Self::Medium => parley::FontWeight::MEDIUM,
            Self::SemiBold => parley::FontWeight::SEMI_BOLD,
            Self::Bold => parley::FontWeight::BOLD,
            Self::ExtraBold => parley::FontWeight::EXTRA_BOLD,
            Self::Black => parley::FontWeight::BLACK,
        }
    }

    pub fn from_f16(value: u16) -> Option<Self> {
        match value {
            100 => Some(Self::Thin),
            200 => Some(Self::ExtraLight),
            300 => Some(Self::Light),
            400 => Some(Self::Regular),
            500 => Some(Self::Medium),
            600 => Some(Self::SemiBold),
            700 => Some(Self::Bold),
            800 => Some(Self::ExtraBold),
            900 => Some(Self::Black),
            _ => None,
        }
    }

    pub fn as_f16(weight: FontWeight) -> u16 {
        match weight {
            FontWeight::Thin => 100,
            FontWeight::ExtraLight => 200,
            FontWeight::Light => 300,
            FontWeight::Regular => 400,
            FontWeight::Medium => 500,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800,
            FontWeight::Black => 900,
        }
    }
}

pub use parley::{OverflowWrap, WordBreak};

pub use parley::layout::Alignment as TextAlign;

#[derive(Clone, Debug, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct TextStyle {
    pub font_size: f32,
    pub font_family: SharedString,
    pub color: Color,
    pub line_height: f32,
    pub font_weight: FontWeight,
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub overflow_wrap: OverflowWrap,
    pub word_break: WordBreak,
    pub text_align: TextAlign,
}

#[derive(Clone, Copy, Debug, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct TransformStyle {
    pub translate_x: f32,
    pub translate_y: f32,
    pub rotate: f32,
    pub scale_x: f32,
    pub scale_y: f32,
}

/// Styling for the scrollbar painted on scrollable views and multiline inputs.
/// Whether the scrollbar reserves layout space or paints over content.
///
/// `Overlay` (the default) is the macOS / modern-app style — thumb paints
/// over content and nothing reflows when it appears. `Gutter` is the
/// classic Windows / always-visible style: a reserved lane next to the
/// content holds the scrollbar so it has a persistent target to grab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollbarMode {
    #[default]
    Overlay,
    Gutter,
}

/// Defaults match the legacy hardcoded look (4px overlay thumb, white@90/140
/// alpha, transparent track, pill-shaped via auto-radius).
#[derive(Clone, Copy, Debug, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct ScrollbarStyle {
    pub width: f32,
    pub color: Color,
    pub hover_color: Color,
    pub active_color: Color,
    /// `None` = pill (radius = width / 2); `Some(v)` = explicit radius.
    pub radius: Option<f32>,
    pub mode: ScrollbarMode,
}

impl Default for ScrollbarStyle {
    fn default() -> Self {
        Self {
            width: 8.0,
            color: Color::rgba(128, 128, 128, 110),
            hover_color: Color::rgba(128, 128, 128, 160),
            active_color: Color::rgba(128, 128, 128, 200),
            radius: None,
            mode: ScrollbarMode::Overlay,
        }
    }
}

impl ScrollbarStyle {
    /// Layout gutter reserved for the scrollbar. Zero in overlay mode —
    /// the thumb paints over content and never reflows the layout.
    pub fn gutter_width(self) -> f32 {
        match self.mode {
            ScrollbarMode::Overlay => 0.0,
            ScrollbarMode::Gutter => self.width.max(0.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextSelectable {
    #[default]
    Inherit,
    True,
    False,
}

impl From<bool> for TextSelectable {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}

impl TextSelectable {
    pub fn as_value(&self) -> Option<bool> {
        (!matches!(self, Self::Inherit)).then_some(self == &Self::True)
    }

    #[inline]
    pub fn selectable(&self) -> bool {
        self == &Self::True
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            font_family: SharedString::new_static("sans-serif"),
            color: Color::WHITE,
            line_height: 1.2,
            font_weight: FontWeight::default(),
            letter_spacing: 0.0,
            word_spacing: 0.0,
            overflow_wrap: OverflowWrap::default(),
            word_break: WordBreak::default(),
            text_align: TextAlign::default(),
        }
    }
}

impl Default for TransformStyle {
    fn default() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            rotate: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

impl TransformStyle {
    pub fn is_identity(&self) -> bool {
        self.translate_x == 0.0
            && self.translate_y == 0.0
            && self.rotate == 0.0
            && self.scale_x == 1.0
            && self.scale_y == 1.0
    }

    pub fn to_affine(self, width: f64, height: f64) -> Affine {
        let cx = width * 0.5;
        let cy = height * 0.5;
        Affine::translate((self.translate_x as f64, self.translate_y as f64))
            * Affine::translate((cx, cy))
            * Affine::rotate((self.rotate as f64).to_radians())
            * Affine::scale_non_uniform(self.scale_x as f64, self.scale_y as f64)
            * Affine::translate((-cx, -cy))
    }
}

impl TextStyle {
    pub fn to_parley_styles(&self) -> impl Iterator<Item = StyleProperty<'static, TextBrush>> {
        let letter_spacing = (self.letter_spacing != 0.0)
            .then_some(StyleProperty::LetterSpacing(self.letter_spacing));
        let word_spacing =
            (self.word_spacing != 0.0).then_some(StyleProperty::WordSpacing(self.word_spacing));

        let font_family = FontFamily::from(&*self.font_family).into_owned();

        [
            StyleProperty::FontFamily(font_family),
            StyleProperty::FontSize(self.font_size),
            StyleProperty::LineHeight(LineHeight::FontSizeRelative(self.line_height)),
            StyleProperty::FontWeight(self.font_weight.to_parley()),
            StyleProperty::OverflowWrap(self.overflow_wrap),
            StyleProperty::WordBreak(self.word_break),
        ]
        .into_iter()
        .chain(letter_spacing)
        .chain(word_spacing)
    }

    pub fn to_parley_text_style(
        &self,
        node_id: usize,
    ) -> parley::TextStyle<'static, 'static, TextBrush> {
        parley::TextStyle {
            font_family: FontFamily::from(&*self.font_family).into_owned(),
            font_size: self.font_size,
            font_weight: self.font_weight.to_parley(),
            line_height: LineHeight::FontSizeRelative(self.line_height),
            letter_spacing: self.letter_spacing,
            word_spacing: self.word_spacing,
            overflow_wrap: self.overflow_wrap,
            word_break: self.word_break,
            brush: TextBrush::from_id(node_id),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct UzStyle {
    // Visibility
    pub display: Display,
    pub visibility: Visibility,
    pub position: Position,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,

    // Sizing
    #[refineable]
    pub size: Size,
    #[refineable]
    pub min_size: Size,
    #[refineable]
    pub max_size: Size,
    pub aspect_ratio: Option<f32>,

    // Spacing
    #[refineable]
    pub margin: Edges,
    #[refineable]
    pub padding: Edges,
    #[refineable]
    pub inset: Inset,

    // Flex layout
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Length,
    pub align_items: Option<AlignItems>,
    pub align_self: Option<AlignSelf>,
    pub justify_content: Option<JustifyContent>,
    #[refineable]
    pub gap: GapSize,

    // Visual
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    #[refineable]
    pub border_widths: Edges,
    #[refineable]
    pub corner_radii: Corners,
    pub opacity: f32,
    pub box_shadow: Option<BoxShadow>,
    pub outline: Option<Outline>,

    // Text (inherited)
    #[refineable]
    pub text: TextStyle,

    /// Whether text within this element is selectable.
    /// None = inherit from parent (default). Some(true) = selectable, Some(false) = not.
    pub text_selectable: TextSelectable,
    pub cursor: Option<UzCursorIcon>,

    // TODO: move these out of UzStyle.
    #[refineable]
    pub transform: TransformStyle,
    #[refineable]
    pub scrollbar: ScrollbarStyle,
}

impl Default for UzStyle {
    fn default() -> Self {
        Self {
            display: Display::default(),
            visibility: Visibility::default(),
            position: Position::default(),
            overflow_x: Overflow::default(),
            overflow_y: Overflow::default(),

            size: Size::default(),
            min_size: Size::default(),
            max_size: Size::default(),
            aspect_ratio: None,

            margin: Edges::default(),
            padding: Edges::default(),
            inset: Inset::default(),

            flex_direction: FlexDirection::default(),
            flex_wrap: FlexWrap::default(),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Length::Auto,
            align_items: None,
            align_self: None,
            justify_content: None,
            gap: GapSize::default(),

            background: None,
            border_color: None,
            border_widths: Edges::default(),
            corner_radii: Corners::default(),
            opacity: 1.0,
            box_shadow: None,
            outline: None,

            cursor: None,

            text: TextStyle::default(),
            transform: TransformStyle::default(),
            scrollbar: ScrollbarStyle::default(),
            text_selectable: TextSelectable::Inherit,
        }
    }
}

impl UzStyle {
    pub fn root() -> Self {
        Self {
            display: Display::Flex,
            size: Size {
                width: Length::Percent(1.0),
                height: Length::Percent(1.0),
            },
            text_selectable: TextSelectable::False,
            ..Default::default()
        }
    }

    pub fn inherit_from(&mut self, parent: &Self, authored: &UzStyleRefinement) {
        self.text.inherit_from(&parent.text, &authored.text);

        if authored.visibility.is_none() {
            self.visibility = parent.visibility;
        }
        if authored.text_selectable.is_none() {
            self.text_selectable = parent.text_selectable;
        }
        if authored.cursor.is_none() {
            self.cursor = parent.cursor;
        }
    }

    /// todo find a better place
    pub fn default_for_element(element_type: &str) -> Self {
        match element_type {
            "view" => Self {
                display: Display::Block,
                ..Default::default()
            },
            "button" => Self {
                flex_shrink: 0.0,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                cursor: Some(UzCursorIcon::Pointer),
                text: TextStyle {
                    overflow_wrap: OverflowWrap::Normal,
                    word_break: WordBreak::KeepAll,
                    ..Default::default()
                },
                ..Default::default()
            },
            "input" => Self {
                text: TextStyle {
                    overflow_wrap: OverflowWrap::Anywhere,
                    word_break: WordBreak::default(),
                    ..Default::default()
                },
                ..Default::default()
            },
            "text" | "#text" => Self {
                display: Display::Block,
                text: TextStyle {
                    overflow_wrap: OverflowWrap::Normal,
                    word_break: WordBreak::Normal,
                    ..Default::default()
                },
                ..Default::default()
            },
            _ => Default::default(),
        }
    }

    pub fn to_taffy(&self) -> taffy::Style {
        taffy::Style {
            display: match self.display {
                Display::None => taffy::Display::None,
                Display::Flex => taffy::Display::Flex,
                Display::Block => taffy::Display::Block,
            },
            position: match self.position {
                Position::Relative => taffy::Position::Relative,
                Position::Absolute => taffy::Position::Absolute,
            },
            overflow: taffy::Point {
                x: overflow_to_taffy(self.overflow_x),
                y: overflow_to_taffy(self.overflow_y),
            },
            scrollbar_width: self.scrollbar.gutter_width(),
            size: taffy::Size {
                width: length_to_dimension(self.size.width),
                height: length_to_dimension(self.size.height),
            },
            min_size: taffy::Size {
                width: length_to_dimension(self.min_size.width),
                height: length_to_dimension(self.min_size.height),
            },
            max_size: taffy::Size {
                width: length_to_dimension(self.max_size.width),
                height: length_to_dimension(self.max_size.height),
            },
            aspect_ratio: self.aspect_ratio,
            margin: edges_to_taffy_lp_auto(&self.margin),
            padding: edges_to_taffy_lp(&self.padding),
            border: edges_to_taffy_lp(&self.border_widths),
            inset: inset_to_taffy(&self.inset),
            flex_direction: match self.flex_direction {
                FlexDirection::Row => taffy::FlexDirection::Row,
                FlexDirection::Column => taffy::FlexDirection::Column,
                FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
                FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
            },
            flex_wrap: match self.flex_wrap {
                FlexWrap::NoWrap => taffy::FlexWrap::NoWrap,
                FlexWrap::Wrap => taffy::FlexWrap::Wrap,
                FlexWrap::WrapReverse => taffy::FlexWrap::WrapReverse,
            },
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: length_to_dimension(self.flex_basis),
            align_items: self.align_items.map(align_items_to_taffy),
            align_self: self.align_self.map(align_self_to_taffy),
            justify_content: self.justify_content.map(justify_content_to_taffy),
            gap: taffy::Size {
                width: definite_to_taffy(self.gap.width),
                height: definite_to_taffy(self.gap.height),
            },
            ..taffy::Style::default()
        }
    }

    /// Paint the visual properties into the scene at bounds.
    /// `continuation` is called between background and borders to paint children.
    pub fn paint(
        &self,
        bounds: Bounds,
        scene: &mut Scene,
        transform: Affine,
        continuation: impl FnOnce(&mut Scene),
    ) {
        if self.visibility == Visibility::Hidden || self.opacity <= 0.0 {
            return;
        }

        let opacity = self.opacity;

        // 1. Box shadow
        if let Some(shadow) = &self.box_shadow {
            self.paint_shadow(bounds, scene, shadow, opacity, transform);
        }

        // 2. Background
        if let Some(bg) = self.background
            && !bg.is_transparent()
        {
            let vbg = bg.with_opacity(opacity).to_vello();
            if self.corner_radii.any_nonzero() {
                let shape = rounded_rect(bounds, &self.corner_radii);
                scene.fill(vello::peniko::Fill::NonZero, transform, vbg, None, &shape);
            } else {
                scene.fill(
                    vello::peniko::Fill::NonZero,
                    transform,
                    vbg,
                    None,
                    &bounds.to_rect(),
                );
            }
        }

        // 3. Children
        continuation(scene);

        // 4. Borders
        if self.border_widths.any_nonzero()
            && let Some(bc) = self.border_color
            && !bc.is_transparent()
        {
            let vbc = bc.with_opacity(opacity).to_vello();
            if self.corner_radii.any_nonzero() {
                self.paint_rounded_borders(bounds, scene, vbc, transform);
            } else {
                self.paint_rect_borders(bounds, scene, vbc, transform);
            }
        }

        // 5. Outline (drawn outside the box, doesn't affect layout)
        if let Some(outline) = self.outline
            && outline.width > 0.0
            && !outline.color.is_transparent()
        {
            self.paint_outline(bounds, scene, outline, opacity, transform);
        }
    }

    fn paint_outline(
        &self,
        bounds: Bounds,
        scene: &mut Scene,
        outline: Outline,
        opacity: f32,
        transform: Affine,
    ) {
        let color = outline.color.with_opacity(opacity).to_vello();
        // Stroke is centered on the path, so push the path outward by
        // `offset + width/2` to land its inner edge `offset` away from the box.
        let inset = -(outline.offset as f64 + outline.width as f64 / 2.0);
        let outer = Bounds::new(
            bounds.x + inset,
            bounds.y + inset,
            bounds.width - 2.0 * inset,
            bounds.height - 2.0 * inset,
        );
        let stroke = Stroke::new(outline.width as f64);
        if self.corner_radii.any_nonzero() {
            let grow = outline.offset + outline.width / 2.0;
            let grown = Corners {
                top_left: (self.corner_radii.top_left + grow).max(0.0),
                top_right: (self.corner_radii.top_right + grow).max(0.0),
                bottom_right: (self.corner_radii.bottom_right + grow).max(0.0),
                bottom_left: (self.corner_radii.bottom_left + grow).max(0.0),
            };
            let shape = rounded_rect(outer, &grown);
            scene.stroke(&stroke, transform, color, None, &shape);
        } else {
            scene.stroke(&stroke, transform, color, None, &outer.to_rect());
        }
    }

    fn paint_shadow(
        &self,
        bounds: Bounds,
        scene: &mut Scene,
        shadow: &BoxShadow,
        opacity: f32,
        transform: Affine,
    ) {
        let spread = shadow.spread_radius as f64;
        let ox = shadow.offset_x as f64;
        let oy = shadow.offset_y as f64;
        let blur = shadow.blur_radius as f64;

        let expanded = Bounds::new(
            bounds.x + ox - spread - blur * 0.5,
            bounds.y + oy - spread - blur * 0.5,
            bounds.width + spread * 2.0 + blur,
            bounds.height + spread * 2.0 + blur,
        );

        let vc = shadow.color.with_opacity(opacity).to_vello();

        if self.corner_radii.any_nonzero() {
            let shape = rounded_rect(expanded, &self.corner_radii);
            scene.fill(vello::peniko::Fill::NonZero, transform, vc, None, &shape);
        } else {
            scene.fill(
                vello::peniko::Fill::NonZero,
                transform,
                vc,
                None,
                &expanded.to_rect(),
            );
        }
    }

    fn paint_rounded_borders(
        &self,
        bounds: Bounds,
        scene: &mut Scene,
        color: VelloColor,
        transform: Affine,
    ) {
        let bw = &self.border_widths;

        if let Some(width) = border_widths_equal(bw) {
            if width > 0.0 {
                let shape = rounded_rect(bounds, &self.corner_radii);
                scene.stroke(&Stroke::new(width as f64), transform, color, None, &shape);
            }
            return;
        }

        // Fill outer, carve inner
        let outer = rounded_rect(bounds, &self.corner_radii);
        scene.fill(vello::peniko::Fill::NonZero, transform, color, None, &outer);

        let inner_rect = Rect::new(
            bounds.x + bw.left as f64,
            bounds.y + bw.top as f64,
            bounds.x + bounds.width - bw.right as f64,
            bounds.y + bounds.height - bw.bottom as f64,
        );
        if inner_rect.width() <= 0.0 || inner_rect.height() <= 0.0 {
            return;
        }

        let inner_radii = inset_radii(&self.corner_radii, bw);
        let inner = rounded_rect(
            Bounds::new(
                inner_rect.x0,
                inner_rect.y0,
                inner_rect.width(),
                inner_rect.height(),
            ),
            &inner_radii,
        );

        let bg = self.background.unwrap_or(Color::TRANSPARENT).to_vello();
        scene.fill(vello::peniko::Fill::NonZero, transform, bg, None, &inner);
    }

    fn paint_rect_borders(
        &self,
        bounds: Bounds,
        scene: &mut Scene,
        color: VelloColor,
        transform: Affine,
    ) {
        let bw = &self.border_widths;
        let x = bounds.x;
        let y = bounds.y;
        let w = bounds.width;
        let h = bounds.height;

        if let Some(width) = border_widths_equal(bw) {
            if width > 0.0 {
                scene.stroke(
                    &Stroke::new(width as f64),
                    transform,
                    color,
                    None,
                    &Rect::new(x, y, x + w, y + h),
                );
            }
            return;
        }

        let fill = |scene: &mut Scene, rect: Rect| {
            scene.fill(vello::peniko::Fill::NonZero, transform, color, None, &rect);
        };

        if bw.top > 0.0 {
            fill(scene, Rect::new(x, y, x + w, y + bw.top as f64));
        }
        if bw.bottom > 0.0 {
            fill(scene, Rect::new(x, y + h - bw.bottom as f64, x + w, y + h));
        }
        if bw.left > 0.0 {
            fill(scene, Rect::new(x, y, x + bw.left as f64, y + h));
        }
        if bw.right > 0.0 {
            fill(scene, Rect::new(x + w - bw.right as f64, y, x + w, y + h));
        }
    }
}

impl TextStyle {
    pub fn inherit_from(&mut self, parent: &Self, authored: &TextStyleRefinement) {
        macro_rules! inherit {
            ($($field:ident),* $(,)?) => {
                $(
                    if authored.$field.is_none() {
                        self.$field.clone_from(&parent.$field);
                    }
                )*
            };
        }

        inherit!(
            font_size,
            font_family,
            color,
            line_height,
            font_weight,
            letter_spacing,
            word_spacing,
            overflow_wrap,
            word_break,
            text_align,
        );
    }
}

fn border_widths_equal(bw: &Edges) -> Option<f32> {
    let first = bw.top;
    if first > 0.0
        && (bw.right - first).abs() < f32::EPSILON
        && (bw.bottom - first).abs() < f32::EPSILON
        && (bw.left - first).abs() < f32::EPSILON
    {
        Some(first)
    } else {
        None
    }
}

fn rounded_rect(bounds: Bounds, radii: &Corners) -> RoundedRect {
    let w = bounds.width;
    let h = bounds.height;
    let clamp = |r: f32| r.max(0.0).min(w as f32 * 0.5).min(h as f32 * 0.5);
    let rect = bounds.to_rect();
    let rr = RoundedRectRadii::new(
        clamp(radii.top_left) as f64,
        clamp(radii.top_right) as f64,
        clamp(radii.bottom_right) as f64,
        clamp(radii.bottom_left) as f64,
    );
    RoundedRect::from_rect(rect, rr)
}

fn inset_radii(radii: &Corners, widths: &Edges) -> Corners {
    Corners {
        top_left: (radii.top_left - widths.top.max(widths.left)).max(0.0),
        top_right: (radii.top_right - widths.top.max(widths.right)).max(0.0),
        bottom_right: (radii.bottom_right - widths.bottom.max(widths.right)).max(0.0),
        bottom_left: (radii.bottom_left - widths.bottom.max(widths.left)).max(0.0),
    }
}

fn length_to_dimension(l: Length) -> taffy::Dimension {
    match l {
        Length::Auto => taffy::Dimension::auto(),
        Length::Px(v) => taffy::Dimension::length(v),
        Length::Percent(v) => taffy::Dimension::percent(v),
    }
}

fn definite_to_taffy(l: DefiniteLength) -> taffy::LengthPercentage {
    match l {
        DefiniteLength::Px(v) => taffy::LengthPercentage::length(v),
        DefiniteLength::Percent(v) => taffy::LengthPercentage::percent(v),
    }
}

fn length_to_lp_auto(l: Length) -> taffy::LengthPercentageAuto {
    match l {
        Length::Auto => taffy::LengthPercentageAuto::auto(),
        Length::Px(v) => taffy::LengthPercentageAuto::length(v),
        Length::Percent(v) => taffy::LengthPercentageAuto::percent(v),
    }
}

fn inset_to_taffy(e: &Inset) -> taffy::Rect<taffy::LengthPercentageAuto> {
    taffy::Rect {
        left: length_to_lp_auto(e.left),
        right: length_to_lp_auto(e.right),
        top: length_to_lp_auto(e.top),
        bottom: length_to_lp_auto(e.bottom),
    }
}

fn edges_to_taffy_lp_auto(e: &Edges) -> taffy::Rect<taffy::LengthPercentageAuto> {
    taffy::Rect {
        left: taffy::LengthPercentageAuto::length(e.left),
        right: taffy::LengthPercentageAuto::length(e.right),
        top: taffy::LengthPercentageAuto::length(e.top),
        bottom: taffy::LengthPercentageAuto::length(e.bottom),
    }
}

fn edges_to_taffy_lp(e: &Edges) -> taffy::Rect<taffy::LengthPercentage> {
    taffy::Rect {
        left: taffy::LengthPercentage::length(e.left),
        right: taffy::LengthPercentage::length(e.right),
        top: taffy::LengthPercentage::length(e.top),
        bottom: taffy::LengthPercentage::length(e.bottom),
    }
}

fn overflow_to_taffy(o: Overflow) -> taffy::Overflow {
    match o {
        Overflow::Visible => taffy::Overflow::Visible,
        Overflow::Hidden => taffy::Overflow::Hidden,
        Overflow::Scroll | Overflow::Auto => taffy::Overflow::Scroll,
    }
}

fn align_items_to_taffy(a: AlignItems) -> taffy::AlignItems {
    match a {
        AlignItems::FlexStart => taffy::AlignItems::FlexStart,
        AlignItems::FlexEnd => taffy::AlignItems::FlexEnd,
        AlignItems::Center => taffy::AlignItems::Center,
        AlignItems::Stretch => taffy::AlignItems::Stretch,
        AlignItems::Baseline => taffy::AlignItems::Baseline,
    }
}

fn align_self_to_taffy(a: AlignSelf) -> taffy::AlignSelf {
    match a {
        AlignSelf::Auto => taffy::AlignSelf::Start,
        AlignSelf::FlexStart => taffy::AlignSelf::FlexStart,
        AlignSelf::FlexEnd => taffy::AlignSelf::FlexEnd,
        AlignSelf::Center => taffy::AlignSelf::Center,
        AlignSelf::Stretch => taffy::AlignSelf::Stretch,
        AlignSelf::Baseline => taffy::AlignSelf::Baseline,
    }
}

fn justify_content_to_taffy(j: JustifyContent) -> taffy::JustifyContent {
    match j {
        JustifyContent::FlexStart => taffy::JustifyContent::FlexStart,
        JustifyContent::FlexEnd => taffy::JustifyContent::FlexEnd,
        JustifyContent::Center => taffy::JustifyContent::Center,
        JustifyContent::SpaceBetween => taffy::JustifyContent::SpaceBetween,
        JustifyContent::SpaceAround => taffy::JustifyContent::SpaceAround,
        JustifyContent::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
    }
}

#[cfg(test)]
mod tests {
    use super::{Display, UzStyle};

    #[test]
    fn text_elements_default_to_block_display() {
        assert_eq!(UzStyle::default_for_element("text").display, Display::Block);
        assert_eq!(
            UzStyle::default_for_element("#text").display,
            Display::Block
        );
    }
}
