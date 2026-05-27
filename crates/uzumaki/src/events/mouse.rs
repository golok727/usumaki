use std::collections::HashSet;

use crate::layout::TaffyLayoutExt;
use crate::node::{ScrollAxis, UzNodeId};
use crate::selection::{Affinity, SelectionEndpoint, TextSelection};
use crate::text::apply_text_style_to_editor;
use crate::ui::{DragMode, ScrollDragState, UIState};
use crate::window::Window;

use super::app_event::AppEvent;
use super::text_input::{input_layout_meta, scroll_input_to_cursor, single_line_align_offset};
use super::types::{MouseButtons, UzFocusEvent, UzInputEvent, UzMouseEvent};

/// Cursor position relative to a node's top-left hitbox corner. Falls back to
/// the window-relative coords when the node has no hitbox.
pub(crate) fn local_offset(dom: &UIState, node_id: UzNodeId, x: f32, y: f32) -> (f32, f32) {
    dom.nodes
        .get(node_id)
        .and_then(|n| n.hitbox_id)
        .and_then(|hid| dom.hitbox_store.get(hid))
        .map(|hb| (x - hb.bounds.x as f32, y - hb.bounds.y as f32))
        .unwrap_or((x, y))
}

/// Node and its ancestors, innermost first, walking the parent chain to the root.
fn ancestor_path(dom: &UIState, node_id: UzNodeId) -> Vec<UzNodeId> {
    let mut path = Vec::new();
    let mut current = Some(node_id);
    while let Some(id) = current {
        path.push(id);
        current = dom.nodes.get(id).and_then(|n| n.parent);
    }
    path
}

fn hover_mouse_event(
    dom: &UIState,
    wid: u32,
    node_id: UzNodeId,
    x: f32,
    y: f32,
    related: Option<UzNodeId>,
    buttons: MouseButtons,
) -> UzMouseEvent {
    let (local_x, local_y) = local_offset(dom, node_id, x, y);
    UzMouseEvent {
        window_id: wid,
        node_id,
        x,
        y,
        local_x,
        local_y,
        screen_x: x,
        screen_y: y,
        button: 0,
        buttons,
        related_node_id: related,
    }
}

/// Emit mouseout/mouseover (bubbling) and mouseleave/mouseenter (per element)
/// for a hover transition from `prev` to `current`. Each event carries
/// coordinates relative to its own target, since Rust owns the node bounds.
fn hover_transition_events(
    dom: &UIState,
    wid: u32,
    prev: Option<UzNodeId>,
    current: Option<UzNodeId>,
    x: f32,
    y: f32,
    buttons: MouseButtons,
) -> Vec<AppEvent> {
    let mut events = Vec::new();
    if prev == current {
        return events;
    }

    let prev_path = prev.map(|id| ancestor_path(dom, id)).unwrap_or_default();
    let target_path = current.map(|id| ancestor_path(dom, id)).unwrap_or_default();
    let prev_set: HashSet<UzNodeId> = prev_path.iter().copied().collect();
    let target_set: HashSet<UzNodeId> = target_path.iter().copied().collect();

    if let Some(p) = prev {
        events.push(AppEvent::MouseOut(hover_mouse_event(
            dom, wid, p, x, y, current, buttons,
        )));
    }
    if let Some(c) = current {
        events.push(AppEvent::MouseOver(hover_mouse_event(
            dom, wid, c, x, y, prev, buttons,
        )));
    }
    // mouseleave: nodes no longer hovered, innermost first.
    for id in &prev_path {
        if !target_set.contains(id) {
            events.push(AppEvent::MouseLeave(hover_mouse_event(
                dom, wid, *id, x, y, current, buttons,
            )));
        }
    }
    // mouseenter: newly hovered nodes, outermost first.
    for id in target_path.iter().rev() {
        if !prev_set.contains(id) {
            events.push(AppEvent::MouseEnter(hover_mouse_event(
                dom, wid, *id, x, y, prev, buttons,
            )));
        }
    }

    events
}

