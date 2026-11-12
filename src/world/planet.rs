use std::f32::consts::{PI, TAU};

use nalgebra_glm as glm;

use crate::math::{Mat4, Vec3};
use crate::render::mesh::{smooth_normals, Mesh};
use crate::render::shaders::ShaderKind;
use crate::world::icosphere::Spheres;
use crate::world::noise::{hash, Noise};
use crate::world::scatter::Instance;

const OCEAN_MARGIN: f32 = 0.08;
const RING_SEGMENTS: usize = 96;

pub enum Relief {
    Smooth,
    Volcanic,
    Continents { sea: f32 },
    Glacial,
    Cratered(Vec<Crater>),
}

pub struct Crater {
    center: Vec3,
    size: f32,
    reach: f32,
    depth: f32,
}

pub struct Orbit {
    pub radius: f32,
    pub speed: f32,
    pub phase: f32,
}

pub struct Spec {
    pub name: &'static str,
    pub radius: f32,
    pub mass: f32,
    pub orbit: Orbit,
    pub spin_speed: f32,
    pub tilt: f32,
    pub amp: f32,
    pub relief: Relief,
    pub shader: ShaderKind,
    pub seed: u32,
}

pub struct Ring {
    pub mesh: Mesh,
    pub shader: ShaderKind,
}

pub struct Planet {
    pub name: String,
    pub radius: f32,
    orbit_radius: f32,
    orbit_speed: f32,
    orbit_angle: f32,
    spin_speed: f32,
    pub spin: f32,
    pub tilt: f32,
    pub mass: f32,
    pub position: Vec3,
    amp: f32,
    pub lods: Vec<Mesh>,
    pub shader: ShaderKind,
    pub ring: Option<Ring>,
    pub instances: Vec<Instance>,
    relief: Relief,
    noise: Noise,
}

impl Planet {
    pub fn new(spec: Spec, spheres: &Spheres) -> Self {
        let noise = Noise::new(spec.seed);
        let amp = spec.amp;
        let lods = lod_meshes(spheres, |p| 1.0 + amp * spec.relief.eval(&noise, p));
        let mut planet = Planet {
            name: spec.name.to_string(),
            radius: spec.radius,
            orbit_radius: spec.orbit.radius,
            orbit_speed: spec.orbit.speed,
            orbit_angle: spec.orbit.phase,
            spin_speed: spec.spin_speed,
            spin: 0.0,
            tilt: spec.tilt,
            mass: spec.mass,
            position: Vec3::zeros(),
            amp,
            lods,
            shader: spec.shader,
            ring: None,
            instances: Vec::new(),
            relief: spec.relief,
            noise,
        };
        planet.place();
        planet
    }

    pub fn advance(&mut self, dt: f32) {
        self.orbit_angle = (self.orbit_angle + self.orbit_speed * dt) % TAU;
        self.spin = (self.spin + self.spin_speed * dt) % TAU;
        self.place();
    }

    pub fn frame(&self, camera: Vec3) -> Mat4 {
        glm::translation(&(self.position - camera))
            * glm::rotation(self.tilt, &Vec3::x_axis())
            * glm::rotation(self.spin, &Vec3::y_axis())
    }

    pub fn model(&self, camera: Vec3) -> Mat4 {
        self.frame(camera) * glm::scaling(&Vec3::repeat(self.radius))
    }

    pub fn relief(&self, dir: Vec3) -> f32 {
        let len = dir.norm();
        if len < 1e-8 {
            return 0.0;
        }
        self.relief.eval(&self.noise, dir / len)
    }

    pub fn surface_height(&self, dir: Vec3) -> f32 {
        self.radius * (1.0 + self.amp * self.relief(dir))
    }

    pub fn floor(&self) -> f32 {
        self.radius * (1.0 - self.amp)
    }

    pub fn lod_level(&self, distance: f32, pixel_scale: f32, thresholds: &[f32; 3]) -> usize {
        pick_lod(
            self.radius,
            distance,
            pixel_scale,
            thresholds,
            self.lods.len(),
        )
    }

    fn place(&mut self) {
        self.position = Vec3::new(
            self.orbit_radius * self.orbit_angle.cos(),
            0.0,
            -self.orbit_radius * self.orbit_angle.sin(),
        );
    }
}

impl Relief {
    fn eval(&self, n: &Noise, p: Vec3) -> f32 {
        let d = match self {
            Relief::Smooth => 0.0,
            Relief::Volcanic => volcanic(n, p),
            Relief::Continents { sea } => continents(n, p, *sea),
            Relief::Glacial => glacial(n, p),
            Relief::Cratered(craters) => cratered(n, p, craters),
        };
        d.clamp(-1.0, 1.0)
    }
}

fn volcanic(n: &Noise, p: Vec3) -> f32 {
    let ridge = n.ridged(p * 1.7, 5, 2.1, 0.5);
    let base = n.fbm(p * 0.9 + Vec3::repeat(41.3), 3, 2.0, 0.5);
    let d = ridge * 1.9 - 0.85 + base * 0.9;
    if d < -0.3 {
        -0.3 + (d + 0.3) * 0.15
    } else {
        d
    }
}

