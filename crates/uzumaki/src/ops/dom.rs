use deno_core::*;
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::app::{JsState, SharedJsState, with_state};
use crate::node::{Node, NodeData, UzNodeId};
use crate::paint::scroll::{ScrollAlign, ScrollIntoViewOptions};
use crate::style::UzStyle;

fn window_not_found() -> deno_error::JsErrorBox {
    deno_error::JsErrorBox::new("WindowNotFound", "window not found")
}

fn node_not_found() -> deno_error::JsErrorBox {
    deno_error::JsErrorBox::new("NodeNotFound", "node not found")
}

fn invalid_child() -> deno_error::JsErrorBox {
    deno_error::JsErrorBox::new("InvalidChild", "child belongs to a different window")
}

pub struct CoreNode {
    js_state: Weak<RefCell<JsState>>,
    window_id: u32,
    node_id: UzNodeId,
    owned: bool,
    /// Bytes this wrapper reported into `external_memory_delta` at creation
    /// time, so the matching subtraction on drop stays balanced even when
    /// the underlying node's heap footprint has changed since.
    reported_bytes: Cell<usize>,
}

impl CoreNode {
    pub fn new(js_state: &SharedJsState, window_id: u32, node_id: UzNodeId, owned: bool) -> Self {
        let reported = if owned {
            with_state(js_state, |s| {
                let bytes = s
                    .windows
                    .get(&window_id)
                    .and_then(|e| e.dom.nodes.get(node_id))
                    .map(Node::heap_bytes)
                    .unwrap_or(0);
                s.external_memory_delta += bytes as i64;
                bytes
            })
        } else {
            0
        };
        Self {
            js_state: Rc::downgrade(js_state),
            window_id,
            node_id,
            owned,
            reported_bytes: Cell::new(reported),
        }
    }

    fn read_node<R>(&self, state: &OpState, read: impl FnOnce(&Node) -> R) -> Option<R> {
        let js_state = state.borrow::<SharedJsState>().clone();
        with_state(&js_state, |s| {
            let entry = s.windows.get(&self.window_id)?;
            let node = entry.dom.nodes.get(self.node_id)?;
            Some(read(node))
        })
    }

    fn related_node_id(
        &self,
        state: &mut OpState,
        read: impl FnOnce(&Node) -> Option<UzNodeId>,
    ) -> Result<Option<u32>, deno_error::JsErrorBox> {
        let js_state = state.borrow::<SharedJsState>().clone();
        with_state(&js_state, |s| {
            let Some(entry) = s.windows.get(&self.window_id) else {
                return Err(window_not_found());
            };
            let Some(node) = entry.dom.nodes.get(self.node_id) else {
                return Ok(None);
            };
            let Some(related_id) = read(node) else {
                return Ok(None);
            };

            Ok(Some(related_id as u32))
        })
    }
}

impl Drop for CoreNode {
    fn drop(&mut self) {
        if !self.owned {
            return;
        }

        let Some(js_state) = self.js_state.upgrade() else {
            return;
        };

        // cppgc finalizers can run inside any V8 turn, including ones where an
        // op already holds AppState borrowed. Use try_borrow_mut and fall back
        // to leaving the slab entry for the next finalizer pass — never panic.
        let Ok(mut state) = js_state.try_borrow_mut() else {
            return;
        };
        state.external_memory_delta -= self.reported_bytes.get() as i64;
        if state.windows.contains_key(&self.window_id) {
            state
                .pending_destroy
                .push_back((self.window_id, self.node_id));
        }
    }
}

unsafe impl GarbageCollected for CoreNode {
    fn trace(&self, _visitor: &mut deno_core::v8::cppgc::Visitor) {}

    fn get_name(&self) -> &'static std::ffi::CStr {
        c"CoreNode"
    }
}

