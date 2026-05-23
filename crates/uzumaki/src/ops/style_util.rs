use crate::cursor::UzCursorIcon;
use crate::interactivity::StyleSlot;
use crate::node::{Node, UzNodeId};
use crate::prop_keys::{AttrValue, StyleProp};
use crate::style::*;
use crate::ui::UIState;

pub(crate) fn set_node_style(
    dom: &mut UIState,
    node_id: UzNodeId,
    prop: StyleProp,
    variant: StyleSlot,
    value: AttrValue<'_>,
    rem_base: f32,
) {
    let Some(node) = dom.nodes.get_mut(node_id) else {
        return;
    };

    match prop {
        StyleProp::W
        | StyleProp::H
        | StyleProp::MinW
        | StyleProp::MinH
        | StyleProp::Top
        | StyleProp::Right
        | StyleProp::Bottom
        | StyleProp::Left => {
            if let Some(length) = value.parse_length(rem_base) {
                set_style_length_prop(node, prop, variant, length);
            }
        }
        StyleProp::P => {
            if let Some(value) = value.parse_f32(rem_base) {
                let style = node.style_slot(variant);
                style.padding.top = Some(value);
                style.padding.right = Some(value);
                style.padding.bottom = Some(value);
                style.padding.left = Some(value);
            }
        }
        StyleProp::Px => {
            if let Some(value) = value.parse_f32(rem_base) {
                let style = node.style_slot(variant);
                style.padding.left = Some(value);
                style.padding.right = Some(value);
            }
        }
        StyleProp::Py => {
            if let Some(value) = value.parse_f32(rem_base) {
                let style = node.style_slot(variant);
                style.padding.top = Some(value);
                style.padding.bottom = Some(value);
            }
        }
        StyleProp::Pt => node.style_slot(variant).padding.top = value.parse_f32(rem_base),
        StyleProp::Pb => node.style_slot(variant).padding.bottom = value.parse_f32(rem_base),
        StyleProp::Pl => node.style_slot(variant).padding.left = value.parse_f32(rem_base),
        StyleProp::Pr => node.style_slot(variant).padding.right = value.parse_f32(rem_base),
        StyleProp::M => {
            if let Some(value) = value.parse_f32(rem_base) {
                let style = node.style_slot(variant);
                style.margin.top = Some(value);
                style.margin.right = Some(value);
                style.margin.bottom = Some(value);
                style.margin.left = Some(value);
            }
        }
        StyleProp::Mx => {
            if let Some(value) = value.parse_f32(rem_base) {
                let style = node.style_slot(variant);
                style.margin.left = Some(value);
                style.margin.right = Some(value);
            }
        }
        StyleProp::My => {
            if let Some(value) = value.parse_f32(rem_base) {
                let style = node.style_slot(variant);
                style.margin.top = Some(value);
                style.margin.bottom = Some(value);
            }
        }
        StyleProp::Mt => node.style_slot(variant).margin.top = value.parse_f32(rem_base),
        StyleProp::Mb => node.style_slot(variant).margin.bottom = value.parse_f32(rem_base),
        StyleProp::Ml => node.style_slot(variant).margin.left = value.parse_f32(rem_base),
        StyleProp::Mr => node.style_slot(variant).margin.right = value.parse_f32(rem_base),
        StyleProp::Flex => {
            if set_flex_prop(node, variant, value.as_str()) {
                return;
            }
            let parsed_f32 = value.parse_f32(rem_base);
            let parsed_bool = value.parse_bool();
            let style = node.style_slot(variant);
            if let Some(value) = parsed_f32 {
                style.display = Some(Display::Flex);
                style.flex_grow = Some(value);
            } else if parsed_bool {
                style.display = Some(Display::Flex);
            } else {
                style.display = Some(Display::Block);
                style.flex_grow = Some(0.0);
            }
        }
        StyleProp::FlexDir
        | StyleProp::FlexWrap
        | StyleProp::Items
        | StyleProp::Justify
        | StyleProp::Display
        | StyleProp::WordBreak
        | StyleProp::TextAlign
        | StyleProp::TextWrap
        | StyleProp::Position => {
            set_enum_style_prop_from_str(node, prop, variant, value.as_str());
        }
        StyleProp::FlexGrow => node.style_slot(variant).flex_grow = value.parse_f32(rem_base),
        StyleProp::FlexShrink => node.style_slot(variant).flex_shrink = value.parse_f32(rem_base),
        StyleProp::Gap => {
            if let Some(length) = value.parse_definite_length(rem_base) {
                set_gap(node, variant, length);
            }
        }
        StyleProp::Bg
        | StyleProp::Color
        | StyleProp::BorderColor
        | StyleProp::OutlineColor
        | StyleProp::ScrollbarColor
        | StyleProp::ScrollbarHoverColor
        | StyleProp::ScrollbarActiveColor => {
            if let Some(color) = crate::parse::parse_color(value.as_str()) {
                set_variant_color(node, prop, variant, color);
            }
        }
        StyleProp::FontSize => node.style_slot(variant).text.font_size = value.parse_f32(rem_base),
        StyleProp::FontWeight => {
            if let Some(weight) = parse_font_weight_str(value.as_str()) {
                node.style_slot(variant).text.font_weight = Some(weight);
            }
        }
        StyleProp::FontFamily => {
            let value = value.as_str().trim();
            if !value.is_empty() {
                node.style_slot(variant).text.font_family = Some(value.into());
            }
        }
        StyleProp::Rounded => {
            if let Some(value) = value.parse_f32(rem_base) {
                let style = node.style_slot(variant);
                style.corner_radii.top_left = Some(value);
                style.corner_radii.top_right = Some(value);
                style.corner_radii.bottom_right = Some(value);
                style.corner_radii.bottom_left = Some(value);
            }
        }
        StyleProp::RoundedTL => {
            node.style_slot(variant).corner_radii.top_left = value.parse_f32(rem_base)
        }
        StyleProp::RoundedTR => {
            node.style_slot(variant).corner_radii.top_right = value.parse_f32(rem_base)
        }
        StyleProp::RoundedBR => {
            node.style_slot(variant).corner_radii.bottom_right = value.parse_f32(rem_base)
        }
        StyleProp::RoundedBL => {
            node.style_slot(variant).corner_radii.bottom_left = value.parse_f32(rem_base)
        }
        StyleProp::Border => {
            if let Some(value) = value.parse_f32(rem_base) {
                let style = node.style_slot(variant);
                style.border_widths.top = Some(value);
                style.border_widths.right = Some(value);
                style.border_widths.bottom = Some(value);
                style.border_widths.left = Some(value);
            }
        }
        StyleProp::BorderTop => {
            node.style_slot(variant).border_widths.top = value.parse_f32(rem_base)
        }
        StyleProp::BorderRight => {
            node.style_slot(variant).border_widths.right = value.parse_f32(rem_base)
        }
        StyleProp::BorderBottom => {
            node.style_slot(variant).border_widths.bottom = value.parse_f32(rem_base)
        }
        StyleProp::BorderLeft => {
            node.style_slot(variant).border_widths.left = value.parse_f32(rem_base)
        }
        StyleProp::Outline => {
            if let Some(value) = value.parse_f32(rem_base) {
                let style = node.style_slot(variant);
                let outline = style.outline.get_or_insert(Outline::FOCUS_RING);
                outline.width = value;
            }
        }
        StyleProp::OutlineOffset => {
            if let Some(value) = value.parse_f32(rem_base) {
                let style = node.style_slot(variant);
                let outline = style.outline.get_or_insert(Outline::FOCUS_RING);
                outline.offset = value;
            }
        }
        StyleProp::Opacity => node.style_slot(variant).opacity = value.parse_f32(rem_base),
        StyleProp::Cursor => {
            if let Some(cursor) = UzCursorIcon::parse(value.as_str()) {
                node.style_slot(variant).cursor = Some(cursor);
            }
        }
        StyleProp::Visibility => {
            if let Some(visibility) = parse_visibility(value.as_str()) {
                node.style_slot(variant).visibility = Some(visibility);
            }
        }
        StyleProp::Scroll => {
            let parsed_bool = value.parse_bool();
            let style = node.style_slot(variant);
            let overflow = if parsed_bool {
                Overflow::Auto
            } else {
                Overflow::Visible
            };
            style.overflow_x = Some(overflow);
            style.overflow_y = Some(overflow);
            if !parsed_bool {
                node.scroll_state = Default::default();
            }
        }
        StyleProp::ScrollX => {
            let parsed_bool = value.parse_bool();
            let style = node.style_slot(variant);
            style.overflow_x = Some(if parsed_bool {
                Overflow::Auto
            } else {
                Overflow::Visible
            });
            if !parsed_bool {
                node.scroll_state.scroll_offset_x = 0.0;
            }
        }
        StyleProp::ScrollY => {
            let parsed_bool = value.parse_bool();
            let style = node.style_slot(variant);
            style.overflow_y = Some(if parsed_bool {
                Overflow::Auto
            } else {
                Overflow::Visible
            });
            if !parsed_bool {
                node.scroll_state.scroll_offset_y = 0.0;
            }
        }
        StyleProp::ScrollbarWidth => {
            node.style_slot(variant).scrollbar.width = value.parse_f32(rem_base)
        }
        StyleProp::ScrollbarRadius => {
            node.style_slot(variant).scrollbar.radius = value.parse_f32(rem_base)
        }
        StyleProp::ScrollbarMode => {
            use crate::style::ScrollbarMode;
            let mode = match value.as_str() {
                "overlay" | "floating" => Some(ScrollbarMode::Overlay),
                "gutter" | "reserved" | "inline" => Some(ScrollbarMode::Gutter),
                _ => None,
            };
            if let Some(mode) = mode {
                node.style_slot(variant).scrollbar.mode = Some(mode);
            }
        }
        StyleProp::TextSelect => {
            let text_selectable: TextSelectable = value.parse_bool().into();
            let style = node.style_slot(variant);
            style.text_selectable = Some(text_selectable);
            if variant == StyleSlot::Base {
                node.set_text_selectable(text_selectable);
            }
        }
        StyleProp::TranslateX => {
            node.style_slot(variant).transform.translate_x = value.parse_f32(rem_base)
        }
        StyleProp::TranslateY => {
            node.style_slot(variant).transform.translate_y = value.parse_f32(rem_base)
        }
        StyleProp::Rotate => node.style_slot(variant).transform.rotate = value.parse_f32(rem_base),
        StyleProp::Scale => {
            if let Some(value) = value.parse_f32(rem_base) {
                let style = node.style_slot(variant);
                style.transform.scale_x = Some(value);
                style.transform.scale_y = Some(value);
            }
        }
        StyleProp::ScaleX => node.style_slot(variant).transform.scale_x = value.parse_f32(rem_base),
        StyleProp::ScaleY => node.style_slot(variant).transform.scale_y = value.parse_f32(rem_base),
    }
}

