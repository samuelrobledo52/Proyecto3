use rayon::prelude::*;

use crate::render::framebuffer::Framebuffer;
use crate::render::pipeline::{edge, ScreenTri, ScreenVertex};
use crate::render::shaders::{shade, Frag, Uniforms};

const BAND: usize = 8;

pub struct Raster {
    bins: Vec<Vec<u32>>,
}

impl Raster {
    pub fn new() -> Self {
        Raster { bins: Vec::new() }
    }

    pub fn draw(&mut self, fb: &mut Framebuffer, tris: &[ScreenTri], uniforms: &Uniforms) {
        let width = fb.width;
        let height = fb.height;
        let band_count = height.div_ceil(BAND);
        self.bins.resize_with(band_count, Vec::new);
        for bin in &mut self.bins {
            bin.clear();
        }

        for (i, tri) in tris.iter().enumerate() {
            let first = (tri.min_y.max(0.0) as usize) / BAND;
            let last = (tri.max_y.min(height as f32 - 1.0).max(0.0) as usize) / BAND;
            for bin in &mut self.bins[first..=last.min(band_count - 1)] {
                bin.push(i as u32);
            }
        }

        fb.color
            .par_chunks_mut(width * BAND)
            .zip(fb.depth.par_chunks_mut(width * BAND))
            .zip(self.bins.par_iter())
            .enumerate()
            .for_each(|(band, ((color, depth), bin))| {
                let top = band * BAND;
                let rows = color.len() / width;
                for &i in bin {
                    raster_triangle(&tris[i as usize], top, rows, width, color, depth, uniforms);
                }
            });
    }
}

fn is_top_left(a: &ScreenVertex, b: &ScreenVertex) -> bool {
    let dy = b.y - a.y;
    let dx = b.x - a.x;
    (dy == 0.0 && dx > 0.0) || dy < 0.0
}

fn covers(w: f32, top_left: bool) -> bool {
    w > 0.0 || (w == 0.0 && top_left)
}

fn raster_triangle(
    tri: &ScreenTri,
    top: usize,
    rows: usize,
    width: usize,
    color: &mut [[u8; 4]],
    depth: &mut [f32],
    uniforms: &Uniforms,
) {
    let [a, b, c] = &tri.v;

    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as usize;
    let max_x = (a.x.max(b.x).max(c.x).ceil() as usize).min(width - 1);
    let min_y = (tri.min_y.floor().max(top as f32) as usize).max(top);
    let max_y = (tri.max_y.ceil() as usize).min(top + rows - 1);
    if min_x > max_x || min_y > max_y {
        return;
    }

    let tl0 = is_top_left(b, c);
    let tl1 = is_top_left(c, a);
    let tl2 = is_top_left(a, b);
    let inv_area = 1.0 / tri.area;

    let start_x = min_x as f32 + 0.5;
    let step0 = -(c.y - b.y);
    let step1 = -(a.y - c.y);
    let step2 = -(b.y - a.y);

    for y in min_y..=max_y {
        let py = y as f32 + 0.5;
        let mut w0 = edge(b, c, start_x, py);
        let mut w1 = edge(c, a, start_x, py);
        let mut w2 = edge(a, b, start_x, py);
        let row = (y - top) * width;

        for x in min_x..=max_x {
            if covers(w0, tl0) && covers(w1, tl1) && covers(w2, tl2) {
                let l0 = w0 * inv_area;
                let l1 = w1 * inv_area;
                let l2 = w2 * inv_area;
                let inv_w = l0 * a.inv_w + l1 * b.inv_w + l2 * c.inv_w;
                let index = row + x;
                if inv_w > depth[index] {
                    let w = 1.0 / inv_w;
                    let frag = Frag {
                        normal: (a.normal * l0 + b.normal * l1 + c.normal * l2) * w,
                        world: (a.world * l0 + b.world * l1 + c.world * l2) * w,
                        local: (a.local * l0 + b.local * l1 + c.local * l2) * w,
                        color: (a.color * l0 + b.color * l1 + c.color * l2) * w,
                        scalar: (a.scalar * l0 + b.scalar * l1 + c.scalar * l2) * w,
                    };
                    let rgba = shade(tri.shader, &frag, uniforms);
                    if rgba[3] != 0 {
                        color[index] = rgba;
                        depth[index] = inv_w;
                    }
                }
            }
            w0 += step0;
            w1 += step1;
            w2 += step2;
        }
    }
}