#[op2]
#[cppgc]
pub fn op_get_root_node(
    state: &mut OpState,
    #[smi] window_id: u32,
) -> Result<CoreNode, deno_error::JsErrorBox> {
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get(&window_id) else {
            return Err(window_not_found());
        };
        let root = entry.dom.root.expect("no root node");
        Ok(CoreNode::new(&js_state, window_id, root, false))
    })
}

#[op2]
#[cppgc]
pub fn op_create_element_node(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[string] element_type: String,
) -> Result<CoreNode, deno_error::JsErrorBox> {
    let js_state = state.borrow::<SharedJsState>().clone();
    let node_id = create_element(state, window_id, &element_type)?;
    Ok(CoreNode::new(
        &js_state,
        window_id,
        node_id as UzNodeId,
        true,
    ))
}

#[op2]
#[cppgc]
pub fn op_create_text_node(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[string] text: String,
) -> Result<CoreNode, deno_error::JsErrorBox> {
    let js_state = state.borrow::<SharedJsState>().clone();
    let node_id = create_text_node(state, window_id, text)?;
    Ok(CoreNode::new(
        &js_state,
        window_id,
        node_id as UzNodeId,
        true,
    ))
}

#[op2]
impl CoreNode {
    /// Re-acquire a wrapper around an existing slab node by id. Used by the JS
    /// registry to rebuild a wrapper after its previous one was collected but
    /// the native node is still alive (because it's connected to the tree).
    /// Throws if the slab entry is gone.
    #[constructor]
    #[cppgc]
    fn new_from_id(
        state: &mut OpState,
        #[smi] window_id: u32,
        #[smi] node_id: u32,
    ) -> Result<CoreNode, deno_error::JsErrorBox> {
        let js_state = state.borrow::<SharedJsState>().clone();
        with_state(&js_state, |s| {
            let Some(entry) = s.windows.get(&window_id) else {
                return Err(window_not_found());
            };
            if !entry.dom.nodes.contains(node_id as UzNodeId) {
                return Err(node_not_found());
            }
            Ok(CoreNode::new(
                &js_state,
                window_id,
                node_id as UzNodeId,
                true,
            ))
        })
    }

    #[getter]
    #[smi]
    pub fn id(&self) -> u32 {
        self.node_id as u32
    }

    #[getter]
    #[smi]
    #[allow(non_snake_case)]
    pub fn windowId(&self) -> u32 {
        self.window_id
    }

    #[getter]
    #[smi]
    #[allow(non_snake_case)]
    pub fn nodeType(&self, state: &OpState) -> Option<u32> {
        self.read_node(state, |node| match node.data {
            NodeData::Root => 1,
            NodeData::Element(_) | NodeData::AnonymousBlock(_) => 2,
            NodeData::Text(_) => 3,
        })
    }

    // #[getter]
    // #[string]
    // #[allow(non_snake_case)]
    // pub fn nodeName(&self) -> String {
    //    todo!()
    // }

    #[getter]
    #[smi]
    #[allow(non_snake_case)]
    pub fn parentNodeId(&self, state: &mut OpState) -> Result<Option<u32>, deno_error::JsErrorBox> {
        self.related_node_id(state, |node| node.parent)
    }

    #[getter]
    #[smi]
    #[allow(non_snake_case)]
    pub fn firstChildId(&self, state: &mut OpState) -> Result<Option<u32>, deno_error::JsErrorBox> {
        let js_state = state.borrow::<SharedJsState>().clone();
        with_state(&js_state, |s| {
            let Some(entry) = s.windows.get(&self.window_id) else {
                return Err(window_not_found());
            };
            Ok(entry.dom.first_child(self.node_id).map(|id| id as u32))
        })
    }

    #[getter]
    #[smi]
    #[allow(non_snake_case)]
    pub fn lastChildId(&self, state: &mut OpState) -> Result<Option<u32>, deno_error::JsErrorBox> {
        let js_state = state.borrow::<SharedJsState>().clone();
        with_state(&js_state, |s| {
            let Some(entry) = s.windows.get(&self.window_id) else {
                return Err(window_not_found());
            };
            Ok(entry.dom.last_child(self.node_id).map(|id| id as u32))
        })
    }