pub(crate) fn clear_node_style(
    dom: &mut UIState,
    node_id: UzNodeId,
    prop: StyleProp,
    variant: StyleSlot,
) {
    let Some(node) = dom.nodes.get_mut(node_id) else {
        return;
    };

    clear_style_prop(node, prop, variant);

    match prop {
        StyleProp::Scroll => node.scroll_state = Default::default(),
        StyleProp::ScrollX => node.scroll_state.scroll_offset_x = 0.0,
        StyleProp::ScrollY => node.scroll_state.scroll_offset_y = 0.0,
        StyleProp::TextSelect if variant == StyleSlot::Base => {
            node.set_text_selectable(TextSelectable::Inherit);
        }
        _ => {}
    }
}

fn parse_visibility(value: &str) -> Option<Visibility> {
    match value.trim() {
        "visible" | "show" => Some(Visibility::Visible),
        "hidden" | "hide" => Some(Visibility::Hidden),
        _ => None,
    }
}

fn set_variant_color(node: &mut Node, prop: StyleProp, variant: StyleSlot, color: Color) {
    let r = node.style_slot(variant);

    match prop {
        StyleProp::Bg => r.background = Some(color),
        StyleProp::Color => r.text.color = Some(color),
        StyleProp::BorderColor => r.border_color = Some(color),
        StyleProp::OutlineColor => {
            let outline = r.outline.get_or_insert(Outline::FOCUS_RING);
            outline.color = color;
        }
        StyleProp::ScrollbarColor => r.scrollbar.color = Some(color),
        StyleProp::ScrollbarHoverColor => r.scrollbar.hover_color = Some(color),
        StyleProp::ScrollbarActiveColor => r.scrollbar.active_color = Some(color),
        _ => {}
    }
}

