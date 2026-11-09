use gdk4::{MemoryFormat, MemoryTexture, Texture};
use glib::Bytes;
use gtk4::prelude::*;
use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};

/// Dominant accent color extracted from cover art (0–1 RGB).
#[derive(Debug, Clone, Copy)]
pub struct AccentColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Default for AccentColor {
    fn default() -> Self {
        Self {
            r: 0.0,
            g: 0.9,
            b: 1.0,
        }
    }
}

impl AccentColor {
    pub fn css_rgb(self) -> String {
        format!(
            "rgb({},{},{})",
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8
        )
    }
}

/// Decode image bytes, extract accent, and build a soft blurred backdrop texture.
pub fn process_cover(bytes: &[u8]) -> Option<(AccentColor, Texture, Texture)> {
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let accent = extract_accent(&img);
    let sharp = rgba_to_texture(&img)?;
    let blurred = blur_backdrop(&img)?;
    Some((accent, sharp, blurred))
}

fn extract_accent(img: &RgbaImage) -> AccentColor {
    let (w, h) = img.dimensions();
    let step_x = (w / 24).max(1);
    let step_y = (h / 24).max(1);
    let mut sum_r = 0u64;
    let mut sum_g = 0u64;
    let mut sum_b = 0u64;
    let mut count = 0u64;

    for y in (0..h).step_by(step_y as usize) {
        for x in (0..w).step_by(step_x as usize) {
            let p = img.get_pixel(x, y).0;
            let r = p[0] as f32;
            let g = p[1] as f32;
            let b = p[2] as f32;
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let sat = if max > 1.0 { (max - min) / max } else { 0.0 };
            let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            if sat > 0.18 && lum > 30.0 && lum < 220.0 {
                let weight = ((sat * 2.0) as u64).max(1);
                sum_r += p[0] as u64 * weight;
                sum_g += p[1] as u64 * weight;
                sum_b += p[2] as u64 * weight;
                count += weight;
            }
        }
    }

    if count == 0 {
        return AccentColor::default();
    }

    let mut r = sum_r as f64 / count as f64 / 255.0;
    let mut g = sum_g as f64 / count as f64 / 255.0;
    let mut b = sum_b as f64 / count as f64 / 255.0;
    let avg = (r + g + b) / 3.0;
    r = (avg + (r - avg) * 1.25).clamp(0.0, 1.0);
    g = (avg + (g - avg) * 1.25).clamp(0.0, 1.0);
    b = (avg + (b - avg) * 1.25).clamp(0.0, 1.0);
    AccentColor { r, g, b }
}

fn blur_backdrop(img: &RgbaImage) -> Option<Texture> {
    let small = imageops::resize(img, 64, 64, FilterType::Triangle);
    let blurred = imageops::blur(&small, 3.0);
    let soft = imageops::resize(&blurred, 480, 480, FilterType::Triangle);
    let muted: RgbaImage = RgbaImage::from_fn(soft.width(), soft.height(), |x, y| {
        let p = soft.get_pixel(x, y).0;
        Rgba([
            (p[0] as f32 * 0.35) as u8,
            (p[1] as f32 * 0.35) as u8,
            (p[2] as f32 * 0.35) as u8,
            220,
        ])
    });
    rgba_to_texture(&muted)
}

fn rgba_to_texture(img: &RgbaImage) -> Option<Texture> {
    let (w, h) = img.dimensions();
    let bytes = Bytes::from(&img.as_raw()[..]);
    let mem = MemoryTexture::new(
        w as i32,
        h as i32,
        MemoryFormat::R8g8b8a8,
        &bytes,
        (w * 4) as usize,
    );
    Some(mem.upcast::<Texture>())
}