    #[getter]
    #[smi]
    #[allow(non_snake_case)]
    pub fn nextSiblingId(
        &self,
        state: &mut OpState,
    ) -> Result<Option<u32>, deno_error::JsErrorBox> {
        let js_state = state.borrow::<SharedJsState>().clone();
        with_state(&js_state, |s| {
            let Some(entry) = s.windows.get(&self.window_id) else {
                return Err(window_not_found());
            };
            Ok(entry.dom.next_sibling(self.node_id).map(|id| id as u32))
        })
    }

    #[getter]
    #[smi]
    #[allow(non_snake_case)]
    pub fn previousSiblingId(
        &self,
        state: &mut OpState,
    ) -> Result<Option<u32>, deno_error::JsErrorBox> {
        let js_state = state.borrow::<SharedJsState>().clone();
        with_state(&js_state, |s| {
            let Some(entry) = s.windows.get(&self.window_id) else {
                return Err(window_not_found());
            };
            Ok(entry.dom.prev_sibling(self.node_id).map(|id| id as u32))
        })
    }

    #[fast]
    #[allow(non_snake_case)]
    pub fn appendChild(
        &self,
        state: &mut OpState,
        #[cppgc] child: &CoreNode,
    ) -> Result<(), deno_error::JsErrorBox> {
        if child.window_id != self.window_id {
            return Err(invalid_child());
        }
        append_child(
            state,
            self.window_id,
            self.node_id as u32,
            child.node_id as u32,
        )
    }

    #[fast]
    #[allow(non_snake_case)]
    pub fn insertBefore(
        &self,
        state: &mut OpState,
        #[cppgc] child: &CoreNode,
        #[cppgc] before: Option<&CoreNode>,
    ) -> Result<(), deno_error::JsErrorBox> {
        if child.window_id != self.window_id
            || before.is_some_and(|b| b.window_id != self.window_id)
        {
            return Err(invalid_child());
        }
        if let Some(before) = before {
            insert_before(
                state,
                self.window_id,
                self.node_id as u32,
                child.node_id as u32,
                before.node_id as u32,
            )
        } else {
            append_child(
                state,
                self.window_id,
                self.node_id as u32,
                child.node_id as u32,
            )
        }
    }

    #[fast]
    #[allow(non_snake_case)]
    pub fn removeChild(
        &self,
        state: &mut OpState,
        #[cppgc] child: &CoreNode,
    ) -> Result<(), deno_error::JsErrorBox> {
        if child.window_id != self.window_id {
            return Err(invalid_child());
        }
        remove_child(
            state,
            self.window_id,
            self.node_id as u32,
            child.node_id as u32,
        )
    }

    #[fast]
    pub fn remove(&self, state: &mut OpState) -> Result<(), deno_error::JsErrorBox> {
        detach_from_parent(state, self.window_id, self.node_id as u32)
    }

    #[fast]
    #[allow(non_snake_case)]
    pub fn removeChildren(&self, state: &mut OpState) -> Result<(), deno_error::JsErrorBox> {
        let js_state = state.borrow::<SharedJsState>().clone();
        with_state(&js_state, |s| {
            let Some(entry) = s.windows.get_mut(&self.window_id) else {
                return Err(window_not_found());
            };
            entry.dom.clear_children(self.node_id);
            Ok(())
        })
    }

    #[fast]
    #[allow(non_snake_case)]
    pub fn setAttribute(&self, state: &mut OpState, #[string] name: &str, #[string] value: &str) {
        set_attribute(state, self.window_id, self.node_id as u32, name, value);
    }

    #[fast]
    #[allow(non_snake_case)]
    pub fn removeAttribute(&self, state: &mut OpState, #[string] name: &str) {
        clear_attribute(state, self.window_id, self.node_id as u32, name);
    }