fn continents(n: &Noise, p: Vec3, sea: f32) -> f32 {
    let shape = n.fbm(p * 1.1, 4, 2.0, 0.5) * 2.4;
    let detail = n.fbm(p * 4.5 + Vec3::repeat(31.7), 4, 2.1, 0.5);
    let ridge = n.ridged(p * 2.6 + Vec3::repeat(7.3), 4, 2.1, 0.5);
    let inland = smooth(sea, sea + 0.4, shape);
    let d = shape * 0.55 + detail * 0.35 + inland * ridge * 0.9;
    if d < sea {
        sea - OCEAN_MARGIN
    } else {
        d
    }
}

fn glacial(n: &Noise, p: Vec3) -> f32 {
    let f = (n.fbm(p * 1.6, 5, 2.0, 0.5) * 2.4).clamp(-1.0, 1.0);
    let steps = 4.0;
    let s = (f + 1.0) * 0.5 * steps;
    let k = s.floor();
    let t = smooth(0.25, 0.75, s - k);
    let terraced = (k + t) / steps * 2.0 - 1.0;
    terraced + n.fbm(p * 7.0 + Vec3::repeat(13.1), 2, 2.0, 0.5) * 0.08
}

fn cratered(n: &Noise, p: Vec3, craters: &[Crater]) -> f32 {
    let mut d = n.fbm(p * 2.2, 5, 2.0, 0.5) * 1.2;
    for c in craters {
        let cos = p.dot(&c.center);
        if cos > c.reach {
            let s = cos.clamp(-1.0, 1.0).acos() / c.size;
            d += c.depth * crater_profile(s);
        }
    }
    d
}

fn crater_profile(s: f32) -> f32 {
    let mut v = 0.0;
    if s < 1.0 {
        let q = 1.0 - s * s;
        v -= q * q;
    }
    let r = (s - 1.0) / 0.35;
    if r.abs() < 1.0 {
        let q = 1.0 - r * r;
        v += 0.35 * q * q;
    }
    v
}

pub fn craters(seed: u32, count: u32) -> Vec<Crater> {
    (0..count)
        .map(|i| {
            let h = |k: u32| hash(seed ^ 0xC2A7_E115, i * 8 + k);
            let z = h(0) * 2.0 - 1.0;
            let phi = h(1) * TAU;
            let ring = (1.0 - z * z).max(0.0).sqrt();
            let size = 0.18 + 0.24 * h(2);
            Crater {
                center: Vec3::new(ring * phi.cos(), z, ring * phi.sin()),
                size,
                reach: (size * 1.35).min(PI).cos(),
                depth: 0.8 + 0.4 * h(3),
            }
        })
        .collect()
}

pub fn ring(inner: f32, outer: f32) -> Ring {
    let mut positions = Vec::with_capacity(RING_SEGMENTS * 4);
    let mut normals = Vec::with_capacity(RING_SEGMENTS * 4);
    for side in [1.0, -1.0] {
        for i in 0..RING_SEGMENTS {
            let a = i as f32 / RING_SEGMENTS as f32 * TAU;
            let (s, c) = a.sin_cos();
            positions.push(Vec3::new(inner * c, 0.0, inner * s));
            positions.push(Vec3::new(outer * c, 0.0, outer * s));
            normals.push(Vec3::new(0.0, side, 0.0));
            normals.push(Vec3::new(0.0, side, 0.0));
        }
    }
    let mut indices = Vec::with_capacity(RING_SEGMENTS * 12);
    let n = RING_SEGMENTS as u32;
    for i in 0..n {
        let j = (i + 1) % n;
        let (i0, o0, i1, o1) = (2 * i, 2 * i + 1, 2 * j, 2 * j + 1);
        indices.extend([i0, o1, o0, i0, i1, o1]);
        let b = 2 * n;
        indices.extend([b + i0, b + o0, b + o1, b + i0, b + o1, b + i1]);
    }
    Ring {
        mesh: Mesh::new(positions, normals, indices),
        shader: ShaderKind::Ring { inner, outer },
    }
}

pub fn lod_meshes(spheres: &Spheres, scale: impl Fn(Vec3) -> f32) -> Vec<Mesh> {
    let deformed: Vec<Vec3> = spheres.positions.iter().map(|&p| p * scale(p)).collect();
    spheres
        .levels
        .iter()
        .map(|level| {
            let positions = deformed[..level.vertices].to_vec();
            let normals = smooth_normals(&positions, &level.indices);
            Mesh::new(positions, normals, level.indices.clone())
        })
        .collect()
}

pub fn pick_lod(
    radius: f32,
    distance: f32,
    pixel_scale: f32,
    thresholds: &[f32; 3],
    count: usize,
) -> usize {
    let px = radius / distance.max(1e-3) * pixel_scale;
    let level = thresholds.iter().filter(|&&t| px >= t).count();
    level.min(count.saturating_sub(1))
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