fn set_style_length_prop(node: &mut Node, prop: StyleProp, variant: StyleSlot, length: Length) {
    let r = node.style_slot(variant);
    match prop {
        StyleProp::W => r.size.width = Some(length),
        StyleProp::H => r.size.height = Some(length),
        StyleProp::MinW => r.min_size.width = Some(length),
        StyleProp::MinH => r.min_size.height = Some(length),
        StyleProp::Top => r.inset.top = Some(length),
        StyleProp::Right => r.inset.right = Some(length),
        StyleProp::Bottom => r.inset.bottom = Some(length),
        StyleProp::Left => r.inset.left = Some(length),
        _ => {
            // rest doesnt affect length
        }
    }
}

fn set_gap(node: &mut Node, variant: StyleSlot, length: DefiniteLength) {
    let r = node.style_slot(variant);
    r.gap.width = Some(length);
    r.gap.height = Some(length);
}

fn set_enum_style_prop_from_str(
    node: &mut Node,
    prop: StyleProp,
    variant: StyleSlot,
    value: &str,
) -> bool {
    let value = value.trim();
    let r = node.style_slot(variant);
    match prop {
        StyleProp::FlexDir => {
            r.flex_direction = Some(match value {
                "row" => FlexDirection::Row,
                "col" | "column" => FlexDirection::Column,
                "row-reverse" => FlexDirection::RowReverse,
                "col-reverse" | "column-reverse" => FlexDirection::ColumnReverse,
                _ => return false,
            });
        }
        StyleProp::FlexWrap => {
            r.flex_wrap = Some(match value {
                "nowrap" | "no-wrap" => FlexWrap::NoWrap,
                "wrap" => FlexWrap::Wrap,
                "wrap-reverse" => FlexWrap::WrapReverse,
                _ => return false,
            });
        }
        StyleProp::Items => {
            r.align_items = Some(match value {
                "flex-start" | "start" => AlignItems::FlexStart,
                "flex-end" | "end" => AlignItems::FlexEnd,
                "center" => AlignItems::Center,
                "stretch" => AlignItems::Stretch,
                "baseline" => AlignItems::Baseline,
                _ => return false,
            });
        }
        StyleProp::Justify => {
            r.justify_content = Some(match value {
                "flex-start" | "start" => JustifyContent::FlexStart,
                "flex-end" | "end" => JustifyContent::FlexEnd,
                "center" => JustifyContent::Center,
                "space-between" | "between" => JustifyContent::SpaceBetween,
                "space-around" | "around" => JustifyContent::SpaceAround,
                "space-evenly" | "evenly" => JustifyContent::SpaceEvenly,
                _ => return false,
            });
        }
        StyleProp::Display => {
            r.display = Some(match value {
                "none" => Display::None,
                "flex" => Display::Flex,
                "block" => Display::Block,
                _ => return false,
            });
        }
        StyleProp::TextWrap => match value {
            "wrap" => {
                r.text.overflow_wrap = Some(OverflowWrap::Normal);
                r.text.word_break = Some(WordBreak::Normal);
            }
            "nowrap" | "none" => {
                r.text.overflow_wrap = Some(OverflowWrap::Normal);
                r.text.word_break = Some(WordBreak::KeepAll);
            }
            "anywhere" => {
                r.text.overflow_wrap = Some(OverflowWrap::Anywhere);
            }
            "break-word" => {
                r.text.overflow_wrap = Some(OverflowWrap::BreakWord);
            }
            _ => return false,
        },
        StyleProp::WordBreak => {
            r.text.word_break = Some(match value {
                "normal" => WordBreak::Normal,
                "break-all" => WordBreak::BreakAll,
                "keep-all" => WordBreak::KeepAll,
                _ => return false,
            });
        }
        StyleProp::TextAlign => {
            r.text.text_align = Some(match value {
                "start" => TextAlign::Start,
                "end" => TextAlign::End,
                "left" => TextAlign::Left,
                "center" => TextAlign::Center,
                "right" => TextAlign::Right,
                "justify" => TextAlign::Justify,
                _ => return false,
            });
        }
        StyleProp::Position => {
            r.position = Some(match value {
                "relative" => Position::Relative,
                "absolute" => Position::Absolute,
                _ => return false,
            });
        }
        _ => return false,
    }
    true
}