    #[serde]
    #[allow(non_snake_case)]
    pub fn getAttribute(
        &self,
        state: &mut OpState,
        #[string] name: String,
    ) -> Result<serde_json::Value, deno_error::JsErrorBox> {
        get_attribute(state, self.window_id, self.node_id as u32, &name)
    }

    #[getter]
    #[string]
    #[allow(non_snake_case)]
    pub fn textContent(
        &self,
        state: &mut OpState,
    ) -> Result<Option<String>, deno_error::JsErrorBox> {
        let js_state = state.borrow::<SharedJsState>().clone();
        with_state(&js_state, |s| {
            let Some(entry) = s.windows.get(&self.window_id) else {
                return Err(window_not_found());
            };
            let Some(node) = entry.dom.nodes.get(self.node_id) else {
                return Err(node_not_found());
            };
            Ok(node.get_text_content().map(|text| text.content.clone()))
        })
    }

    #[setter]
    #[allow(non_snake_case)]
    pub fn textContent(
        &self,
        state: &mut OpState,
        #[string] text: String,
    ) -> Result<(), deno_error::JsErrorBox> {
        set_text(state, self.window_id, self.node_id as u32, text)
    }

    #[fast]
    #[allow(non_snake_case)]
    pub fn scrollIntoView(
        &self,
        state: &mut OpState,
        #[smi] block: u32,
        #[smi] inline: u32,
    ) -> Result<(), deno_error::JsErrorBox> {
        let opts = ScrollIntoViewOptions {
            block: scroll_align_from_int(block),
            inline: scroll_align_from_int(inline),
            ..Default::default()
        };
        let app_state = state.borrow::<SharedAppState>().clone();
        with_state(&app_state, |s| {
            let Some(entry) = s.windows.get_mut(&self.window_id) else {
                return Err(window_not_found());
            };
            entry.dom.request_scroll_node_into_view(self.node_id, opts);
            if let Some(handle) = entry.handle.as_ref() {
                handle.winit_window.request_redraw();
            }
            Ok(())
        })
    }
}

fn scroll_align_from_int(v: u32) -> ScrollAlign {
    match v {
        0 => ScrollAlign::Start,
        1 => ScrollAlign::Center,
        2 => ScrollAlign::End,
        3 => ScrollAlign::Nearest,
        _ => ScrollAlign::Start,
    }
}

fn create_element(
    state: &mut OpState,
    window_id: u32,
    element_type: &str,
) -> Result<u32, deno_error::JsErrorBox> {
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(window_not_found());
        };
        let style = UzStyle::default_for_element(element_type);
        let id = if element_type == "input" {
            entry.dom.create_input(style)
        } else if element_type == "checkbox" {
            entry.dom.create_checkbox(style)
        } else if element_type == "image" {
            entry.dom.create_image(style)
        } else if element_type == "text" {
            entry.dom.create_text_element(String::new(), style)
        } else if element_type == "button" {
            entry.dom.create_button(style)
        } else {
            entry.dom.create_view(style)
        };
        Ok(id as u32)
    })
}

fn create_text_node(
    state: &mut OpState,
    window_id: u32,
    text: String,
) -> Result<u32, deno_error::JsErrorBox> {
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(window_not_found());
        };
        Ok(entry
            .dom
            .create_text_node(text, UzStyle::default_for_element("#text")) as u32)
    })
}

fn append_child(
    state: &mut OpState,
    window_id: u32,
    parent_id: u32,
    child_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let pid = parent_id as UzNodeId;
    let cid = child_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(window_not_found());
        };
        entry.dom.append_child(pid, cid);
        Ok(())
    })
}

fn insert_before(
    state: &mut OpState,
    window_id: u32,
    parent_id: u32,
    child_id: u32,
    before_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let pid = parent_id as UzNodeId;
    let cid = child_id as UzNodeId;
    let bid = before_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(window_not_found());
        };
        entry.dom.insert_before(pid, cid, bid);
        Ok(())
    })
}

