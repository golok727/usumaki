use winit::keyboard::{Key, NamedKey};

use crate::input::{EditKind, KeyResult, preview_key_edit};
use crate::selection::{Affinity, SelectionEndpoint, TextSelection};
use crate::text::apply_text_style_to_editor;
use crate::ui::UIState;
use crate::window::Window;

use super::app_event::AppEvent;
use super::text_input::{input_layout_meta, scroll_input_to_cursor};
use super::types::{
    KeyModifiers, MouseButtons, UzFocusEvent, UzInputEvent, UzKeyboardEvent, UzMouseEvent,
};

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

    let data = UzKeyboardEvent {
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

/// Build a cancelable `beforeinput` for the focused input, describing the edit
/// the key would produce. Dispatched before [`handle_key_for_input`] so JS can
/// `preventDefault()` to stop the edit from committing. Returns `None` when the
/// key is not an edit or no input is focused.
pub fn build_beforeinput_event(
    dom: &UIState,
    wid: u32,
    key_event: &winit::event::KeyEvent,
    modifiers: KeyModifiers,
) -> Option<AppEvent> {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return None;
    }
    let fid = dom.focused_node?;
    let is = dom.nodes.get(fid)?.as_text_input()?;
    let (kind, data) = is.preview_edit(&key_event.logical_key, modifiers)?;
    Some(AppEvent::BeforeInput(UzInputEvent::plain(
        wid,
        fid,
        kind.input_type(),
        data,
    )))
}

fn compute_edit_range(
    dom: &UIState,
    kind: EditKind,
) -> Option<(SelectionEndpoint, SelectionEndpoint)> {
    if matches!(kind, EditKind::HistoryUndo | EditKind::HistoryRedo) {
        return None;
    }
    let sel = dom.get_text_selection()?;
    let focus = sel.focus?;
    let root = dom.selection_root(&sel)?;

    let (mut start, mut end) = dom.ordered_text_selection()?;

    if !sel.is_collapsed() {
        return Some((start, end));
    }

    let flat = dom.flat_index_for_endpoint(focus)?;
    match kind {
        EditKind::DeleteBackward => {
            if flat == 0 {
                return Some((start, end));
            }
            start = dom.endpoint_from_flat_index(root, flat - 1, Affinity::Downstream)?;
        }
        EditKind::DeleteForward => {
            end = dom.endpoint_from_flat_index(root, flat + 1, Affinity::Upstream)?;
        }
        EditKind::DeleteWordBackward => {
            let prev = dom.prev_word_boundary_in_run(root, flat);
            if prev == flat {
                return Some((start, end));
            }
            start = dom.endpoint_from_flat_index(root, prev, Affinity::Downstream)?;
        }
        EditKind::DeleteWordForward => {
            let next = dom.next_word_boundary_in_run(root, flat);
            if next == flat {
                return Some((start, end));
            }
            end = dom.endpoint_from_flat_index(root, next, Affinity::Upstream)?;
        }
        _ => {}
    }
    Some((start, end))
}

/// Fire a `textupdate` event describing the edit a key would produce on the
/// focused editContext view. The framework never mutates the view's text —
/// JS applies the edit to its own text buffer and re-renders.
pub fn handle_key_for_edit_context(
    dom: &mut UIState,
    wid: u32,
    key_event: &winit::event::KeyEvent,
    modifiers: KeyModifiers,
) -> Vec<AppEvent> {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return Vec::new();
    }
    let Some(fid) = dom.focused_node else {
        return Vec::new();
    };
    let Some(node) = dom.nodes.get(fid) else {
        return Vec::new();
    };
    if !node.is_edit_context_root() {
        return Vec::new();
    }
    let Some((kind, data)) = preview_key_edit(&key_event.logical_key, modifiers, true) else {
        return Vec::new();
    };
    let (start_node_id, start_offset, end_node_id, end_offset) = match compute_edit_range(dom, kind)
    {
        Some((s, e)) => (Some(s.node), s.offset as u32, Some(e.node), e.offset as u32),
        None => (None, 0, None, 0),
    };
    vec![AppEvent::TextUpdate(UzInputEvent {
        window_id: wid,
        node_id: fid,
        input_type: kind.input_type(),
        data,
        start_node_id,
        start_offset,
        end_node_id,
        end_offset,
    })]
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
                        events.push(AppEvent::Input(UzInputEvent::plain(
                            wid,
                            focused_id,
                            edit.kind.input_type(),
                            edit.inserted,
                        )));
                        needs_redraw = true;
                    }
                    KeyResult::Blur => {
                        new_focus = None;
                        events.push(AppEvent::Blur(UzFocusEvent {
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
        vec![AppEvent::Input(UzInputEvent::plain(
            wid, focused_id, "toggle", None,
        ))],
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
    let (x, y, local_x, local_y) = node
        .hitbox_id
        .and_then(|hid| dom.hitbox_store.get(hid))
        .map(|hb| {
            let bounds = hb.window_aabb();
            (
                (bounds.x + bounds.width / 2.0) as f32,
                (bounds.y + bounds.height / 2.0) as f32,
                (bounds.width / 2.0) as f32,
                (bounds.height / 2.0) as f32,
            )
        })
        .unwrap_or((0.0, 0.0, 0.0, 0.0));

    (
        true,
        vec![AppEvent::Click(UzMouseEvent {
            window_id: wid,
            node_id: focused_id,
            x,
            y,
            local_x,
            local_y,
            screen_x: x,
            screen_y: y,
            button: 0,
            buttons: MouseButtons::empty(),
            related_node_id: None,
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
            outcome.events.push(AppEvent::Blur(UzFocusEvent {
                window_id: wid,
                node_id: old,
            }));
        }
        outcome.events.push(AppEvent::Focus(UzFocusEvent {
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
    wid: u32,
    key_event: &winit::event::KeyEvent,
    modifiers: KeyModifiers,
) -> (bool, Vec<super::AppEvent>) {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return (false, Vec::new());
    }

    let selection_before = dom.text_selection;
    let redraw = handle_key_for_view_selection_inner(dom, key_event, modifiers);
    let mut events = Vec::new();
    if let Some(event) = super::selection_change_event(wid, &selection_before, &dom.text_selection)
    {
        events.push(event);
    }
    (redraw, events)
}

fn handle_key_for_view_selection_inner(
    dom: &mut UIState,
    key_event: &winit::event::KeyEvent,
    modifiers: KeyModifiers,
) -> bool {
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
