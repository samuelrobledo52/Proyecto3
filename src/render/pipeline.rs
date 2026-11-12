use rayon::prelude::*;

use crate::math::{transform_dir, transform_point, Mat4, Vec3, Vec4};
use crate::render::clip::{clip_near, ClipVertex};
use crate::render::mesh::Mesh;
use crate::render::shaders::{vertex_scalar, ShaderKind, Uniforms};
use crate::render::stats::Stats;

const PAR_VERTICES: usize = 2000;
const PAR_TRIS: usize = 4096;
const TRI_CHUNK: usize = 1024;

#[derive(Clone, Copy)]
pub struct ScreenVertex {
    pub x: f32,
    pub y: f32,
    pub inv_w: f32,
    pub normal: Vec3,
    pub world: Vec3,
    pub local: Vec3,
    pub color: Vec3,
    pub scalar: f32,
}

#[derive(Clone, Copy)]
pub struct ScreenTri {
    pub v: [ScreenVertex; 3],
    pub area: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub shader: ShaderKind,
}

#[derive(Clone, Copy)]
struct Viewport {
    width: f32,
    height: f32,
}

#[derive(Default)]
struct Batch {
    tris: Vec<ScreenTri>,
    stats: Stats,
}

pub struct Pipeline {
    pub tris: Vec<ScreenTri>,
    pub stats: Stats,
    pub culling: bool,
    view_projection: Mat4,
    planes: [Vec4; 6],
    viewport: Viewport,
    vertices: Vec<ClipVertex>,
    batches: Vec<Batch>,
}

impl Pipeline {
    pub fn new() -> Self {
        Pipeline {
            tris: Vec::new(),
            stats: Stats::default(),
            culling: true,
            view_projection: Mat4::identity(),
            planes: [Vec4::zeros(); 6],
            viewport: Viewport {
                width: 1.0,
                height: 1.0,
            },
            vertices: Vec::new(),
            batches: Vec::new(),
        }
    }

    pub fn begin(&mut self, view: &Mat4, projection: &Mat4, width: usize, height: usize) {
        self.view_projection = projection * view;
        self.planes = frustum_planes(&self.view_projection);
        self.viewport = Viewport {
            width: width as f32,
            height: height as f32,
        };
        self.tris.clear();
        self.stats = Stats::default();
    }

    pub fn submit(&mut self, mesh: &Mesh, model: &Mat4, shader: ShaderKind, u: &Uniforms) {
        if self.culling && !self.visible(mesh, model) {
            self.stats.culled += 1;
            self.stats.culled_tris += mesh.tris();
            return;
        }
        self.stats.objects += 1;
        let mvp = self.view_projection * model;

        let vertex = |i: usize| {
            let p = &mesh.positions[i];
            ClipVertex {
                pos: mvp * Vec4::new(p.x, p.y, p.z, 1.0),
                normal: transform_dir(model, &mesh.normals[i]),
                world: transform_point(model, p),
                local: *p,
                color: mesh.colors[i],
                scalar: vertex_scalar(shader, p, mesh.slopes[i], u),
            }
        };
        let count = mesh.positions.len();
        self.vertices.clear();
        if count >= PAR_VERTICES {
            self.vertices
                .par_extend((0..count).into_par_iter().map(&vertex));
        } else {
            self.vertices.extend((0..count).map(&vertex));
        }

        let vertices = &self.vertices;
        let viewport = self.viewport;
        if mesh.indices.len() < PAR_TRIS * 3 {
            for tri in mesh.indices.chunks_exact(3) {
                assemble(
                    tri,
                    vertices,
                    shader,
                    viewport,
                    &mut self.tris,
                    &mut self.stats,
                );
            }
            return;
        }

        let chunks = mesh.indices.len().div_ceil(TRI_CHUNK * 3);
        if self.batches.len() < chunks {
            self.batches.resize_with(chunks, Batch::default);
        }
        self.batches[..chunks]
            .par_iter_mut()
            .zip(mesh.indices.par_chunks(TRI_CHUNK * 3))
            .for_each(|(batch, indices)| {
                batch.tris.clear();
                batch.stats = Stats::default();
                for tri in indices.chunks_exact(3) {
                    assemble(
                        tri,
                        vertices,
                        shader,
                        viewport,
                        &mut batch.tris,
                        &mut batch.stats,
                    );
                }
            });
        for batch in &self.batches[..chunks] {
            self.tris.extend_from_slice(&batch.tris);
            self.stats.add_tris(&batch.stats);
        }
    }

