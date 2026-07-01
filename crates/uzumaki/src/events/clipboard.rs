use winit::keyboard::Key;

use crate::clipboard::ClipboardBridge;
use crate::node::UzNodeId;
use crate::ui::UIState;

use super::app_event::AppEvent;
use super::types::{KeyModifiers, UzClipboardEvent, UzInputEvent};

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
        } => AppEvent::Copy(UzClipboardEvent {
            window_id: wid,
            node_id: *target,
            selection_text: Some(selection_text.clone()),
            clipboard_text: None,
        }),
        ClipboardCommand::Cut {
            target,
            selection_text,
            ..
        } => AppEvent::Cut(UzClipboardEvent {
            window_id: wid,
            node_id: *target,
            selection_text: Some(selection_text.clone()),
            clipboard_text: None,
        }),
        ClipboardCommand::Paste {
            target,
            clipboard_text,
            ..
        } => AppEvent::Paste(UzClipboardEvent {
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
                events.push(AppEvent::Input(UzInputEvent::plain(
                    wid,
                    target_id,
                    edit.kind.input_type(),
                    edit.inserted,
                )));
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
                events.push(AppEvent::Input(UzInputEvent::plain(
                    wid,
                    target_id,
                    edit.kind.input_type(),
                    edit.inserted,
                )));
                needs_redraw = true;
            }
            // For view selections, paste is a no-op
        }
    }

    (needs_redraw, events)
}
