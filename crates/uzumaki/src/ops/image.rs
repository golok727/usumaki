use std::sync::Arc;

use deno_core::{JsBuffer, OpState, op2};
use deno_error::JsErrorBox;
use image::GenericImageView;

use crate::{
    app::{SharedJsState, with_state},
    element::{ImageData, RasterImageData},
    node::UzNodeId,
};

/// Apply new image data to a node and adjust V8's external-memory accounting
/// by the delta. Without this, swapping a 4MB raster onto a node would look
/// free to the GC heuristic.
fn apply_and_report(s: &mut crate::app::JsState, window_id: u32, nid: UzNodeId, image: ImageData) {
    let Some(entry) = s.windows.get_mut(&window_id) else {
        return;
    };
    let old = entry
        .dom
        .nodes
        .get(nid)
        .and_then(|n| n.as_image())
        .map(|i| i.heap_bytes())
        .unwrap_or(0);
    entry.dom.set_image_data(nid, image);
    let new = entry
        .dom
        .nodes
        .get(nid)
        .and_then(|n| n.as_image())
        .map(|i| i.heap_bytes())
        .unwrap_or(0);
    s.external_memory_delta += new as i64 - old as i64;
}

fn window_not_found() -> JsErrorBox {
    JsErrorBox::new("WindowNotFound", "window not found")
}

fn invalid_image_data(error: impl std::fmt::Display) -> JsErrorBox {
    JsErrorBox::new("InvalidImageData", error.to_string())
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let head = &bytes[i..bytes.len().min(i + 512)];
    let s = std::str::from_utf8(head).unwrap_or("");
    s.starts_with("<?xml") || s.starts_with("<svg") || s.contains("<svg")
}

fn decode(data: &[u8]) -> Result<ImageData, JsErrorBox> {
    if looks_like_svg(data) {
        let opts = usvg::Options::default();
        let tree = usvg::Tree::from_data(data, &opts).map_err(invalid_image_data)?;
        let text = std::str::from_utf8(data).unwrap_or("");
        return Ok(ImageData::Svg {
            tree: Arc::new(tree),
            uses_current_color: text.contains("currentColor") || text.contains("currentcolor"),
        });
    }
    let decoded = image::load_from_memory(data).map_err(invalid_image_data)?;
    let (width, height) = decoded.dimensions();
    let rgba = decoded.to_rgba8();
    Ok(ImageData::Raster(RasterImageData::new(
        width,
        height,
        Arc::new(rgba.into_raw()),
    )))
}

#[op2]
pub fn op_set_encoded_image_data(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] cache_key: String,
    #[buffer] data: JsBuffer,
) -> Result<(), JsErrorBox> {
    let nid = node_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();

    let cached = with_state(&js_state, |s| s.image_cache.get(&cache_key).cloned());
    let image = match cached {
        Some(img) => img,
        None => {
            let decoded = decode(&data)?;
            with_state(&js_state, |s| {
                s.image_cache.insert(cache_key.clone(), decoded.clone());
            });
            decoded
        }
    };

    with_state(&js_state, |s| {
        if !s.windows.contains_key(&window_id) {
            return Err(window_not_found());
        }
        apply_and_report(s, window_id, nid, image);
        Ok(())
    })
}

#[op2(fast)]
pub fn op_apply_cached_image(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] cache_key: String,
) -> bool {
    let nid = node_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();

    let cached = with_state(&js_state, |s| s.image_cache.get(&cache_key).cloned());
    let Some(image) = cached else {
        return false;
    };

    with_state(&js_state, |s| {
        apply_and_report(s, window_id, nid, image);
    });
    true
}

#[op2(fast)]
pub fn op_clear_image_data(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
) -> Result<(), JsErrorBox> {
    let nid = node_id as UzNodeId;
    let js_state = state.borrow::<SharedJsState>().clone();
    with_state(&js_state, |s| {
        if !s.windows.contains_key(&window_id) {
            return Err(window_not_found());
        }
        apply_and_report(s, window_id, nid, ImageData::None);
        Ok(())
    })
}
