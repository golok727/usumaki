use std::{ops::Deref, str::FromStr};

use serde::Deserialize;

use crate::interactivity::StyleSlot;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct AttrValue<'a>(pub &'a str);

impl<'a> From<&'a str> for AttrValue<'a> {
    fn from(value: &'a str) -> Self {
        AttrValue(value)
    }
}

impl<'a> Deref for AttrValue<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl AttrValue<'_> {
    pub fn as_str(&self) -> &str {
        self.0
    }

    pub fn parse_bool(&self) -> bool {
        crate::parse::parse_bool(self)
    }

    pub fn parse_f32(&self, rem_base: f32) -> Option<f32> {
        crate::parse::parse_px_scalar(self, rem_base)
    }

    pub fn parse_length(&self, rem_base: f32) -> Option<crate::style::Length> {
        crate::parse::parse_length(self, rem_base)
    }

    pub fn parse_definite_length(&self, rem_base: f32) -> Option<crate::style::DefiniteLength> {
        crate::parse::parse_definite_length(self, rem_base)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum StyleProp {
    W,
    H,
    MinW,
    MinH,
    P,
    Px,
    Py,
    Pt,
    Pb,
    Pl,
    Pr,
    M,
    Mx,
    My,
    Mt,
    Mb,
    Ml,
    Mr,
    Flex,
    FlexDir,
    FlexWrap,
    FlexGrow,
    FlexShrink,
    Items,
    Justify,
    Gap,
    Bg,
    Color,
    FontSize,
    FontWeight,
    FontFamily,
    Rounded,
    RoundedTL,
    RoundedTR,
    RoundedBR,
    RoundedBL,
    Border,
    BorderTop,
    BorderRight,
    BorderBottom,
    BorderLeft,
    BorderColor,
    Outline,
    OutlineColor,
    OutlineOffset,
    Opacity,
    Display,
    Cursor,
    Visibility,
    Scroll,
    ScrollX,
    ScrollY,
    ScrollbarWidth,
    ScrollbarColor,
    ScrollbarHoverColor,
    ScrollbarActiveColor,
    ScrollbarRadius,
    ScrollbarMode,
    TextSelect,
    TextWrap,
    WordBreak,
    TextAlign,
    Position,
    Top,
    Right,
    Bottom,
    Left,
    TranslateX,
    TranslateY,
    Rotate,
    Scale,
    ScaleX,
    ScaleY,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum AttributeKind<'a> {
    Style(StyleProp, StyleSlot),
    Element(&'a str),
}

impl<'a> AttributeKind<'a> {
    pub fn parse(name: &'a str) -> Self {
        if let Some(rest) = name.strip_prefix("hover:")
            && let Ok(prop) = rest.parse::<StyleProp>()
        {
            return Self::Style(prop, StyleSlot::Hover);
        }

        if let Some(rest) = name.strip_prefix("active:")
            && let Ok(prop) = rest.parse::<StyleProp>()
        {
            return Self::Style(prop, StyleSlot::Active);
        }

        if let Some(rest) = name.strip_prefix("focus:")
            && let Ok(prop) = rest.parse::<StyleProp>()
        {
            return Self::Style(prop, StyleSlot::Focus);
        }

        if let Ok(prop) = name.parse::<StyleProp>() {
            return Self::Style(prop, StyleSlot::Base);
        }

        Self::Element(name)
    }
}

impl FromStr for StyleProp {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "w" => Self::W,
            "h" => Self::H,
            "minW" => Self::MinW,
            "minH" => Self::MinH,
            "p" => Self::P,
            "px" => Self::Px,
            "py" => Self::Py,
            "pt" => Self::Pt,
            "pb" => Self::Pb,
            "pl" => Self::Pl,
            "pr" => Self::Pr,
            "m" => Self::M,
            "mx" => Self::Mx,
            "my" => Self::My,
            "mt" => Self::Mt,
            "mb" => Self::Mb,
            "ml" => Self::Ml,
            "mr" => Self::Mr,
            "flex" => Self::Flex,
            "flexDir" => Self::FlexDir,
            "flexWrap" => Self::FlexWrap,
            "flexGrow" => Self::FlexGrow,
            "flexShrink" => Self::FlexShrink,
            "items" => Self::Items,
            "justify" => Self::Justify,
            "gap" => Self::Gap,
            "bg" => Self::Bg,
            "color" => Self::Color,
            "fontSize" => Self::FontSize,
            "fontWeight" => Self::FontWeight,
            "fontFamily" => Self::FontFamily,
            "rounded" => Self::Rounded,
            "roundedTL" => Self::RoundedTL,
            "roundedTR" => Self::RoundedTR,
            "roundedBR" => Self::RoundedBR,
            "roundedBL" => Self::RoundedBL,
            "border" => Self::Border,
            "borderTop" => Self::BorderTop,
            "borderRight" => Self::BorderRight,
            "borderBottom" => Self::BorderBottom,
            "borderLeft" => Self::BorderLeft,
            "borderColor" => Self::BorderColor,
            "outline" => Self::Outline,
            "outlineColor" => Self::OutlineColor,
            "outlineOffset" => Self::OutlineOffset,
            "opacity" => Self::Opacity,
            "display" => Self::Display,
            "cursor" => Self::Cursor,
            "visibility" => Self::Visibility,
            "scroll" | "scrollable" => Self::Scroll,
            "scrollX" | "scrollableX" => Self::ScrollX,
            "scrollY" | "scrollableY" => Self::ScrollY,
            "scrollbarWidth" => Self::ScrollbarWidth,
            "scrollbarColor" => Self::ScrollbarColor,
            "scrollbarHoverColor" => Self::ScrollbarHoverColor,
            "scrollbarActiveColor" => Self::ScrollbarActiveColor,
            "scrollbarRadius" => Self::ScrollbarRadius,
            "scrollbarMode" => Self::ScrollbarMode,
            "selectable" => Self::TextSelect,
            "textWrap" => Self::TextWrap,
            "wordBreak" => Self::WordBreak,
            "textAlign" => Self::TextAlign,
            "position" => Self::Position,
            "top" => Self::Top,
            "right" => Self::Right,
            "bottom" => Self::Bottom,
            "left" => Self::Left,
            "translateX" => Self::TranslateX,
            "translateY" => Self::TranslateY,
            "rotate" => Self::Rotate,
            "scale" => Self::Scale,
            "scaleX" => Self::ScaleX,
            "scaleY" => Self::ScaleY,
            _ => return Err(()),
        })
    }
}
