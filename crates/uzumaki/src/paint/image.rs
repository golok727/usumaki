use std::sync::Arc;
use std::sync::OnceLock;

use vello::Scene;
use vello::kurbo::Affine;
use vello::peniko::{ImageAlphaType, ImageData as VelloImageData, ImageFormat};

use crate::element::{ImageData, RasterImageData};
use crate::paint::svg::render_svg_tree;
use crate::style::{Bounds, Color, UzStyle};

#[derive(Clone)]
pub struct ImageRenderInfo {
    pub data: ImageData,
}

const FALLBACK_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
<rect x="6" y="10" width="52" height="44" rx="4" fill="none" stroke="#808080" stroke-width="1"/>
<circle cx="22" cy="24" r="4" fill="none" stroke="#808080" stroke-width="1"/>
<path d="M10 48 L26 30 L36 40 L44 32 L56 46" fill="none" stroke="#808080" stroke-width="1.25" stroke-linejoin="round"/>
</svg>"##;

fn fallback_tree() -> &'static Arc<usvg::Tree> {
    static TREE: OnceLock<Arc<usvg::Tree>> = OnceLock::new();
    TREE.get_or_init(|| {
        let opts = usvg::Options::default();
        let tree = usvg::Tree::from_str(FALLBACK_SVG, &opts).expect("fallback svg parses");
        Arc::new(tree)
    })
}

pub fn paint_image(
    scene: &mut Scene,
    bounds: Bounds,
    style: &UzStyle,
    image: &ImageRenderInfo,
    transform: Affine,
) {
    style.paint(bounds, scene, transform, |scene| match &image.data {
        ImageData::Raster(raster) => paint_raster(scene, bounds, raster, transform),
        ImageData::Svg {
            tree,
            uses_current_color,
        } => paint_svg(
            scene,
            bounds,
            tree,
            transform,
            uses_current_color.then_some(style.text.color),
        ),
        ImageData::None => paint_svg(scene, bounds, fallback_tree(), transform, None),
    });
}

fn paint_raster(scene: &mut Scene, bounds: Bounds, raster: &RasterImageData, transform: Affine) {
    if raster.width == 0 || raster.height == 0 {
        return;
    }
    let vello_image = VelloImageData {
        data: raster.data.clone(),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width: raster.width,
        height: raster.height,
    };
    let scale_x = bounds.width / raster.width as f64;
    let scale_y = bounds.height / raster.height as f64;
    let image_transform = transform * Affine::new([scale_x, 0.0, 0.0, scale_y, 0.0, 0.0]);
    scene.draw_image(&vello_image, image_transform);
}

fn paint_svg(
    scene: &mut Scene,
    bounds: Bounds,
    tree: &usvg::Tree,
    transform: Affine,
    current_color: Option<Color>,
) {
    let size = tree.size();
    if size.width() <= 0.0 || size.height() <= 0.0 {
        return;
    }
    let scale_x = bounds.width / size.width() as f64;
    let scale_y = bounds.height / size.height() as f64;
    let svg_transform = transform * Affine::scale_non_uniform(scale_x, scale_y);
    render_svg_tree(scene, tree, svg_transform, current_color);
}