    fn visible(&self, mesh: &Mesh, model: &Mat4) -> bool {
        let center = Vec4::new(model[(0, 3)], model[(1, 3)], model[(2, 3)], 1.0);
        let scale = (0..3)
            .map(|i| model.fixed_view::<3, 1>(0, i).norm())
            .fold(0.0, f32::max);
        let radius = mesh.bound * scale;
        self.planes
            .iter()
            .all(|plane| plane.dot(&center) >= -radius)
    }
}

fn assemble(
    tri: &[u32],
    vertices: &[ClipVertex],
    shader: ShaderKind,
    viewport: Viewport,
    out: &mut Vec<ScreenTri>,
    stats: &mut Stats,
) {
    stats.submitted += 1;
    let corners = [
        vertices[tri[0] as usize],
        vertices[tri[1] as usize],
        vertices[tri[2] as usize],
    ];

    if outside_frustum(&corners) {
        stats.outside += 1;
        return;
    }

    let face = (corners[1].world - corners[0].world).cross(&(corners[2].world - corners[0].world));
    if face.dot(&corners[0].world) >= 0.0 {
        stats.backface += 1;
        return;
    }

    if corners.iter().all(|c| c.near_distance() >= 0.0) {
        emit(&corners, viewport, shader, out, stats);
        return;
    }

    stats.clipped += 1;
    let mut polygon = [corners[0]; 4];
    let count = clip_near(&corners, &mut polygon);
    for i in 1..count.saturating_sub(1) {
        let fan = [polygon[0], polygon[i], polygon[i + 1]];
        emit(&fan, viewport, shader, out, stats);
    }
}

fn to_screen(v: &ClipVertex, viewport: Viewport) -> ScreenVertex {
    let inv_w = 1.0 / v.pos.w;
    ScreenVertex {
        x: (v.pos.x * inv_w * 0.5 + 0.5) * viewport.width,
        y: (0.5 - v.pos.y * inv_w * 0.5) * viewport.height,
        inv_w,
        normal: v.normal * inv_w,
        world: v.world * inv_w,
        local: v.local * inv_w,
        color: v.color * inv_w,
        scalar: v.scalar * inv_w,
    }
}

fn emit(
    c: &[ClipVertex; 3],
    viewport: Viewport,
    shader: ShaderKind,
    out: &mut Vec<ScreenTri>,
    stats: &mut Stats,
) {
    let mut v = [
        to_screen(&c[0], viewport),
        to_screen(&c[1], viewport),
        to_screen(&c[2], viewport),
    ];
    let mut area = edge(&v[0], &v[1], v[2].x, v[2].y);
    if area.abs() < 1e-6 {
        return;
    }
    if area < 0.0 {
        v.swap(1, 2);
        area = -area;
    }

    let min_x = v[0].x.min(v[1].x).min(v[2].x);
    let max_x = v[0].x.max(v[1].x).max(v[2].x);
    let min_y = v[0].y.min(v[1].y).min(v[2].y);
    let max_y = v[0].y.max(v[1].y).max(v[2].y);
    if max_x < 0.0 || min_x >= viewport.width || max_y < 0.0 || min_y >= viewport.height {
        return;
    }

    stats.rasterized += 1;
    out.push(ScreenTri {
        v,
        area,
        min_y,
        max_y,
        shader,
    });
}

pub fn edge(a: &ScreenVertex, b: &ScreenVertex, px: f32, py: f32) -> f32 {
    (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x)
}

fn frustum_planes(m: &Mat4) -> [Vec4; 6] {
    let row = |i: usize| Vec4::new(m[(i, 0)], m[(i, 1)], m[(i, 2)], m[(i, 3)]);
    let (r0, r1, r2, r3) = (row(0), row(1), row(2), row(3));
    [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r3 + r2, r3 - r2].map(|p| {
        let len = p.xyz().norm();
        if len > 1e-12 {
            p / len
        } else {
            p
        }
    })
}

fn outside_frustum(c: &[ClipVertex; 3]) -> bool {
    let all = |test: fn(&Vec4) -> bool| c.iter().all(|v| test(&v.pos));
    all(|p| p.x > p.w)
        || all(|p| p.x < -p.w)
        || all(|p| p.y > p.w)
        || all(|p| p.y < -p.w)
        || all(|p| p.z < -p.w)
        || all(|p| p.z > p.w)
}
