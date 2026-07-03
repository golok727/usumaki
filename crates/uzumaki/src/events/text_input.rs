use crate::input::input_align_offset;
use crate::layout::TaffyLayoutExt;
use crate::node::UzNodeId;
use crate::style::TextStyle;
use crate::text::{apply_text_style_to_editor, secure_cursor_geometry};
use crate::ui::UIState;
use crate::window::Window;

pub struct FocusedInputLayoutMeta {
    pub taffy_x: f64,
    pub taffy_y: f64,
    pub content_x: f32,
    pub content_y: f32,
    pub multiline: bool,
    pub text_style: TextStyle,
    pub input_width: f32,
    pub input_height: f32,
}

pub fn input_layout_meta(dom: &UIState, focused_id: UzNodeId) -> Option<FocusedInputLayoutMeta> {
    let node = dom.nodes.get(focused_id)?;
    let is = node.as_text_input()?;
    let text_style = node.computed_style().text.clone();
    let hb = node.hitbox_id.and_then(|hid| dom.hitbox_store.get(hid))?;
    let layout = &node.final_layout;
    let content_box = layout.content_box_bounds();
    let bounds = hb.window_aabb();
    Some(FocusedInputLayoutMeta {
        taffy_x: bounds.x,
        taffy_y: bounds.y,
        content_x: content_box.x as f32,
        content_y: content_box.y as f32,
        multiline: is.multiline,
        text_style,
        input_width: content_box.width as f32,
        input_height: content_box.height as f32,
    })
}

fn sync_focused_input_cursor(
    dom: &mut UIState,
    handle: &mut Window,
    focused_id: UzNodeId,
    meta: &FocusedInputLayoutMeta,
) -> Option<(parley::BoundingBox, f32, f32)> {
    let node = dom.nodes.get_mut(focused_id)?;
    let cursor_rect = {
        let is = node.as_text_input_mut()?;
        apply_text_style_to_editor(&mut is.editor, &meta.text_style);
        is.editor.set_width(if meta.multiline {
            Some(meta.input_width)
        } else {
            None
        });
        is.editor.refresh_layout(
            &mut handle.text_renderer.font_ctx,
            &mut handle.text_renderer.layout_ctx,
        );
        if is.secure {
            secure_cursor_geometry(&is.editor, 1.5, &meta.text_style, &mut handle.text_renderer)
        } else {
            is.editor.cursor_geometry(1.5)
        }
    }?;
    let scroll_offset_x = node.scroll_state.scroll_offset_x;
    let scroll_offset_y = node.scroll_state.scroll_offset_y;
    Some((cursor_rect, scroll_offset_x, scroll_offset_y))
}

fn set_ime_cursor_area(
    handle: &mut Window,
    meta: &FocusedInputLayoutMeta,
    ime_area: &parley::BoundingBox,
    _scroll_offset_x: f32,
    scroll_offset_y: f32,
) {
    let line_height = (meta.text_style.font_size * meta.text_style.line_height).round() as f64;
    let text_origin_x = meta.taffy_x + meta.content_x as f64;
    let text_origin_y = if meta.multiline {
        meta.taffy_y + meta.content_y as f64 - scroll_offset_y as f64
    } else {
        meta.taffy_y
            + meta.content_y as f64
            + ((meta.input_height as f64 - line_height) / 2.0).max(0.0)
    };
    let position =
        winit::dpi::LogicalPosition::new(text_origin_x + ime_area.x0, text_origin_y + ime_area.y0);
    let size = winit::dpi::LogicalSize::new(
        (ime_area.x1 - ime_area.x0).max(24.0) as f32,
        (ime_area.y1 - ime_area.y0).max(1.0) as f32,
    );
    handle.set_ime_cursor_area(position, size);
}

