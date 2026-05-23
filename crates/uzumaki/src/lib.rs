pub mod runtime;

pub mod app;
pub mod clipboard;
pub mod construct;
pub mod cursor;
pub mod element;
pub mod event_dispatch;
pub mod gpu;
pub mod headless;
pub mod hit_tree;
pub mod input;
pub mod interactivity;
pub(crate) mod layout;
pub mod node;
pub mod ops;
pub mod paint;
pub mod selection;
pub mod shared_string;
pub mod style;
pub mod terminal_colors;
pub mod text;
pub mod ui;
pub mod window;

pub use shared_string::*;

use deno_core::*;

pub use crate::app::AppConfig;
pub use crate::app::Application;

pub(crate) mod parse;
pub(crate) mod prop_keys;

pub use deno_core;
pub use deno_runtime;
pub use rustls;

pub static TS_VERSION: &str = "5.9.2";

#[cfg(feature = "snapshot")]
pub fn create_snapshot(
    snapshot_path: std::path::PathBuf,
    snapshot_options: deno_runtime::ops::bootstrap::SnapshotOptions,
) {
    deno_runtime::snapshot::create_runtime_snapshot(
        snapshot_path,
        snapshot_options,
        vec![uzumaki::init()],
    );
}

use crate::ops::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[op2]
#[string]
fn op_get_uz_runtime_version() -> Result<String, deno_error::JsErrorBox> {
    Ok(VERSION.to_string())
}

extension!(
  uzumaki,
  ops = [
    op_get_uz_runtime_version,
    op_create_window,
    op_request_quit,
    op_request_redraw,
    op_get_root_node,
    op_create_element_node,
    op_create_text_node,
    op_set_encoded_image_data,
    op_apply_cached_image,
    op_clear_image_data,
    op_focus_element,
    op_get_ancestor_path,
    op_get_selection,
    op_get_selected_text,
    op_read_clipboard_text,
    op_write_clipboard_text,
  ],
  objects = [ops::window::CoreWindow, ops::dom::CoreNode, ops::path::AppPath],
  esm_entry_point = "ext:uzumaki/runtime.ts",
  esm = [
    dir "js",
    "ext:uzumaki/runtime.ts" = "runtime.ts",
    "ext:uzumaki/node.ts" = "node.ts",
    "ext:uzumaki/registry.ts" = "registry.ts",
    "ext:uzumaki/types.ts" = "types.ts",
    "ext:uzumaki/window.ts" = "window.ts",
    "ext:uzumaki/events.ts" = "events.ts",
    "ext:uzumaki/event-target.ts" = "event-target.ts",
    "ext:uzumaki/dispatcher.ts" = "dispatcher.ts",
    "ext:uzumaki/core.ts" = "core.ts",
    "ext:uzumaki/clipboard.ts" = "clipboard.ts",
    "ext:uzumaki/elements/base.ts" = "elements/base.ts",
    "ext:uzumaki/elements/element.ts" = "elements/element.ts",
    "ext:uzumaki/elements/button.ts" = "elements/button.ts",
    "ext:uzumaki/elements/checkbox.ts" = "elements/checkbox.ts",
    "ext:uzumaki/elements/image.ts" = "elements/image.ts",
    "ext:uzumaki/elements/input.ts" = "elements/input.ts",
    "ext:uzumaki/elements/root.ts" = "elements/root.ts",
    "ext:uzumaki/elements/text.ts" = "elements/text.ts",
    "ext:uzumaki/elements/view.ts" = "elements/view.ts"
  ],
);
