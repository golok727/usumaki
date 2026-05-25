use crate::node::{ScrollAxis, UzNodeId};
use crate::ui::{ScrollWheelTarget, UIState};
use crate::window::Window;

use super::text_input::update_ime_cursor_area;

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
