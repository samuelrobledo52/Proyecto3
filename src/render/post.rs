use rayon::prelude::*;

use crate::math::{Mat4, Vec3, Vec4};
use crate::render::framebuffer::Framebuffer;

const ROWS: usize = 8;
const HALO_LEN: usize = 4096;
const HALO_SPAN: f32 = 8.0;
const MIN_RADIUS: f32 = 9.0;
const FRONT_BLOOM: f32 = 0.35;
const LEVELS: usize = 64;
const GRADE_LEN: usize = 512;
const VIGNETTE: f32 = 0.25;
const VIGNETTE_BLUE: f32 = 0.4;
const LIFT: [f32; 3] = [0.016, 0.012, 0.04];
const SHOULDER: f32 = 0.85;
const ROLL: f32 = 0.4;
const FADE_RATE: f32 = 10.0;
const CORONA: Vec3 = Vec3::new(1.0, 0.66, 0.32);
const RING: Vec3 = Vec3::new(1.0, 0.86, 0.62);

#[derive(Clone, Copy)]
pub struct SunSpot {
    x: f32,
    y: f32,
    radius: f32,
    near: f32,
}

#[derive(Clone, Copy)]
struct Halo {
    x: f32,
    y: f32,
    scale: f32,
    reach2: f32,
    near: f32,
    strength: u32,
    front: u32,
}

pub struct Post {
    width: usize,
    height: usize,
    vignette: Vec<u8>,
    grade: Vec<u8>,
    halo: Vec<[u32; 3]>,
    visible: f32,
}

pub fn sun_spot(
    view: &Mat4,
    projection: &Mat4,
    center: Vec3,
    radius: f32,
    width: usize,
    height: usize,
) -> Option<SunSpot> {
    let clip = projection * view * Vec4::new(center.x, center.y, center.z, 1.0);
    if clip.w <= 1e-4 {
        return None;
    }
    let distance = center.norm().max(1e-4);
    let pixel_scale = projection[(1, 1)] * height as f32 * 0.5;
    let gap = clip.w - radius;
    Some(SunSpot {
        x: (clip.x / clip.w * 0.5 + 0.5) * width as f32,
        y: (0.5 - clip.y / clip.w * 0.5) * height as f32,
        radius: radius / distance * pixel_scale,
        near: if gap > 1e-3 { 1.0 / gap } else { f32::INFINITY },
    })
}

impl Post {
    pub fn new(width: usize, height: usize) -> Self {
        let cx = width as f32 * 0.5;
        let cy = height as f32 * 0.5;
        let inv_diag = 1.0 / (cx * cx + cy * cy).sqrt();
        let vignette = (0..width * height)
            .map(|i| {
                let dx = (i % width) as f32 + 0.5 - cx;
                let dy = (i / width) as f32 + 0.5 - cy;
                let d = (dx * dx + dy * dy).sqrt() * inv_diag;
                (smooth(0.3, 1.0, d) * (LEVELS - 1) as f32 + 0.5) as u8
            })
            .collect();
        let grade = (0..LEVELS * 3 * GRADE_LEN)
            .map(|i| {
                let level = i / (3 * GRADE_LEN);
                let channel = (i / GRADE_LEN) % 3;
                let v = 1.0 - VIGNETTE * level as f32 / (LEVELS - 1) as f32;
                let v = if channel == 2 {
                    v + (1.0 - v) * VIGNETTE_BLUE
                } else {
                    v
                };
                let x = (i % GRADE_LEN) as f32 / 255.0 * v;
                let over = (x - SHOULDER).max(0.0);
                let f = x.min(SHOULDER) + over / (1.0 + over / ROLL);
                let dark = (1.0 - f).max(0.0);
                ((f + LIFT[channel] * dark * dark * dark).min(1.0) * 255.0 + 0.5) as u8
            })
            .collect();
        let halo = (0..HALO_LEN)
            .map(|i| {
                let t = ((i as f32 + 0.5) / HALO_LEN as f32).sqrt() * HALO_SPAN;
                let corona = if t < 1.0 {
                    0.45 * t.powi(4)
                } else {
                    0.45 * (-(t - 1.0) * 2.6).exp() + 0.2 * (-(t - 1.0) * 0.65).exp()
                };
                let ring = 0.045 * (-((t - 3.2) / 0.35).powi(2)).exp();
                let tail = 1.0 - smooth(HALO_SPAN * 0.8, HALO_SPAN, t);
                let rgb = (CORONA * corona + RING * ring) * (tail * 255.0);
                [rgb.x, rgb.y, rgb.z].map(|c| (c * 256.0) as u32)
            })
            .collect();
        Post {
            width,
            height,
            vignette,
            halo,
            grade,
            visible: 0.0,
        }
    }

    pub fn apply(&mut self, fb: &mut Framebuffer, sun: Option<SunSpot>, dt: f32) {
        if fb.width != self.width || fb.height != self.height {
            *self = Post::new(fb.width, fb.height);
        }
        let target = sun.map_or(0.0, |s| self.visibility(&fb.depth, &s));
        self.visible += (target - self.visible) * (1.0 - (-dt * FADE_RATE).exp());
        let halo = sun
            .filter(|_| self.visible > 1e-3)
            .map(|s| self.halo_for(&s));
        self.finish(&mut fb.color, &fb.depth, halo);
    }

