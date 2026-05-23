use bitflags::bitflags;
use serde::Serialize;
use winit::keyboard::{Key, NamedKey};

use crate::clipboard::ClipboardBridge;
use crate::input::{KeyResult, input_align_offset};
use crate::layout::TaffyLayoutExt;
use crate::node::{ScrollAxis, UzNodeId};
use crate::selection::{Affinity, SelectionEndpoint, TextSelection};
use crate::style::TextStyle;
use crate::text::{apply_text_style_to_editor, secure_cursor_geometry};
use crate::ui::{DragMode, ScrollDragState, ScrollWheelTarget, UIState};
use crate::window::Window;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct KeyModifiers: u32 {
        const CTRL  = 1 << 0;
        const ALT   = 1 << 1;
        const SHIFT = 1 << 2;
        const SUPER = 1 << 3;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct MouseButtons: u8 {
        const LEFT   = 1 << 0;
        const RIGHT  = 1 << 1;
        const MIDDLE = 1 << 2;
    }
}

impl Serialize for KeyModifiers {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u32(self.bits())
    }
}

impl Serialize for MouseButtons {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.bits())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseEventData {
    pub window_id: u32,
    pub node_id: UzNodeId,
    pub x: f32,
    pub y: f32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub button: u8,
    pub buttons: MouseButtons,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyEventData {
    pub window_id: u32,
    pub node_id: Option<UzNodeId>,
    pub key: String,
    pub code: String,
    pub key_code: u32,
    pub modifiers: KeyModifiers,
    pub repeat: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowLoadEventData {
    pub window_id: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeChangedEventData {
    pub window_id: u32,
    pub theme: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeEventData {
    pub window_id: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputEventData {
    pub window_id: u32,
    pub node_id: UzNodeId,
    pub input_type: String,
    pub data: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusEventData {
    pub window_id: u32,
    pub node_id: UzNodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEventData {
    pub window_id: u32,
    pub node_id: Option<UzNodeId>,
    pub selection_text: Option<String>,
    pub clipboard_text: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AppEvent {
    // move all this to UzUIEvent ?
    Click(MouseEventData),
    MouseDown(MouseEventData),
    MouseUp(MouseEventData),
    KeyDown(KeyEventData),
    KeyUp(KeyEventData),
    Resize(ResizeEventData),
    Input(InputEventData),
    Focus(FocusEventData),
    Blur(FocusEventData),
    Copy(ClipboardEventData),
    Cut(ClipboardEventData),
    Paste(ClipboardEventData),
    #[serde(rename = "windowLoad")]
    WindowLoad(WindowLoadEventData),
    #[serde(rename = "windowClose")]
    WindowClose(WindowLoadEventData),
    #[serde(rename = "themeChanged")]
    ThemeChanged(ThemeChangedEventData),
    HotReload,
}

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
    Some(FocusedInputLayoutMeta {
        taffy_x: hb.bounds.x,
        taffy_y: hb.bounds.y,
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
fn single_line_align_offset(dom: &mut UIState, handle: &mut Window, nid: UzNodeId) -> f32 {
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

pub fn handle_cursor_moved(
    dom: &mut UIState,
    handle: &mut Window,
    position: winit::dpi::PhysicalPosition<f64>,
    mouse_buttons: MouseButtons,
) -> bool {
    let mut needs_redraw = false;
    let scale = handle.scale_factor();
    let logical_x = position.x / scale;
    let logical_y = position.y / scale;
    // Burst-scroll inputs may have left the hit tree stale before this
    // event arrived — refresh against current scroll state so the cursor
    // hits what the user actually sees.
    dom.ensure_hit_tree_fresh(&mut handle.text_renderer, scale);
    let old_top = dom.hit_state.top_node;
    dom.update_hit_test(logical_x, logical_y);
    if old_top != dom.hit_state.top_node {
        needs_redraw = true;
    }

    // Scroll thumb drag
    if let Some(drag) = dom.drag_mode.as_scrollbar_thumb() {
        let mouse_pos = match drag.axis {
            ScrollAxis::Y => logical_y,
            ScrollAxis::X => logical_x,
        };
        let delta = mouse_pos - drag.start_mouse_pos;
        let new_offset = if drag.track_range > 0.0 {
            drag.start_scroll_offset + (delta as f32 / drag.track_range as f32) * drag.max_scroll
        } else {
            drag.start_scroll_offset
        };
        let nid = drag.node_id;
        let axis = drag.axis;
        let clamped = new_offset.clamp(0.0, drag.max_scroll);
        if let Some(node) = dom.nodes.get_mut(nid) {
            node.scroll_state.set_offset(axis, clamped);
        }
        dom.hit_tree_dirty = true;
        needs_redraw = true;
    }

    // Input drag selection
    if mouse_buttons.contains(MouseButtons::LEFT) {
        if let DragMode::InputSelection(drag_nid) = dom.drag_mode {
            let hit_info = dom.nodes.get(drag_nid).and_then(|node| {
                let is = node.as_text_input()?;
                let scroll_offset_x = node.scroll_state.scroll_offset_x;
                let scroll_offset_y = node.scroll_state.scroll_offset_y;
                let content_box = node.final_layout.content_box_bounds();
                let hb = node
                    .hitbox_id
                    .and_then(|hid| dom.hitbox_store.get(hid))?
                    .bounds;
                Some((
                    scroll_offset_x,
                    scroll_offset_y,
                    is.multiline,
                    content_box.x,
                    content_box.y,
                    hb,
                ))
            });

            if let Some((scroll_offset, scroll_offset_y, is_multiline, content_x, content_y, hb)) =
                hit_info
            {
                // Apply styles/width so the driver's layout accounts for
                // wrapping; also gives us a fresh natural width for align_offset.
                if let Some(meta) = input_layout_meta(dom, drag_nid)
                    && let Some(node) = dom.nodes.get_mut(drag_nid)
                    && let Some(is) = node.as_text_input_mut()
                {
                    apply_text_style_to_editor(&mut is.editor, &meta.text_style);
                    is.editor.set_width(if meta.multiline {
                        Some(meta.input_width)
                    } else {
                        None
                    });
                }

                let align_offset = if is_multiline {
                    0.0
                } else {
                    single_line_align_offset(dom, handle, drag_nid)
                };
                let relative_x = if is_multiline {
                    (logical_x - hb.x - content_x) as f32
                } else {
                    (logical_x - hb.x - content_x) as f32 + scroll_offset - align_offset
                };
                let relative_y = (logical_y - hb.y - content_y) as f32 + scroll_offset_y;

                if let Some(node) = dom.nodes.get_mut(drag_nid)
                    && let Some(is) = node.as_text_input_mut()
                {
                    is.extend_selection_to_point(relative_x, relative_y, &mut handle.text_renderer);
                }

                scroll_input_to_cursor(dom, handle);
                needs_redraw = true;
            }
        }

        // View text selection drag
        if let DragMode::ViewSelection(root_id) = dom.drag_mode
            && let Some(hit) = hit_text_in_run(
                dom,
                &mut handle.text_renderer,
                root_id,
                logical_x,
                logical_y,
            )
        {
            if let Some(selection) = dom.get_text_selection()
                && dom.selection_root(&selection) == Some(root_id)
                && let Some(anchor) = selection.anchor
            {
                dom.set_selection(TextSelection::new(anchor, hit.endpoint));
            }
            needs_redraw = true;
        }
    }

    let cursor = dom
        .hit_state
        .top_node
        .map(|id| dom.resolve_cursor(id))
        .unwrap_or(crate::cursor::UzCursorIcon::Default);
    handle.set_cursor(cursor);

    needs_redraw
}

/// Hit-test a mouse position against all text nodes in a textSelect run.
/// Returns the matched text node and flat grapheme index if a suitable text node is found.
struct TextRunHit {
    node_id: UzNodeId,
    endpoint: SelectionEndpoint,
}

fn hit_text_in_run(
    dom: &UIState,
    text_renderer: &mut crate::text::TextRenderer,
    root_id: UzNodeId,
    mx: f64,
    my: f64,
) -> Option<TextRunHit> {
    use crate::style::Bounds;

    let run = dom
        .selectable_text_runs
        .iter()
        .find(|r| r.root_id == root_id)?;

    let mut best: Option<(UzNodeId, f64, Bounds)> = None;
    for entry in &run.entries {
        let node = dom.nodes.get(entry.layout_node_id)?;
        let hid = node.hitbox_id?;
        let hb = dom.hitbox_store.get(hid)?;
        let dist = point_to_rect_dist(mx, my, &hb.bounds);
        if best.is_none() || dist < best.unwrap().1 {
            best = Some((entry.layout_node_id, dist, hb.bounds));
        }
    }

    let (layout_node_id, _, bounds) = best?;
    let node = dom.nodes.get(layout_node_id)?;
    let text_len = node
        .as_element()
        .and_then(|element| element.inline_layout.as_ref())
        .map(|inline| inline.text_len)
        .or_else(|| node.get_text_content().map(|text| text.content.len()))?;

    if text_len == 0 {
        let entry = run
            .entries
            .iter()
            .find(|entry| entry.layout_node_id == layout_node_id)?;
        return Some(TextRunHit {
            node_id: entry.node_id,
            endpoint: SelectionEndpoint::new(entry.node_id, 0, Affinity::Downstream),
        });
    }

    let content_box = node.final_layout.content_box_bounds();
    let relative_x = (mx - bounds.x - content_box.x) as f32;
    let relative_y = (my - bounds.y - content_box.y) as f32;
    let (global_offset, affinity) = if let Some(layout) = node
        .as_element()
        .and_then(|element| element.inline_layout.as_ref())
        .map(|inline| &inline.layout)
    {
        crate::text::hit_to_text_position_from_layout(layout, text_len, relative_x, relative_y)
    } else {
        let text = node.get_text_content()?;
        text_renderer.hit_to_text_position(
            &text.content,
            &node.computed_style().text,
            Some(content_box.width as f32),
            relative_x,
            relative_y,
        )
    };

    let entry = run
        .entries
        .iter()
        .find(|entry| {
            entry.layout_node_id == layout_node_id
                && global_offset >= entry.flat_byte_start
                && global_offset <= entry.flat_byte_start + entry.byte_len
        })
        .or_else(|| {
            run.entries
                .iter()
                .find(|entry| entry.layout_node_id == layout_node_id)
        })?;
    let offset = global_offset
        .saturating_sub(entry.flat_byte_start)
        .min(entry.byte_len);

    Some(TextRunHit {
        node_id: entry.node_id,
        endpoint: SelectionEndpoint::new(entry.node_id, offset, affinity),
    })
}

fn point_to_rect_dist(px: f64, py: f64, bounds: &crate::style::Bounds) -> f64 {
    let cx = px.clamp(bounds.x, bounds.x + bounds.width);
    let cy = py.clamp(bounds.y, bounds.y + bounds.height);
    let dx = px - cx;
    let dy = py - cy;
    (dx * dx + dy * dy).sqrt()
}

fn text_range_at_point(
    dom: &UIState,
    text_renderer: &mut crate::text::TextRenderer,
    node_id: UzNodeId,
    mx: f64,
    my: f64,
    select_line: bool,
) -> Option<(SelectionEndpoint, SelectionEndpoint)> {
    let (run, entry) = dom.find_run_entry_for_node(node_id)?;
    let layout_node = dom.nodes.get(entry.layout_node_id)?;
    let text_len = layout_node
        .as_element()
        .and_then(|element| element.inline_layout.as_ref())
        .map(|inline| inline.text_len)
        .or_else(|| {
            layout_node
                .get_text_content()
                .map(|text| text.content.len())
        })?;
    let bounds = layout_node
        .hitbox_id
        .and_then(|hid| dom.hitbox_store.get(hid))
        .map(|hb| hb.bounds)?;

    if text_len == 0 {
        let endpoint = SelectionEndpoint::new(node_id, 0, Affinity::Downstream);
        return Some((endpoint, endpoint));
    }

    let content_box = layout_node.final_layout.content_box_bounds();
    let rel_x = (mx - bounds.x - content_box.x) as f32;
    let rel_y = (my - bounds.y - content_box.y) as f32;
    let (global_start, global_end) = if let Some(layout) = layout_node
        .as_element()
        .and_then(|element| element.inline_layout.as_ref())
        .map(|inline| &inline.layout)
    {
        if select_line {
            crate::text::line_byte_range_at_point_from_layout(layout, text_len, rel_x, rel_y)
        } else {
            crate::text::word_byte_range_at_point_from_layout(layout, text_len, rel_x, rel_y)
        }
    } else if select_line {
        let text = layout_node.get_text_content()?;
        text_renderer.line_byte_range_at_point(
            &text.content,
            &layout_node.computed_style().text,
            Some(content_box.width as f32),
            rel_x,
            rel_y,
        )
    } else {
        let text = layout_node.get_text_content()?;
        text_renderer.word_byte_range_at_point(
            &text.content,
            &layout_node.computed_style().text,
            Some(content_box.width as f32),
            rel_x,
            rel_y,
        )
    };

    let start = endpoint_for_layout_byte(
        run,
        entry.layout_node_id,
        global_start,
        Affinity::Downstream,
    )?;
    let end = endpoint_for_layout_byte(run, entry.layout_node_id, global_end, Affinity::Upstream)?;
    Some((start, end))
}

fn endpoint_for_layout_byte(
    run: &crate::element::TextSelectRun,
    layout_node_id: UzNodeId,
    byte: usize,
    affinity: Affinity,
) -> Option<SelectionEndpoint> {
    let entry = run
        .entries
        .iter()
        .find(|entry| {
            entry.layout_node_id == layout_node_id
                && byte >= entry.flat_byte_start
                && byte <= entry.flat_byte_start + entry.byte_len
        })
        .or_else(|| {
            run.entries
                .iter()
                .find(|entry| entry.layout_node_id == layout_node_id)
        })?;
    Some(SelectionEndpoint::new(
        entry.node_id,
        byte.saturating_sub(entry.flat_byte_start)
            .min(entry.byte_len),
        affinity,
    ))
}

pub fn handle_mouse_input(
    dom: &mut UIState,
    handle: &mut Window,
    wid: u32,
    btn_state: winit::event::ElementState,
    button: winit::event::MouseButton,
    mouse_buttons: MouseButtons,
) -> (bool, Vec<AppEvent>) {
    use winit::event::ElementState;

    // Defensive: a programmatic scroll or other mutation since the last
    // input event may have flagged the hit tree dirty. Refresh before
    // dispatching so clicks land where the user sees them.
    let scale = handle.scale_factor();
    dom.ensure_hit_tree_fresh(&mut handle.text_renderer, scale);
    if let Some((mx, my)) = dom.hit_state.mouse_position {
        dom.update_hit_test(mx, my);
    }

    let mut needs_redraw = false;
    let mut events: Vec<AppEvent> = Vec::new();

    let button_num: u8 = match button {
        winit::event::MouseButton::Left => 0,
        winit::event::MouseButton::Middle => 1,
        winit::event::MouseButton::Right => 2,
        _ => 0,
    };

    let Some((mx, my)) = dom.hit_state.mouse_position else {
        return (needs_redraw, events);
    };
    let x = mx as f32;
    let y = my as f32;

    // Check scroll thumb click (left button press)
    if btn_state == ElementState::Pressed && button == winit::event::MouseButton::Left {
        let thumb_hit = dom
            .scroll_thumbs
            .iter()
            .rev()
            .find(|t| t.thumb_bounds.contains(mx, my));
        if let Some(t) = thumb_hit {
            let nid = t.node_id;
            let axis = t.axis;
            let visible = t.visible_size as f64;
            let content = t.content_size as f64;
            let max_scroll = (t.content_size - t.visible_size).max(0.0);
            let track = match axis {
                ScrollAxis::Y => t.view_bounds.height,
                ScrollAxis::X => t.view_bounds.width,
            };
            let thumb_length = (track * visible / content.max(1.0)).max(24.0);
            let track_range = (track - thumb_length).max(0.0);
            let start_mouse_pos = match axis {
                ScrollAxis::Y => my,
                ScrollAxis::X => mx,
            };
            let start_offset = dom
                .nodes
                .get(nid)
                .map(|n| n.scroll_state.offset(axis))
                .unwrap_or(0.0);
            dom.drag_mode = DragMode::ScrollbarThumb(ScrollDragState {
                node_id: nid,
                axis,
                start_mouse_pos,
                start_scroll_offset: start_offset,
                track_range,
                max_scroll,
            });
            return (true, events);
        }
    }

    // End scroll drag on mouse up
    if btn_state == ElementState::Released
        && button == winit::event::MouseButton::Left
        && matches!(dom.drag_mode, DragMode::ScrollbarThumb(_))
    {
        dom.drag_mode = DragMode::None;
    }

    // Resolve topmost hit -> NodeId for JS event target. Active state normally
    // belongs to the hit node; buttons are the special case where a child press
    // should style the owning button.
    let target_node = dom.hit_state.top_node;
    let press_target = target_node.and_then(|nid| dom.nearest_button_ancestor(nid).or(Some(nid)));

    match btn_state {
        ElementState::Pressed => {
            dom.set_active(press_target);
            if let Some(target) = target_node {
                events.push(AppEvent::MouseDown(MouseEventData {
                    window_id: wid,
                    node_id: target,
                    x,
                    y,
                    screen_x: x,
                    screen_y: y,
                    button: button_num,
                    buttons: mouse_buttons,
                }));
            }

            // Input focus handling (left button)
            if button == winit::event::MouseButton::Left {
                let input_target = target_node
                    .filter(|&nid| dom.nodes.get(nid).is_some_and(|n| n.is_text_input()));

                let old_focus = dom.focused_node;

                if let Some(nid) = input_target {
                    // Multi-click detection (double=word, triple=line, quad=select all)
                    let now = std::time::Instant::now();
                    let is_consecutive = dom.last_click_node == Some(nid)
                        && dom
                            .last_click_time
                            .is_some_and(|t| now.duration_since(t).as_millis() < 400);
                    dom.last_click_time = Some(now);
                    dom.last_click_node = Some(nid);
                    if is_consecutive {
                        dom.click_count = (dom.click_count + 1).min(4);
                    } else {
                        dom.click_count = 1;
                    }

                    // Focus if not already focused
                    if old_focus != Some(nid) {
                        if let Some(old_id) = old_focus {
                            events.push(AppEvent::Blur(FocusEventData {
                                window_id: wid,
                                node_id: old_id,
                            }));
                        }
                        events.push(AppEvent::Focus(FocusEventData {
                            window_id: wid,
                            node_id: nid,
                        }));
                    }

                    // Place cursor at click position
                    let click_info = {
                        let node = &dom.nodes[nid];
                        let is = node.as_text_input().unwrap();
                        let scroll_offset_x = node.scroll_state.scroll_offset_x;
                        let scroll_offset_y = node.scroll_state.scroll_offset_y;
                        let content_box = node.final_layout.content_box_bounds();
                        let hb = node
                            .hitbox_id
                            .and_then(|hid| dom.hitbox_store.get(hid))
                            .map(|hb| hb.bounds);
                        (
                            scroll_offset_x,
                            scroll_offset_y,
                            is.multiline,
                            content_box.x,
                            content_box.y,
                            hb,
                        )
                    };
                    let (
                        scroll_offset,
                        scroll_offset_y,
                        is_multiline,
                        content_x,
                        content_y,
                        hitbox_bounds,
                    ) = click_info;

                    if let Some(hb) = hitbox_bounds {
                        dom.focus_element(nid);

                        // Apply styles/width so hit-testing accounts for wrapping
                        if let Some(meta) = input_layout_meta(dom, nid)
                            && let Some(node) = dom.nodes.get_mut(nid)
                            && let Some(is) = node.as_text_input_mut()
                        {
                            apply_text_style_to_editor(&mut is.editor, &meta.text_style);
                            is.editor.set_width(if meta.multiline {
                                Some(meta.input_width)
                            } else {
                                None
                            });
                        }

                        let align_offset = if is_multiline {
                            0.0
                        } else {
                            single_line_align_offset(dom, handle, nid)
                        };
                        let relative_x = if is_multiline {
                            (mx - hb.x - content_x) as f32
                        } else {
                            (mx - hb.x - content_x) as f32 + scroll_offset - align_offset
                        };
                        let relative_y = (my - hb.y - content_y) as f32 + scroll_offset_y;

                        if let Some(node) = dom.nodes.get_mut(nid)
                            && let Some(is) = node.as_text_input_mut()
                        {
                            let renderer = &mut handle.text_renderer;
                            match dom.click_count {
                                2 => is.select_word_at_point(relative_x, relative_y, renderer),
                                3 => is.select_line_at_point(relative_x, relative_y, renderer),
                                4 => is.select_all(renderer),
                                _ => is.move_to_point(relative_x, relative_y, renderer),
                            }
                            is.reset_blink();
                        }
                    }

                    scroll_input_to_cursor(dom, handle);
                    dom.drag_mode = DragMode::InputSelection(nid);
                } else {
                    // Clicked non-input: blur focused input
                    if let Some(old_id) = old_focus {
                        dom.focused_node = None;
                        events.push(AppEvent::Blur(FocusEventData {
                            window_id: wid,
                            node_id: old_id,
                        }));
                    }

                    // Selection starts if the click landed anywhere inside a
                    // text-selectable scope — on a text node, on the
                    // container itself, or on any non-text descendant. This
                    // matches browser behaviour where clicking padding/empty
                    // space inside a `<p>` begins selection.
                    let run_root_for_click =
                        target_node.and_then(|nid| dom.containing_text_run_root(nid));

                    if let Some(run_root) = run_root_for_click {
                        let nid = target_node.unwrap();

                        // Starting a view selection blurs any focused input
                        if let Some(old_id) = dom.focused_node.take() {
                            events.push(AppEvent::Blur(FocusEventData {
                                window_id: wid,
                                node_id: old_id,
                            }));
                        }

                        if let Some(hit) =
                            hit_text_in_run(dom, &mut handle.text_renderer, run_root, mx, my)
                        {
                            let endpoint = hit.endpoint;

                            // Multi-click detection
                            let now = std::time::Instant::now();
                            let is_consecutive = dom.last_click_node == Some(nid)
                                && dom
                                    .last_click_time
                                    .is_some_and(|t| now.duration_since(t).as_millis() < 400);
                            dom.last_click_time = Some(now);
                            dom.last_click_node = Some(nid);
                            if is_consecutive {
                                dom.click_count = (dom.click_count + 1).min(4);
                            } else {
                                dom.click_count = 1;
                            }

                            match dom.click_count {
                                2 => {
                                    if let Some((start, end)) = text_range_at_point(
                                        dom,
                                        &mut handle.text_renderer,
                                        hit.node_id,
                                        mx,
                                        my,
                                        false,
                                    ) {
                                        dom.set_selection(TextSelection::new(start, end));
                                    }
                                }
                                3 => {
                                    if let Some((start, end)) = text_range_at_point(
                                        dom,
                                        &mut handle.text_renderer,
                                        hit.node_id,
                                        mx,
                                        my,
                                        true,
                                    ) {
                                        dom.set_selection(TextSelection::new(start, end));
                                    }
                                }
                                4 => {
                                    // Select all text in the run
                                    if let Some(run) = dom
                                        .selectable_text_runs
                                        .iter()
                                        .find(|r| r.root_id == run_root)
                                        && let (Some(start), Some(end)) = (
                                            dom.endpoint_from_flat_index(
                                                run_root,
                                                0,
                                                Affinity::Downstream,
                                            ),
                                            dom.endpoint_from_flat_index(
                                                run_root,
                                                run.total_graphemes,
                                                Affinity::Upstream,
                                            ),
                                        )
                                    {
                                        dom.set_selection(TextSelection::new(start, end));
                                    }
                                }
                                _ => {
                                    // Single click: place cursor
                                    dom.set_selection(TextSelection::new(endpoint, endpoint));
                                }
                            }
                            dom.drag_mode = DragMode::ViewSelection(run_root);
                        }
                    } else {
                        // Clicked on non-selectable area: clear view selection
                        dom.clear_selection();
                    }
                }
            }

            needs_redraw = true;
        }
        ElementState::Released => {
            if let Some(target) = target_node {
                events.push(AppEvent::MouseUp(MouseEventData {
                    window_id: wid,
                    node_id: target,
                    x,
                    y,
                    screen_x: x,
                    screen_y: y,
                    button: button_num,
                    buttons: mouse_buttons,
                }));
            }
            // Click fires if released on the same element that was pressed
            if let Some(active) = dom.hit_state.active_node
                && dom.hit_state.is_hovered(active)
            {
                if button == winit::event::MouseButton::Left
                    && let Some(node) = dom.nodes.get_mut(active)
                    && let Some(checked) = node.as_checkbox_input_mut()
                {
                    *checked = !*checked;
                    events.push(AppEvent::Input(InputEventData {
                        window_id: wid,
                        node_id: active,
                        input_type: "toggle".to_string(),
                        data: None,
                    }));
                }
                if let Some(target) = target_node {
                    events.push(AppEvent::Click(MouseEventData {
                        window_id: wid,
                        node_id: target,
                        x,
                        y,
                        screen_x: x,
                        screen_y: y,
                        button: button_num,
                        buttons: mouse_buttons,
                    }));
                }
            }
            dom.set_active(None);
            if matches!(
                dom.drag_mode,
                DragMode::InputSelection(_) | DragMode::ViewSelection(_)
            ) {
                dom.drag_mode = DragMode::None;
            }
            needs_redraw = true;
        }
    }

    (needs_redraw, events)
}

/// Build the raw KeyDown/KeyUp event. Returns None for F5 (hot reload) or unmappable keys.
pub fn build_key_event(
    dom: &UIState,
    wid: u32,
    key_event: &winit::event::KeyEvent,
    modifiers: KeyModifiers,
) -> Option<AppEvent> {
    use winit::event::ElementState;
    use winit::keyboard::PhysicalKey;

    // F5 hot reload
    if key_event.state == ElementState::Pressed && key_event.logical_key == Key::Named(NamedKey::F5)
    {
        return Some(AppEvent::HotReload);
    }

    let key_str = match &key_event.logical_key {
        Key::Character(c) => c.to_string(),
        Key::Named(named) => format!("{:?}", named),
        _ => return None,
    };

    let code_str = match key_event.physical_key {
        PhysicalKey::Code(kc) => format!("{:?}", kc),
        _ => String::new(),
    };

    let data = KeyEventData {
        window_id: wid,
        node_id: dom.focused_node,
        key: key_str,
        code: code_str,
        key_code: 0,
        modifiers,
        repeat: key_event.repeat,
    };

    Some(match key_event.state {
        ElementState::Pressed => AppEvent::KeyDown(data),
        ElementState::Released => AppEvent::KeyUp(data),
    })
}

/// Handle keyboard input for the focused input element. Called AFTER the raw key
/// event has been dispatched to JS (so preventDefault can suppress this).
/// Returns (needs_redraw, events_to_dispatch).
pub fn handle_key_for_input(
    dom: &mut UIState,
    handle: &mut Window,
    wid: u32,
    key_event: &winit::event::KeyEvent,
    modifiers: KeyModifiers,
) -> (bool, Vec<AppEvent>) {
    use winit::event::ElementState;

    let mut needs_redraw = false;
    let mut events: Vec<AppEvent> = Vec::new();

    if key_event.state != ElementState::Pressed {
        return (needs_redraw, events);
    }

    // Apply text styles and width to the editor BEFORE handling the key,
    // so parley's driver has the correct layout for cursor movement in wrapped text.
    if let Some(meta) = dom.focused_node.and_then(|id| input_layout_meta(dom, id))
        && let Some(node) = dom.focused_node.and_then(|id| dom.nodes.get_mut(id))
        && let Some(is) = node.as_text_input_mut()
    {
        apply_text_style_to_editor(&mut is.editor, &meta.text_style);
        is.editor.set_width(if meta.multiline {
            Some(meta.input_width)
        } else {
            None
        });
    }

    let new_focus = dom
        .with_focused_node(|node, focused_id| {
            let mut new_focus = Some(focused_id);

            if let Some(input_state) = node.as_text_input_mut() {
                let result = input_state.handle_key(
                    &key_event.logical_key,
                    modifiers,
                    &mut handle.text_renderer,
                );
                match result {
                    KeyResult::Edit(edit) => {
                        let input_type = edit.kind.input_type();
                        events.push(AppEvent::Input(InputEventData {
                            window_id: wid,
                            node_id: focused_id,
                            input_type: input_type.to_string(),
                            data: edit.inserted,
                        }));
                        needs_redraw = true;
                    }
                    KeyResult::Blur => {
                        new_focus = None;
                        events.push(AppEvent::Blur(FocusEventData {
                            window_id: wid,
                            node_id: focused_id,
                        }));
                        needs_redraw = true;
                    }
                    KeyResult::Handled => {
                        needs_redraw = true;
                    }
                    KeyResult::Ignored => {}
                }
            }
            new_focus
        })
        .flatten();

    dom.focused_node = new_focus;

    if needs_redraw {
        scroll_input_to_cursor(dom, handle);
    }

    (needs_redraw, events)
}

pub fn handle_key_for_checkbox(
    dom: &mut UIState,
    wid: u32,
    key_event: &winit::event::KeyEvent,
) -> (bool, Vec<AppEvent>) {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return (false, Vec::new());
    }

    let should_toggle = matches!(
        &key_event.logical_key,
        Key::Named(NamedKey::Space) | Key::Named(NamedKey::Enter)
    );
    if !should_toggle {
        return (false, Vec::new());
    }

    let Some(focused_id) = dom.focused_node else {
        return (false, Vec::new());
    };
    let Some(node) = dom.nodes.get_mut(focused_id) else {
        return (false, Vec::new());
    };
    let Some(checked) = node.as_checkbox_input_mut() else {
        return (false, Vec::new());
    };

    *checked = !*checked;
    (
        true,
        vec![AppEvent::Input(InputEventData {
            window_id: wid,
            node_id: focused_id,
            input_type: "toggle".to_string(),
            data: None,
        })],
    )
}

/// Handle Enter/Space on a focused button element. Fires a synthetic click,
/// mirroring browser behavior on `<button>`.
pub fn handle_key_for_button(
    dom: &mut UIState,
    wid: u32,
    key_event: &winit::event::KeyEvent,
) -> (bool, Vec<AppEvent>) {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return (false, Vec::new());
    }
    if !matches!(
        &key_event.logical_key,
        Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space)
    ) {
        return (false, Vec::new());
    }

    let Some(focused_id) = dom.focused_node else {
        return (false, Vec::new());
    };
    let Some(node) = dom.nodes.get(focused_id) else {
        return (false, Vec::new());
    };
    if !node.is_button() {
        return (false, Vec::new());
    }

    // Synthetic click: use the element's bounds center if we have a hitbox,
    // otherwise (0, 0). The JS handler usually doesn't depend on coords for
    // keyboard activations.
    let (x, y) = node
        .hitbox_id
        .and_then(|hid| dom.hitbox_store.get(hid))
        .map(|hb| {
            (
                (hb.bounds.x + hb.bounds.width / 2.0) as f32,
                (hb.bounds.y + hb.bounds.height / 2.0) as f32,
            )
        })
        .unwrap_or((0.0, 0.0));

    (
        true,
        vec![AppEvent::Click(MouseEventData {
            window_id: wid,
            node_id: focused_id,
            x,
            y,
            screen_x: x,
            screen_y: y,
            button: 0,
            buttons: MouseButtons::empty(),
        })],
    )
}

pub struct TabFocusOutcome {
    pub consumed: bool,
    pub needs_redraw: bool,
    pub events: Vec<AppEvent>,
}

/// Handle Tab/Shift-Tab to advance focus to the next/previous focusable
/// element. Tab is always consumed (never inserts a tab character).
pub fn handle_tab_focus(
    dom: &mut UIState,
    wid: u32,
    key_event: &winit::event::KeyEvent,
    modifiers: KeyModifiers,
) -> TabFocusOutcome {
    use winit::event::ElementState;

    let mut outcome = TabFocusOutcome {
        consumed: false,
        needs_redraw: false,
        events: Vec::new(),
    };

    if key_event.state != ElementState::Pressed
        || !matches!(&key_event.logical_key, Key::Named(NamedKey::Tab))
    {
        return outcome;
    }

    outcome.consumed = true;

    let shift = modifiers.contains(KeyModifiers::SHIFT);
    let change = if shift {
        dom.focus_prev_node()
    } else {
        dom.focus_next_node()
    };
    if let Some(change) = change {
        if let Some(old) = change.old {
            outcome.events.push(AppEvent::Blur(FocusEventData {
                window_id: wid,
                node_id: old,
            }));
        }
        outcome.events.push(AppEvent::Focus(FocusEventData {
            window_id: wid,
            node_id: change.new,
        }));

        dom.request_scroll_focus_into_view(change.new);

        outcome.needs_redraw = true;
    }

    outcome
}

/// Handle keyboard shortcuts for view text selection (Shift+Arrows, Ctrl+A, etc.)
/// Called after input-level processing, only when there's no focused input.
/// Returns true if a redraw is needed.
pub fn handle_key_for_view_selection(
    dom: &mut UIState,
    key_event: &winit::event::KeyEvent,
    modifiers: KeyModifiers,
) -> bool {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return false;
    }

    let Some(sel) = dom.get_text_selection() else {
        return false;
    };

    let Some(root) = dom.selection_root(&sel) else {
        return false;
    };
    let Some(anchor_endpoint) = sel.anchor else {
        return false;
    };
    let Some(focus_endpoint) = sel.focus else {
        return false;
    };
    let Some(active) = dom.flat_index_for_endpoint(focus_endpoint) else {
        return false;
    };

    let run_len = dom
        .selectable_text_runs
        .iter()
        .find(|r| r.root_id == root)
        .map(|r| r.total_graphemes)
        .unwrap_or(0);

    if run_len == 0 {
        return false;
    }

    let shift = modifiers.contains(KeyModifiers::SHIFT);
    let ctrl = modifiers.contains(KeyModifiers::CTRL);

    match &key_event.logical_key {
        Key::Named(NamedKey::ArrowLeft) if shift && ctrl => {
            // Move active to previous word boundary
            let new_active = dom.prev_word_boundary_in_run(root, active);
            if let Some(focus) =
                dom.endpoint_from_flat_index(root, new_active, Affinity::Downstream)
            {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Named(NamedKey::ArrowRight) if shift && ctrl => {
            let new_active = dom.next_word_boundary_in_run(root, active);
            if let Some(focus) =
                dom.endpoint_from_flat_index(root, new_active, Affinity::Downstream)
            {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Named(NamedKey::ArrowLeft) if shift => {
            let new_active = if active > 0 { active - 1 } else { 0 };
            if let Some(focus) =
                dom.endpoint_from_flat_index(root, new_active, Affinity::Downstream)
            {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Named(NamedKey::ArrowRight) if shift => {
            let new_active = (active + 1).min(run_len);
            if let Some(focus) =
                dom.endpoint_from_flat_index(root, new_active, Affinity::Downstream)
            {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Named(NamedKey::Home) if shift => {
            if let Some(focus) = dom.endpoint_from_flat_index(root, 0, Affinity::Downstream) {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Named(NamedKey::End) if shift => {
            if let Some(focus) = dom.endpoint_from_flat_index(root, run_len, Affinity::Upstream) {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Character(c) if ctrl && (c.as_ref() == "a" || c.as_ref() == "A") => {
            if let (Some(start), Some(end)) = (
                dom.endpoint_from_flat_index(root, 0, Affinity::Downstream),
                dom.endpoint_from_flat_index(root, run_len, Affinity::Upstream),
            ) {
                dom.set_selection(TextSelection::new(start, end));
            }
            true
        }
        _ => false,
    }
}

/// Identifies the target of a clipboard operation.
pub enum ClipboardTarget {
    /// Focused input node.
    Input(UzNodeId),
    /// Non-input text selection root.
    ViewSelection(UzNodeId),
}

/// A resolved clipboard command ready for event dispatch and default action.
pub enum ClipboardCommand {
    Copy {
        target: Option<UzNodeId>,
        selection_text: String,
    },
    Cut {
        target: Option<UzNodeId>,
        selection_text: String,
        is_input: bool,
    },
    Paste {
        target: Option<UzNodeId>,
        clipboard_text: Option<String>,
        is_input: bool,
    },
}

/// Resolve the current clipboard target from DOM state.
fn resolve_clipboard_target(dom: &UIState) -> Option<ClipboardTarget> {
    if let Some(focused_id) = dom.focused_node
        && let Some(node) = dom.nodes.get(focused_id)
        && node.as_text_input().is_some()
    {
        return Some(ClipboardTarget::Input(focused_id));
    }
    if let Some(sel) = dom.get_text_selection()
        && !sel.is_collapsed()
        && let Some(root) = dom.selection_root(&sel)
    {
        return Some(ClipboardTarget::ViewSelection(root));
    }
    None
}

/// Detect whether a key event is a clipboard shortcut and build the corresponding
/// command. Returns `None` if the key is not a clipboard shortcut.
pub fn build_clipboard_command(
    dom: &UIState,
    key_event: &winit::event::KeyEvent,
    modifiers: KeyModifiers,
    clipboard: &ClipboardBridge<'_>,
) -> Option<ClipboardCommand> {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return None;
    }

    let ctrl = modifiers.contains(KeyModifiers::CTRL);
    if !ctrl {
        return None;
    }

    let ch = match &key_event.logical_key {
        Key::Character(c) => c.as_ref(),
        _ => return None,
    };

    match ch {
        "c" | "C" => {
            let target = resolve_clipboard_target(dom);
            let selection_text = match &target {
                Some(ClipboardTarget::Input(nid)) => {
                    let node = dom.nodes.get(*nid)?;
                    let is = node.as_text_input()?;
                    if is.secure {
                        return None; // Block copy on secure inputs
                    }
                    let text = is.selected_text();
                    if text.is_empty() {
                        return None;
                    }
                    text
                }
                Some(ClipboardTarget::ViewSelection(_)) => {
                    let text = dom.selected_text();
                    if text.is_empty() {
                        return None;
                    }
                    text
                }
                None => return None,
            };
            let target_id = match &target {
                Some(ClipboardTarget::Input(nid)) => Some(*nid),
                Some(ClipboardTarget::ViewSelection(nid)) => Some(*nid),
                None => None,
            };
            Some(ClipboardCommand::Copy {
                target: target_id,
                selection_text,
            })
        }
        "x" | "X" => {
            let target = resolve_clipboard_target(dom);
            let (target_id, is_input) = match &target {
                Some(ClipboardTarget::Input(nid)) => {
                    let node = dom.nodes.get(*nid)?;
                    let is = node.as_text_input()?;
                    if is.secure {
                        return None; // Block cut on secure inputs
                    }
                    (Some(*nid), true)
                }
                Some(ClipboardTarget::ViewSelection(nid)) => (Some(*nid), false),
                None => return None,
            };
            let selection_text = match &target {
                Some(ClipboardTarget::Input(nid)) => {
                    let node = dom.nodes.get(*nid)?;
                    let is = node.as_text_input()?;
                    let text = is.selected_text();
                    if text.is_empty() {
                        return None;
                    }
                    text
                }
                Some(ClipboardTarget::ViewSelection(_)) => {
                    let text = dom.selected_text();
                    if text.is_empty() {
                        return None;
                    }
                    text
                }
                None => return None,
            };
            Some(ClipboardCommand::Cut {
                target: target_id,
                selection_text,
                is_input,
            })
        }
        "v" | "V" => {
            let target = resolve_clipboard_target(dom);
            let (target_id, is_input) = match &target {
                Some(ClipboardTarget::Input(nid)) => (Some(*nid), true),
                Some(ClipboardTarget::ViewSelection(nid)) => (Some(*nid), false),
                None => return None,
            };
            let clipboard_text = clipboard.read_text().unwrap_or(None);
            Some(ClipboardCommand::Paste {
                target: target_id,
                clipboard_text,
                is_input,
            })
        }
        _ => None,
    }
}

/// Build the AppEvent for dispatching a clipboard command to JS.
pub fn clipboard_command_to_event(cmd: &ClipboardCommand, wid: u32) -> AppEvent {
    match cmd {
        ClipboardCommand::Copy {
            target,
            selection_text,
        } => AppEvent::Copy(ClipboardEventData {
            window_id: wid,
            node_id: *target,
            selection_text: Some(selection_text.clone()),
            clipboard_text: None,
        }),
        ClipboardCommand::Cut {
            target,
            selection_text,
            ..
        } => AppEvent::Cut(ClipboardEventData {
            window_id: wid,
            node_id: *target,
            selection_text: Some(selection_text.clone()),
            clipboard_text: None,
        }),
        ClipboardCommand::Paste {
            target,
            clipboard_text,
            ..
        } => AppEvent::Paste(ClipboardEventData {
            window_id: wid,
            node_id: *target,
            selection_text: None,
            clipboard_text: clipboard_text.clone(),
        }),
    }
}

/// Apply the default clipboard action. Returns (needs_redraw, follow_up_events).
pub fn apply_clipboard_command(
    cmd: ClipboardCommand,
    dom: &mut UIState,
    wid: u32,
    clipboard: &ClipboardBridge<'_>,
    text_renderer: &mut crate::text::TextRenderer,
) -> (bool, Vec<AppEvent>) {
    let mut events = Vec::new();
    let mut needs_redraw = false;

    match cmd {
        ClipboardCommand::Copy { selection_text, .. } => {
            if let Err(e) = clipboard.write_text(&selection_text) {
                eprintln!("[uzumaki] clipboard write error: {e}");
            }
        }
        ClipboardCommand::Cut {
            target,
            selection_text,
            is_input,
        } => {
            if let Err(e) = clipboard.write_text(&selection_text) {
                eprintln!("[uzumaki] clipboard write error: {e}");
            }
            if is_input
                && let Some(target_id) = target
                && let Some(node) = dom.nodes.get_mut(target_id)
                && let Some(is) = node.as_text_input_mut()
                && let Some((_cut_text, edit)) = is.cut_selected_text(text_renderer)
            {
                events.push(AppEvent::Input(InputEventData {
                    window_id: wid,
                    node_id: target_id,
                    input_type: edit.kind.input_type().to_string(),
                    data: edit.inserted,
                }));
                needs_redraw = true;
            }
            // For view selections, cut is a no-op on the content
        }
        ClipboardCommand::Paste {
            target,
            clipboard_text,
            is_input,
        } => {
            if is_input
                && let (Some(target_id), Some(text)) = (target, clipboard_text)
                && let Some(node) = dom.nodes.get_mut(target_id)
                && let Some(is) = node.as_text_input_mut()
                && let Some(edit) = is.paste_text(&text, text_renderer)
            {
                events.push(AppEvent::Input(InputEventData {
                    window_id: wid,
                    node_id: target_id,
                    input_type: edit.kind.input_type().to_string(),
                    data: edit.inserted,
                }));
                needs_redraw = true;
            }
            // For view selections, paste is a no-op
        }
    }

    (needs_redraw, events)
}

pub fn handle_mouse_wheel(
    dom: &mut UIState,
    handle: &mut Window,
    scroll_delta_x: f64,
    scroll_delta_y: f64,
) -> bool {
    let Some((mx, my)) = dom.hit_state.mouse_position else {
        return false;
    };

    let mut needs_redraw = false;
    if scroll_delta_y != 0.0 {
        needs_redraw |= apply_wheel_axis(dom, mx, my, ScrollAxis::Y, scroll_delta_y);
    }
    if scroll_delta_x != 0.0 {
        needs_redraw |= apply_wheel_axis(dom, mx, my, ScrollAxis::X, scroll_delta_x);
    }

    if needs_redraw {
        // Rebuild now so subsequent input events in this same frame (or
        // the next, before paint) see post-scroll geometry. The scroll
        // bug was: clicks during a fast wheel burst hit the previous
        // frame's hitboxes because paint hadn't refreshed them yet.
        let scale = handle.scale_factor();
        crate::hit_tree::rebuild(dom, &mut handle.text_renderer, scale);
        // And re-hit-test the cursor so hover/active state matches what
        // the user now sees under the pointer.
        dom.update_hit_test(mx, my);
        update_ime_cursor_area(dom, handle);
    }
    needs_redraw
}

fn apply_wheel_axis(dom: &mut UIState, mx: f64, my: f64, axis: ScrollAxis, delta: f64) -> bool {
    const SCROLL_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(200);

    // Honour the existing wheel capture for momentum/inertia continuity, but
    // only when the captured node is actually scrollable on this axis.
    let locked = dom.wheel_capture.as_ref().and_then(|capture| {
        if capture.axis == axis && capture.started_at.elapsed() < SCROLL_LOCK_TIMEOUT {
            dom.scroll_thumbs.iter().rev().find(|tr| {
                tr.node_id == capture.node_id && tr.axis == axis && tr.view_bounds.contains(mx, my)
            })
        } else {
            None
        }
    });

    let (target, locked_to_target) = if let Some(t) = locked {
        (Some(t.node_id), true)
    } else {
        (
            dom.scroll_thumbs
                .iter()
                .rev()
                .find(|t| t.axis == axis && t.view_bounds.contains(mx, my))
                .map(|t| t.node_id),
            false,
        )
    };

    let Some(mut nid) = target else {
        return false;
    };

    let mut remaining = delta;
    let mut needs_redraw = false;
    let mut capture_node = None;

    loop {
        if let Some(next_remaining) = apply_wheel_delta_to_node(dom, nid, axis, remaining)
            && next_remaining != remaining
        {
            needs_redraw = true;
            capture_node = Some(nid);
            remaining = next_remaining;
            if remaining == 0.0 {
                break;
            }
        }

        // While the wheel is locked to a previously-captured node, refuse
        // to chain into ancestors even if the captured node is saturated.
        // The user must pause wheeling for SCROLL_LOCK_TIMEOUT before the
        // parent can take over
        if locked_to_target {
            capture_node = Some(nid);
            break;
        }

        // Wheel bubbles up the layout tree (matches CSS scroll
        // containment) so an anonymous wrapper between the cursor and a
        // scrollable ancestor doesn't break wheel propagation.
        let Some(parent) = dom.nodes.get(nid).and_then(|node| node.layout_parent) else {
            break;
        };
        nid = parent;
    }

    if let Some(node_id) = capture_node {
        dom.wheel_capture = Some(ScrollWheelTarget {
            node_id,
            axis,
            started_at: std::time::Instant::now(),
        });
    }

    needs_redraw
}

fn apply_wheel_delta_to_node(
    dom: &mut UIState,
    node_id: UzNodeId,
    axis: ScrollAxis,
    delta: f64,
) -> Option<f64> {
    let thumb = dom
        .scroll_thumbs
        .iter()
        .find(|t| t.node_id == node_id && t.axis == axis)?;
    let max_scroll = (thumb.content_size - thumb.visible_size).max(0.0);
    let node = dom.nodes.get_mut(node_id)?;

    let cur = node.scroll_state.offset(axis);
    let next = (cur - delta as f32).clamp(0.0, max_scroll);
    let actual_change = next - cur;
    node.scroll_state.set_offset(axis, next);
    Some(delta + actual_change as f64)
}