pub fn update_ime_cursor_area(dom: &mut UIState, handle: &mut Window) {
    let Some(focused_id) = dom.focused_node else {
        return;
    };
    let Some(meta) = input_layout_meta(dom, focused_id) else {
        return;
    };
    let Some((_cursor_rect, scroll_offset_x, scroll_offset_y)) =
        sync_focused_input_cursor(dom, handle, focused_id, &meta)
    else {
        return;
    };
    let Some(node) = dom.nodes.get(focused_id) else {
        return;
    };
    let Some(is) = node.as_text_input() else {
        return;
    };
    let ime_area = is.editor.ime_cursor_area();
    set_ime_cursor_area(handle, &meta, &ime_area, scroll_offset_x, scroll_offset_y);
}

/// Browser-style horizontal alignment shift for a single-line input. Returns
/// 0 for multiline (the editor handles alignment internally) or when the text
/// is wider than the content box (scroll takes over). Refreshes the editor's
/// layout as a side effect so callers see consistent natural-coord geometry.
pub(super) fn single_line_align_offset(
    dom: &mut UIState,
    handle: &mut Window,
    nid: UzNodeId,
) -> f32 {
    let Some(meta) = input_layout_meta(dom, nid) else {
        return 0.0;
    };
    if meta.multiline {
        return 0.0;
    }
    let Some(node) = dom.nodes.get_mut(nid) else {
        return 0.0;
    };
    let Some(is) = node.as_text_input_mut() else {
        return 0.0;
    };
    apply_text_style_to_editor(&mut is.editor, &meta.text_style);
    is.editor.set_width(None);
    is.editor.refresh_layout(
        &mut handle.text_renderer.font_ctx,
        &mut handle.text_renderer.layout_ctx,
    );
    let display_text = is.display_text();
    let natural_w = handle
        .text_renderer
        .measure_text(&display_text, &meta.text_style, None, None)
        .0;
    input_align_offset(meta.input_width, natural_w, meta.text_style.text_align)
}

/// Scroll the focused input so the cursor stays visible.
/// Call this after any action that moves the cursor (key press, click, drag).
pub fn scroll_input_to_cursor(dom: &mut UIState, handle: &mut Window) {
    let Some(focused_id) = dom.focused_node else {
        return;
    };
    let Some(meta) = input_layout_meta(dom, focused_id) else {
        return;
    };

    if let Some(node) = dom.nodes.get_mut(focused_id)
        && let Some(is) = node.as_text_input_mut()
    {
        apply_text_style_to_editor(&mut is.editor, &meta.text_style);
        is.editor.set_width(if meta.multiline {
            Some(meta.input_width)
        } else {
            None
        });
        is.editor.refresh_layout(
            &mut handle.text_renderer.font_ctx,
            &mut handle.text_renderer.layout_ctx,
        );
        let cursor_rect = if is.secure {
            secure_cursor_geometry(&is.editor, 1.5, &meta.text_style, &mut handle.text_renderer)
        } else {
            is.editor.cursor_geometry(1.5)
        };
        if let Some(rect) = cursor_rect {
            if meta.multiline {
                let line_height = (meta.text_style.font_size * meta.text_style.line_height).round();
                node.scroll_state
                    .scroll_input_y(rect.y0 as f32, line_height, meta.input_height);
            } else {
                let display_text = is.display_text();
                let natural_w = handle
                    .text_renderer
                    .measure_text(&display_text, &meta.text_style, None, None)
                    .0;
                let raw_selection = is.editor.raw_selection();
                let cursor_at_text_end = raw_selection.is_collapsed()
                    && raw_selection.focus().index() == is.editor.raw_text().len();
                if cursor_at_text_end {
                    node.scroll_state
                        .scroll_single_line_input_end(natural_w, meta.input_width);
                } else {
                    node.scroll_state.scroll_input_x(
                        rect.x0 as f32,
                        rect.x1 as f32,
                        natural_w,
                        meta.input_width,
                    );
                }
            }
        }
    }

    if let Some((_cursor_rect, scroll_offset_x, scroll_offset_y)) =
        sync_focused_input_cursor(dom, handle, focused_id, &meta)
        && let Some(node) = dom.nodes.get(focused_id)
        && let Some(is) = node.as_text_input()
    {
        let ime_area = is.editor.ime_cursor_area();
        set_ime_cursor_area(handle, &meta, &ime_area, scroll_offset_x, scroll_offset_y);
    }
}
