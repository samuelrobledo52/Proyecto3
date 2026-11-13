use rayon::prelude::*;

use crate::math::Vec3;
use crate::render::framebuffer::Framebuffer;

const MIN_RADIUS: f32 = 0.7;
const MAX_RADIUS: f32 = 80.0;
const BAND: usize = 8;

#[derive(Clone, Copy)]
pub struct Sprite {
    x: f32,
    y: f32,
    radius: f32,
    inv_w: f32,
    rgb: Vec3,
}

impl Sprite {
    pub fn new(x: f32, y: f32, radius: f32, inv_w: f32, rgb: Vec3) -> Option<Sprite> {
        let (radius, rgb) = if radius < MIN_RADIUS {
            let ratio = radius.max(0.0) / MIN_RADIUS;
            (MIN_RADIUS, rgb * (ratio * ratio * 255.0))
        } else {
            (radius.min(MAX_RADIUS), rgb * 255.0)
        };
        radius
            .is_finite()
            .then_some(Sprite {
                x,
                y,
                radius,
                inv_w,
                rgb,
            })
            .filter(|s| s.x.is_finite() && s.y.is_finite())
    }
}

pub fn discs(fb: &mut Framebuffer, sprites: &[Sprite]) {
    if sprites.is_empty() {
        return;
    }
    let width = fb.width;
    let height = fb.height as f32;
    fb.color
        .par_chunks_mut(width * BAND)
        .zip(fb.depth.par_chunks(width * BAND))
        .enumerate()
        .for_each(|(band, (color, depth))| {
            let top = (band * BAND) as f32;
            let bottom = top + (color.len() / width) as f32;
            for s in sprites {
                if s.y + s.radius < top || s.y - s.radius >= bottom.min(height) {
                    continue;
                }
                disc(s, top as usize, width, color, depth);
            }
        });
}

fn disc(s: &Sprite, top: usize, width: usize, color: &mut [[u8; 4]], depth: &[f32]) {
    let rows = color.len() / width;
    let w = width as f32;
    if s.x + s.radius < 0.0 || s.x - s.radius >= w {
        return;
    }
    let x0 = (s.x - s.radius).floor().max(0.0) as usize;
    let x1 = ((s.x + s.radius).ceil() as usize).min(width - 1);
    let y0 = ((s.y - s.radius).floor().max(top as f32) as usize).max(top);
    let y1 = ((s.y + s.radius).ceil() as usize).min(top + rows - 1);
    let inv_r2 = 1.0 / (s.radius * s.radius);

    for py in y0..=y1 {
        let dy = py as f32 + 0.5 - s.y;
        let row = (py - top) * width;
        for px in x0..=x1 {
            let dx = px as f32 + 0.5 - s.x;
            let d2 = (dx * dx + dy * dy) * inv_r2;
            if d2 >= 1.0 {
                continue;
            }
            let i = row + px;
            if s.inv_w < depth[i] {
                continue;
            }
            let falloff = (1.0 - d2) * (1.0 - d2);
            let c = &mut color[i];
            c[0] = c[0].saturating_add((s.rgb.x * falloff) as u8);
            c[1] = c[1].saturating_add((s.rgb.y * falloff) as u8);
            c[2] = c[2].saturating_add((s.rgb.z * falloff) as u8);
        }
    }
}
