//! SVG rasterizado com `resvg` e mostrado como imagem.

use anyhow::{Context, Result};
use image::{DynamicImage, RgbaImage};
use resvg::{tiny_skia, usvg};

/// Se `bytes` é um SVG que o `usvg` consegue ler. Não carrega fontes.
pub fn is_valid(bytes: &[u8]) -> bool {
    usvg::Tree::from_data(bytes, &usvg::Options::default()).is_ok()
}

/// Rasteriza o SVG no maior tamanho que cabe em `max_w` x `max_h` pixels
/// (`None`: altura livre), mantendo a proporção. Por ser vetorial, pode
/// ampliar. O fundo é branco: a maioria dos SVGs supõe uma página branca.
pub fn rasterize(bytes: &[u8], max_w: f64, max_h: Option<f64>) -> Result<DynamicImage> {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_data(bytes, &options).context("SVG inválido")?;
    let size = tree.size();
    let (w, h) = (f64::from(size.width()), f64::from(size.height()));
    let mut scale = max_w / w;
    if let Some(max_h) = max_h {
        scale = scale.min(max_h / h);
    }
    let width = (w * scale).round().max(1.0) as u32;
    let height = (h * scale).round().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height).context("SVG grande demais")?;
    pixmap.fill(tiny_skia::Color::WHITE);
    let transform = tiny_skia::Transform::from_scale(scale as f32, scale as f32);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    // Fundo opaco: o RGBA pré-multiplicado do `tiny-skia` já é o RGBA comum.
    let rgba = RgbaImage::from_raw(width, height, pixmap.take()).expect("tamanho do pixmap");
    Ok(DynamicImage::ImageRgba8(rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: &[u8] =
        br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="5"><rect width="5" height="5" fill="#ff0000"/></svg>"##;

    #[test]
    fn fits_the_box_keeping_the_aspect_ratio() {
        let img = rasterize(SQUARE, 100.0, Some(20.0)).unwrap();
        assert_eq!((img.width(), img.height()), (40, 20));
        let img = rasterize(SQUARE, 100.0, None).unwrap();
        assert_eq!((img.width(), img.height()), (100, 50));
    }

    #[test]
    fn background_is_white_and_shapes_are_drawn() {
        let img = rasterize(SQUARE, 10.0, None).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(2, 2).0, [255, 0, 0, 255]);
        assert_eq!(img.get_pixel(8, 2).0, [255, 255, 255, 255]);
    }

    #[test]
    fn invalid_svg_is_detected() {
        assert!(is_valid(SQUARE));
        assert!(!is_valid(b"<svg"));
        assert!(!is_valid(b"not xml"));
    }
}