fn remove_child(
    state: &mut OpState,
    window_id: u32,
    parent_id: u32,
    child_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let pid = parent_id as UzNodeId;
    let cid = child_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(window_not_found());
        };
        entry.dom.remove_child(pid, cid);
        Ok(())
    })
}

fn detach_from_parent(
    state: &mut OpState,
    window_id: u32,
    child_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let cid = child_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(window_not_found());
        };
        let Some(parent_id) = entry.dom.nodes.get(cid).and_then(|node| node.parent) else {
            return Ok(());
        };
        entry.dom.remove_child(parent_id, cid);
        Ok(())
    })
}

fn set_text(
    state: &mut OpState,
    window_id: u32,
    node_id: u32,
    text: String,
) -> Result<(), deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(window_not_found());
        };
        entry.dom.set_text_content(nid, text);
        Ok(())
    })
}

fn set_attribute(state: &mut OpState, window_id: u32, node_id: u32, name: &str, value: &str) {
    let nid = node_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        if let Some(entry) = s.windows.get_mut(&window_id) {
            entry.set_attribute(nid, name, value);
        }
    });
}

fn clear_attribute(state: &mut OpState, window_id: u32, node_id: u32, name: &str) {
    let nid = node_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        if let Some(entry) = s.windows.get_mut(&window_id) {
            entry.clear_attribute(nid, name);
        }
    });
}

fn get_attribute(
    state: &mut OpState,
    window_id: u32,
    node_id: u32,
    name: &str,
) -> Result<Value, deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get(&window_id) else {
            return Ok(Value::Null);
        };
        Ok(entry.get_attribute(nid, name))
    })
}

#[op2(fast)]
pub fn op_focus_element(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(window_not_found());
        };
        entry.dom.focus_element(nid);
        entry.dom.request_scroll_focus_into_view(nid);
        if let Some(window) = entry.window.as_ref() {
            window.request_redraw();
        }
        Ok(())
    })
}

#[op2]
#[serde]
pub fn op_get_ancestor_path(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
) -> Result<Vec<u32>, deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get(&window_id) else {
            return Ok(Vec::new());
        };
        let mut path = Vec::new();
        let mut current = Some(nid);
        while let Some(id) = current {
            path.push(id as u32);
            current = entry.dom.nodes.get(id).and_then(|n| n.parent);
        }
        Ok(path)
    })
}

// Selection
#[op2]
#[serde]
pub fn op_get_selection(
    state: &mut OpState,
    #[smi] window_id: u32,
) -> Result<serde_json::Value, deno_error::JsErrorBox> {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SelectionEndpointState {
        node_id: u32,
        offset: usize,
        affinity: crate::selection::Affinity,
    }

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SelectionState {
        anchor: SelectionEndpointState,
        focus: SelectionEndpointState,
        is_collapsed: bool,
        text: String,
    }

    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get(&window_id) else {
            return Ok(serde_json::Value::Null);
        };
        let dom = &entry.dom;
        let Some(sel) = dom.get_selection() else {
            return Ok(serde_json::Value::Null);
        };
        let (Some(anchor), Some(focus)) = (sel.anchor, sel.focus) else {
            return Ok(serde_json::Value::Null);
        };
        let text = dom.selected_text();
        Ok(serde_json::to_value(SelectionState {
            anchor: SelectionEndpointState {
                node_id: anchor.node as u32,
                offset: anchor.offset,
                affinity: anchor.affinity,
            },
            focus: SelectionEndpointState {
                node_id: focus.node as u32,
                offset: focus.offset,
                affinity: focus.affinity,
            },
            is_collapsed: sel.is_collapsed(),
            text,
        })
        .unwrap())
    })
}

#[op2]
#[string]
pub fn op_get_selected_text(
    state: &mut OpState,
    #[smi] window_id: u32,
) -> Result<String, deno_error::JsErrorBox> {
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        let Some(entry) = s.windows.get(&window_id) else {
            return Ok(String::new());
        };
        Ok(entry.dom.selected_text())
    })
}