// flex = { val }
fn set_flex_prop(node: &mut Node, variant: StyleSlot, value: &str) -> bool {
    let dir = match value.trim() {
        "row" => FlexDirection::Row,
        "col" | "column" => FlexDirection::Column,
        "row-reverse" => FlexDirection::RowReverse,
        "col-reverse" | "column-reverse" => FlexDirection::ColumnReverse,
        _ => return false,
    };
    let r = node.style_slot(variant);
    r.display = Some(Display::Flex);
    r.flex_direction = Some(dir);
    true
}

fn clear_style_prop(node: &mut Node, prop: StyleProp, variant: StyleSlot) {
    let style = node.style_slot(variant);

    match prop {
        StyleProp::W => style.size.width = None,
        StyleProp::H => style.size.height = None,
        StyleProp::MinW => style.min_size.width = None,
        StyleProp::MinH => style.min_size.height = None,
        StyleProp::P => style.padding = EdgesRefinement::default(),
        StyleProp::Px => {
            style.padding.left = None;
            style.padding.right = None;
        }
        StyleProp::Py => {
            style.padding.top = None;
            style.padding.bottom = None;
        }
        StyleProp::Pt => style.padding.top = None,
        StyleProp::Pb => style.padding.bottom = None,
        StyleProp::Pl => style.padding.left = None,
        StyleProp::Pr => style.padding.right = None,
        StyleProp::M => style.margin = EdgesRefinement::default(),
        StyleProp::Mx => {
            style.margin.left = None;
            style.margin.right = None;
        }
        StyleProp::My => {
            style.margin.top = None;
            style.margin.bottom = None;
        }
        StyleProp::Mt => style.margin.top = None,
        StyleProp::Mb => style.margin.bottom = None,
        StyleProp::Ml => style.margin.left = None,
        StyleProp::Mr => style.margin.right = None,
        StyleProp::Flex => {
            style.display = None;
            style.flex_grow = None;
        }
        StyleProp::FlexDir => style.flex_direction = None,
        StyleProp::FlexWrap => style.flex_wrap = None,
        StyleProp::FlexGrow => style.flex_grow = None,
        StyleProp::FlexShrink => style.flex_shrink = None,
        StyleProp::Items => style.align_items = None,
        StyleProp::Justify => style.justify_content = None,
        StyleProp::Gap => style.gap = GapSizeRefinement::default(),
        StyleProp::Bg => style.background = None,
        StyleProp::Color => style.text.color = None,
        StyleProp::FontSize => style.text.font_size = None,
        StyleProp::FontWeight => style.text.font_weight = None,
        StyleProp::FontFamily => style.text.font_family = None,
        StyleProp::Rounded => style.corner_radii = CornersRefinement::default(),
        StyleProp::RoundedTL => style.corner_radii.top_left = None,
        StyleProp::RoundedTR => style.corner_radii.top_right = None,
        StyleProp::RoundedBR => style.corner_radii.bottom_right = None,
        StyleProp::RoundedBL => style.corner_radii.bottom_left = None,
        StyleProp::Border => style.border_widths = EdgesRefinement::default(),
        StyleProp::BorderTop => style.border_widths.top = None,
        StyleProp::BorderRight => style.border_widths.right = None,
        StyleProp::BorderBottom => style.border_widths.bottom = None,
        StyleProp::BorderLeft => style.border_widths.left = None,
        StyleProp::Opacity => style.opacity = None,
        StyleProp::BorderColor => style.border_color = None,
        StyleProp::Outline | StyleProp::OutlineColor | StyleProp::OutlineOffset => {
            style.outline = None;
        }
        StyleProp::Display => style.display = None,
        StyleProp::Cursor => style.cursor = None,
        StyleProp::Visibility => style.visibility = None,
        StyleProp::Scroll => {
            style.overflow_x = None;
            style.overflow_y = None;
        }
        StyleProp::ScrollX => style.overflow_x = None,
        StyleProp::ScrollY => style.overflow_y = None,
        StyleProp::ScrollbarWidth => style.scrollbar.width = None,
        StyleProp::ScrollbarColor => style.scrollbar.color = None,
        StyleProp::ScrollbarHoverColor => style.scrollbar.hover_color = None,
        StyleProp::ScrollbarActiveColor => style.scrollbar.active_color = None,
        StyleProp::ScrollbarRadius => style.scrollbar.radius = None,
        StyleProp::ScrollbarMode => style.scrollbar.mode = None,
        StyleProp::TextSelect => style.text_selectable = None,
        StyleProp::TextWrap => {
            style.text.overflow_wrap = None;
            style.text.word_break = None;
        }
        StyleProp::WordBreak => style.text.word_break = None,
        StyleProp::TextAlign => style.text.text_align = None,
        StyleProp::Position => style.position = None,
        StyleProp::Top => style.inset.top = None,
        StyleProp::Right => style.inset.right = None,
        StyleProp::Bottom => style.inset.bottom = None,
        StyleProp::Left => style.inset.left = None,
        StyleProp::TranslateX => style.transform.translate_x = None,
        StyleProp::TranslateY => style.transform.translate_y = None,
        StyleProp::Rotate => style.transform.rotate = None,
        StyleProp::Scale => {
            style.transform.scale_x = None;
            style.transform.scale_y = None;
        }
        StyleProp::ScaleX => style.transform.scale_x = None,
        StyleProp::ScaleY => style.transform.scale_y = None,
    }
}

fn parse_font_weight_str(value: &str) -> Option<FontWeight> {
    match value.trim().to_ascii_lowercase().as_str() {
        "thin" => Some(FontWeight::Thin),
        "extra-light" | "extralight" | "ultra-light" | "ultralight" => Some(FontWeight::ExtraLight),
        "light" => Some(FontWeight::Light),
        "normal" | "regular" => Some(FontWeight::Regular),
        "medium" => Some(FontWeight::Medium),
        "semi-bold" | "semibold" | "demi-bold" | "demibold" => Some(FontWeight::SemiBold),
        "bold" => Some(FontWeight::Bold),
        "extra-bold" | "extrabold" | "ultra-bold" | "ultrabold" => Some(FontWeight::ExtraBold),
        "black" | "heavy" => Some(FontWeight::Black),
        value => value.parse::<u16>().ok().and_then(FontWeight::from_f16),
    }
}

#[cfg(test)]
mod tests {
    // todo add back old tests
}