    fn finish(&self, color: &mut [[u8; 4]], depth: &[f32], halo: Option<Halo>) {
        let width = self.width;
        let lut = &self.halo;
        let grade = &self.grade;
        color
            .par_chunks_mut(width * ROWS)
            .zip(depth.par_chunks(width * ROWS))
            .zip(self.vignette.par_chunks(width * ROWS))
            .enumerate()
            .for_each(|(band, ((color, depth), vignette))| {
                let top = band * ROWS;
                for (r, ((row, row_depth), row_vig)) in color
                    .chunks_exact_mut(width)
                    .zip(depth.chunks_exact(width))
                    .zip(vignette.chunks_exact(width))
                    .enumerate()
                {
                    let Some((h, x0, x1, dy)) = halo.and_then(|h| h.span(top + r, width)) else {
                        plain(row, row_vig, grade);
                        continue;
                    };
                    plain(&mut row[..x0], &row_vig[..x0], grade);
                    plain(&mut row[x1..], &row_vig[x1..], grade);
                    for (x, ((px, &d), &v)) in row[x0..x1]
                        .iter_mut()
                        .zip(&row_depth[x0..x1])
                        .zip(&row_vig[x0..x1])
                        .enumerate()
                    {
                        let dx = (x0 + x) as f32 + 0.5 - h.x;
                        let i = (((dx * dx + dy * dy) * h.scale) as usize).min(HALO_LEN - 1);
                        let w = if d > h.near { h.front } else { h.strength };
                        let g = lut[i];
                        tone(px, g.map(|c| ((c * w) >> 16) as usize), v, grade);
                    }
                }
            });
    }

    fn halo_for(&self, s: &SunSpot) -> Halo {
        let r = s.radius.max(MIN_RADIUS);
        let reach = r * HALO_SPAN;
        Halo {
            x: s.x,
            y: s.y,
            scale: HALO_LEN as f32 / (reach * reach),
            reach2: reach * reach,
            near: s.near,
            strength: (self.visible * 256.0) as u32,
            front: (self.visible * FRONT_BLOOM * 256.0) as u32,
        }
    }

    fn visibility(&self, depth: &[f32], s: &SunSpot) -> f32 {
        let w = self.width as f32;
        let h = self.height as f32;
        let out_x = (-s.x).max(s.x - w).max(0.0);
        let out_y = (-s.y).max(s.y - h).max(0.0);
        let out = (out_x * out_x + out_y * out_y).sqrt();
        let r = s.radius.max(MIN_RADIUS);
        let fade = 1.0 - smooth(0.0, r * 2.5 + 40.0, out);
        if fade <= 0.0 {
            return 0.0;
        }

        let probe = s.radius.max(1.0) * 0.7;
        let mut seen = 0;
        let mut open = 0;
        for k in 0..9 {
            let (dx, dy) = if k == 0 {
                (0.0, 0.0)
            } else {
                let a = (k - 1) as f32 * std::f32::consts::FRAC_PI_4;
                (a.cos() * probe, a.sin() * probe)
            };
            let px = s.x + dx;
            let py = s.y + dy;
            if px < 0.0 || py < 0.0 || px >= w || py >= h {
                continue;
            }
            seen += 1;
            let i = py as usize * self.width + px as usize;
            if depth.get(i).is_some_and(|&d| d <= s.near * 1.01) {
                open += 1;
            }
        }
        let open = if seen == 0 {
            1.0
        } else {
            open as f32 / seen as f32
        };
        fade * open
    }
}

impl Halo {
    fn span(self, y: usize, width: usize) -> Option<(Halo, usize, usize, f32)> {
        let dy = y as f32 + 0.5 - self.y;
        let left = self.reach2 - dy * dy;
        if left <= 0.0 {
            return None;
        }
        let half = left.sqrt();
        let x0 = (self.x - half).clamp(0.0, width as f32) as usize;
        let x1 = ((self.x + half).clamp(0.0, width as f32).ceil() as usize).max(x0);
        Some((self, x0, x1, dy))
    }
}

fn plain(row: &mut [[u8; 4]], vignette: &[u8], grade: &[u8]) {
    for (px, &v) in row.iter_mut().zip(vignette) {
        tone(px, [0; 3], v, grade);
    }
}

#[inline(always)]
fn tone(px: &mut [u8; 4], add: [usize; 3], level: u8, grade: &[u8]) {
    let base = level as usize * 3 * GRADE_LEN;
    let Some(table) = grade.get(base..base + 3 * GRADE_LEN) else {
        return;
    };
    let (red, rest) = table.split_at(GRADE_LEN);
    let (green, blue) = rest.split_at(GRADE_LEN);
    let at = |k: usize| (px[k] as usize + add[k]).min(GRADE_LEN - 1);
    *px = [red[at(0)], green[at(1)], blue[at(2)], 255];
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
