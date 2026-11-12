use std::f32::consts::TAU;

use crate::math::{Mat4, Vec3, Vec4};
use crate::world::noise::hash;

const STARS: u32 = 1500;
const BAND_SHARE: f32 = 0.35;
const DUST: u32 = 2600;

struct Star {
    dir: Vec3,
    color: [f32; 3],
    big: bool,
    phase: f32,
    rate: f32,
}

pub struct Skybox {
    stars: Vec<Star>,
}

impl Skybox {
    pub fn new(seed: u32) -> Self {
        let pole = Vec3::new(0.35, 0.86, -0.37).normalize();
        let (u, v) = basis(pole);
        let mut stars = Vec::with_capacity((STARS + DUST) as usize);
        let s = seed ^ 0x5741_5253;

        for i in 0..STARS {
            let h = |k: u32| hash(s, i * 8 + k);
            let dir = if h(0) < BAND_SHARE {
                band_dir(h(1), h(2), h(3), h(4), pole, u, v)
            } else {
                sphere_dir(h(1), h(2))
            };
            let bright = h(5).powi(3) * 0.85 + 0.15;
            let tint = h(6);
            let rgb = if tint < 0.18 {
                [0.68, 0.8, 1.0]
            } else if tint < 0.3 {
                [1.0, 0.8, 0.52]
            } else {
                [1.0, 0.97, 0.92]
            };
            stars.push(Star {
                dir,
                color: rgb.map(|c| c * bright * 255.0),
                big: bright > 0.72,
                phase: h(7) * TAU,
                rate: 1.0 + 2.5 * hash(s ^ 0x77, i),
            });
        }

        for i in 0..DUST {
            let h = |k: u32| hash(s ^ 0xD057, i * 8 + k);
            let dir = band_dir(h(0), h(1), h(2), h(3), pole, u, v);
            let level = 14.0 + 22.0 * h(4);
            let warm = h(5) * 0.2;
            stars.push(Star {
                dir,
                color: [level * (0.85 + warm), level * 0.85, level],
                big: h(6) < 0.25,
                phase: 0.0,
                rate: 0.0,
            });
        }
        Skybox { stars }
    }

    pub fn draw(
        &self,
        color: &mut [[u8; 4]],
        width: usize,
        height: usize,
        view_projection: &Mat4,
        time: f32,
    ) {
        let w = width as f32;
        let h = height as f32;
        for star in &self.stars {
            let d = star.dir;
            let clip = view_projection * Vec4::new(d.x, d.y, d.z, 1.0);
            if clip.w <= 1e-6 {
                continue;
            }
            let nx = clip.x / clip.w;
            let ny = clip.y / clip.w;
            if nx.abs() > 1.0 || ny.abs() > 1.0 {
                continue;
            }
            let x = ((nx * 0.5 + 0.5) * w) as usize;
            let y = ((0.5 - ny * 0.5) * h) as usize;
            let flicker = if star.rate > 0.0 {
                0.82 + 0.18 * (time * star.rate + star.phase).sin()
            } else {
                1.0
            };
            let rgb = star.color.map(|c| c * flicker);
            let size = if star.big { 2 } else { 1 };
            for py in y..(y + size).min(height) {
                for px in x..(x + size).min(width) {
                    for (dst, add) in color[py * width + px].iter_mut().zip(rgb) {
                        *dst = (*dst as f32 + add).min(255.0) as u8;
                    }
                }
            }
        }
    }
}

fn sphere_dir(a: f32, b: f32) -> Vec3 {
    let z = a * 2.0 - 1.0;
    let phi = b * TAU;
    let r = (1.0 - z * z).max(0.0).sqrt();
    Vec3::new(r * phi.cos(), z, r * phi.sin())
}

fn band_dir(a: f32, b: f32, c: f32, e: f32, pole: Vec3, u: Vec3, v: Vec3) -> Vec3 {
    let angle = a * TAU;
    let spread = (b + c + e - 1.5) * 0.16;
    (u * angle.cos() + v * angle.sin() + pole * spread).normalize()
}

fn basis(n: Vec3) -> (Vec3, Vec3) {
    let helper = if n.x.abs() < 0.9 {
        Vec3::x()
    } else {
        Vec3::y()
    };
    let u = n.cross(&helper).normalize();
    (u, n.cross(&u))
}