/// Cursor to display for the pointer at `(x, y)`. The scrollbar thumb paints
/// over content but isn't a hit-tree node, so resolving from `top_node` alone
/// would pick up whatever sits underneath (e.g. selectable text -> Text
/// cursor). Force the default arrow while hovering or dragging a thumb so it
/// behaves like a normal scrollbar.
fn resolve_pointer_cursor(dom: &UIState, x: f64, y: f64) -> crate::cursor::UzCursorIcon {
    let over_thumb = dom.drag_mode.as_scrollbar_thumb().is_some()
        || dom
            .scroll_thumbs
            .iter()
            .any(|t| t.thumb_bounds.contains(x, y));
    if over_thumb {
        return crate::cursor::UzCursorIcon::Default;
    }
    dom.hit_state
        .top_node
        .map(|id| dom.resolve_cursor(id))
        .unwrap_or(crate::cursor::UzCursorIcon::Default)
}

pub fn handle_cursor_moved(
    dom: &mut UIState,
    handle: &mut Window,
    wid: u32,
    position: winit::dpi::PhysicalPosition<f64>,
    mouse_buttons: MouseButtons,
) -> (bool, Vec<AppEvent>) {
    let mut needs_redraw = false;
    let mut events: Vec<AppEvent> = Vec::new();
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

    handle.set_cursor(resolve_pointer_cursor(dom, logical_x, logical_y));

    // Synthesize the boundary-crossing events (out/over/leave/enter) before the
    // move, with per-target coordinates and relatedTarget resolved from the DOM.
    let new_top = dom.hit_state.top_node;
    events.extend(hover_transition_events(
        dom,
        wid,
        old_top,
        new_top,
        logical_x as f32,
        logical_y as f32,
        mouse_buttons,
    ));

    if let Some(node_id) = new_top {
        let (local_x, local_y) = local_offset(dom, node_id, logical_x as f32, logical_y as f32);
        events.push(AppEvent::MouseMove(UzMouseEvent {
            window_id: wid,
            node_id,
            x: logical_x as f32,
            y: logical_y as f32,
            local_x,
            local_y,
            screen_x: logical_x as f32,
            screen_y: logical_y as f32,
            button: 0,
            buttons: mouse_buttons,
            related_node_id: None,
        }));
    }

    (needs_redraw, events)
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
        // Entries whose layout node is scrolled outside the run's clip
        // region have no hitbox. Skip them rather than aborting the hit so a
        // drag keeps tracking the closest visible line.
        let Some(node) = dom.nodes.get(entry.layout_node_id) else {
            continue;
        };
        let Some(hb) = node.hitbox_id.and_then(|hid| dom.hitbox_store.get(hid)) else {
            continue;
        };
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
                let (local_x, local_y) = local_offset(dom, target, x, y);
                events.push(AppEvent::MouseDown(UzMouseEvent {
                    window_id: wid,
                    node_id: target,
                    x,
                    y,
                    local_x,
                    local_y,
                    screen_x: x,
                    screen_y: y,
                    button: button_num,
                    buttons: mouse_buttons,
                    related_node_id: None,
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
                            events.push(AppEvent::Blur(UzFocusEvent {
                                window_id: wid,
                                node_id: old_id,
                            }));
                        }
                        events.push(AppEvent::Focus(UzFocusEvent {
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
                        events.push(AppEvent::Blur(UzFocusEvent {
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
                            events.push(AppEvent::Blur(UzFocusEvent {
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
                let (local_x, local_y) = local_offset(dom, target, x, y);
                events.push(AppEvent::MouseUp(UzMouseEvent {
                    window_id: wid,
                    node_id: target,
                    x,
                    y,
                    local_x,
                    local_y,
                    screen_x: x,
                    screen_y: y,
                    button: button_num,
                    buttons: mouse_buttons,
                    related_node_id: None,
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
                    events.push(AppEvent::Input(UzInputEvent {
                        window_id: wid,
                        node_id: active,
                        input_type: "toggle".to_string(),
                        data: None,
                    }));
                }
                if let Some(target) = target_node {
                    let (local_x, local_y) = local_offset(dom, target, x, y);
                    events.push(AppEvent::Click(UzMouseEvent {
                        window_id: wid,
                        node_id: target,
                        x,
                        y,
                        local_x,
                        local_y,
                        screen_x: x,
                        screen_y: y,
                        button: button_num,
                        buttons: mouse_buttons,
                        related_node_id: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Bounds, Display, TextSelectable, UzStyle};
    use crate::text::TextRenderer;

    fn line_style() -> UzStyle {
        UzStyle {
            display: Display::Block,
            text_selectable: TextSelectable::True,
            ..Default::default()
        }
    }

    // Overflowing content (e.g. a long code block) lays out more lines than the
    // clip region shows. Lines scrolled outside that region have no hitbox. A
    // drag-select that walks past them must skip the missing entries and keep
    // tracking the closest visible line instead of aborting the hit.
    #[test]
    fn drag_hit_tracks_closest_visible_line_when_top_lines_clipped() {
        let mut dom = UIState::new();
        let mut renderer = TextRenderer::new();

        let root = dom.create_view(line_style());
        dom.set_root(root);

        for text in ["first line", "second line", "third line"] {
            let block = dom.create_view(line_style());
            let txt = dom.create_text_element(text.into(), Default::default());
            dom.append_child(block, txt);
            dom.append_child(root, block);
        }

        dom.compute_layout(200.0, 200.0, &mut renderer, 1.0);
        dom.build_text_select_runs();

        let entries: Vec<(UzNodeId, UzNodeId)> = dom
            .selectable_text_runs
            .iter()
            .find(|r| r.root_id == root)
            .expect("selectable run exists")
            .entries
            .iter()
            .map(|e| (e.layout_node_id, e.node_id))
            .collect();
        assert_eq!(entries.len(), 3);

        // Simulate the first line scrolled out of the clip region: no hitbox.
        // The remaining lines get stacked 20px-tall hitboxes.
        for (i, &(layout_node, _)) in entries.iter().enumerate() {
            if i == 0 {
                continue;
            }
            let y = (i as f64) * 20.0;
            let hid = dom
                .hitbox_store
                .insert(layout_node, Bounds::new(0.0, y, 200.0, 20.0));
            dom.nodes[layout_node].hitbox_id = Some(hid);
        }

        let hit = hit_text_in_run(&dom, &mut renderer, root, 5.0, 25.0)
            .expect("drag still hits a visible line despite the clipped first entry");
        assert_eq!(hit.node_id, entries[1].1);
    }

    fn push_thumb(dom: &mut UIState, node_id: UzNodeId, thumb: Bounds) {
        dom.scroll_thumbs
            .push(crate::paint::scroll::ScrollThumbRect {
                node_id,
                axis: ScrollAxis::Y,
                thumb_bounds: thumb,
                view_bounds: Bounds::new(thumb.x, 0.0, thumb.width, 200.0),
                content_size: 400.0,
                visible_size: 200.0,
            });
    }

    // The scrollbar thumb paints over content without being a hit-tree node, so
    // the cursor must come from the thumb rather than the selectable text
    // underneath it.
    #[test]
    fn pointer_cursor_is_default_over_scroll_thumb() {
        use crate::cursor::UzCursorIcon;

        let mut dom = UIState::new();
        let mut style = UzStyle::default();
        style.cursor = Some(UzCursorIcon::Text);
        let node = dom.create_view(style);
        dom.nodes[node].compute_styles(false, false, false, None);
        dom.hit_state.top_node = Some(node);

        push_thumb(&mut dom, node, Bounds::new(190.0, 50.0, 10.0, 60.0));

        // Over the thumb: default arrow, ignoring the underlying text cursor.
        assert_eq!(
            resolve_pointer_cursor(&dom, 194.0, 80.0),
            UzCursorIcon::Default
        );
        // Away from the thumb: the node's own cursor resolves normally.
        assert_eq!(resolve_pointer_cursor(&dom, 20.0, 80.0), UzCursorIcon::Text);
    }

    // A thumb drag may move the pointer off the thumb rect, but the cursor must
    // stay the default arrow for the whole drag.
    #[test]
    fn pointer_cursor_is_default_while_dragging_thumb() {
        use crate::cursor::UzCursorIcon;

        let mut dom = UIState::new();
        let mut style = UzStyle::default();
        style.cursor = Some(UzCursorIcon::Text);
        let node = dom.create_view(style);
        dom.nodes[node].compute_styles(false, false, false, None);
        dom.hit_state.top_node = Some(node);

        dom.drag_mode = DragMode::ScrollbarThumb(ScrollDragState {
            node_id: node,
            axis: ScrollAxis::Y,
            start_mouse_pos: 80.0,
            start_scroll_offset: 0.0,
            track_range: 140.0,
            max_scroll: 200.0,
        });

        assert_eq!(
            resolve_pointer_cursor(&dom, 20.0, 80.0),
            UzCursorIcon::Default
        );
    }
}
