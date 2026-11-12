use std::f32::consts::{PI, TAU};

use nalgebra_glm as glm;

use crate::math::{transform_dir, transform_point, Mat4, Vec3, Vec4};
use crate::world::noise::{hash, Noise};
use crate::world::planet::Planet;

pub const MODELS: [&str; 4] = ["tree", "rock", "tower", "crystal"];
pub const TREE: usize = 0;
pub const ROCK: usize = 1;
pub const TOWER: usize = 2;
pub const CRYSTAL: usize = 3;

const OVERSAMPLE: u32 = 20;
const MAX_CANDIDATES: u32 = 40_000;
const GOLDEN: f32 = 2.399_963;
const JITTER: f32 = 0.8;
const UPRIGHT: f32 = 0.7;
const SINK: f32 = 0.02;
const FOOTPRINT: f32 = 0.3;
const STEP: f32 = 0.025;

pub struct Rule {
    pub kind: usize,
    pub count: u32,
    pub size: f32,
    pub heights: [f32; 2],
    pub max_slope: f32,
    pub noise_freq: f32,
    pub noise_min: f32,
}

pub struct Instance {
    pub kind: usize,
    pub local: Mat4,
    dir: Vec3,
    base: Vec3,
    scale: f32,
}

pub struct Sight {
    center: Vec3,
    distance: f32,
    angle: f32,
    floor: f32,
}

pub struct View {
    pub near: f32,
    pub far: f32,
    pub lod: bool,
    pub horizon: bool,
}

#[derive(PartialEq)]
pub enum Pick {
    High,
    Low,
    Hidden { low: bool },
    Far,
}

pub fn scatter(planet: &Planet, rules: &[Rule], seed: u32) -> Vec<Instance> {
    let mut out: Vec<Instance> = Vec::new();
    for (k, rule) in rules.iter().enumerate() {
        let k = k as u32;
        let salt = seed.wrapping_mul(0x9E37_79B1) ^ k.wrapping_mul(0x85EB_CA6B).wrapping_add(1);
        let noise = Noise::new(seed.wrapping_add(31 * k + 1));
        let n = rule
            .count
            .saturating_mul(OVERSAMPLE)
            .clamp(1, MAX_CANDIDATES);
        let turn = lattice_turn(salt);
        let stride = coprime(n);
        let spacing = (4.0 * PI / n as f32).sqrt();
        let mut placed = 0;
        for i in 0..n {
            if placed >= rule.count {
                break;
            }
            let j = ((i as u64 * stride as u64) % n as u64) as u32;
            let h = |c: u32| hash(salt, j * 8 + c);
            let dir = candidate(j, n, spacing, [h(0), h(1)], &turn);
            let height = planet.relief(dir);
            if height < rule.heights[0] || height > rule.heights[1] {
                continue;
            }
            let class = noise.fbm(dir * rule.noise_freq, 3, 2.0, 0.5) + (h(2) - 0.5) * 0.15;
            if class < rule.noise_min {
                continue;
            }
            let normal = terrain_normal(planet, dir);
            if 1.0 - normal.dot(&dir) > rule.max_slope {
                continue;
            }
            let scale = rule.size * (0.6 + 0.8 * h(3));
            let surface = planet.surface_height(dir);
            if crowded(&out, dir, scale, surface) {
                continue;
            }
            let base = dir * (surface - SINK * planet.radius);
            out.push(Instance {
                kind: rule.kind,
                local: orient(dir, normal, base, scale, h(4) * TAU),
                dir,
                base,
                scale,
            });
            placed += 1;
        }
    }
    out
}

pub fn census(instances: &[Instance]) -> [usize; 4] {
    let mut count = [0; 4];
    for inst in instances {
        if let Some(c) = count.get_mut(inst.kind) {
            *c += 1;
        }
    }
    count
}

pub fn sight(frame: &Mat4, floor: f32) -> Sight {
    let center = Vec3::new(frame[(0, 3)], frame[(1, 3)], frame[(2, 3)]);
    let distance = center.norm();
    let angle = if distance > floor {
        (floor / distance).acos()
    } else {
        PI
    };
    Sight {
        center,
        distance,
        angle,
        floor,
    }
}

impl Instance {
    pub fn pick(&self, frame: &Mat4, sight: &Sight, bound: f32, view: &View) -> Pick {
        let level = if view.lod {
            let distance = transform_point(frame, &self.base).norm() / self.scale.max(1e-3);
            if distance > view.far {
                return Pick::Far;
            }
            if distance > view.near {
                Pick::Low
            } else {
                Pick::High
            }
        } else {
            Pick::High
        };
        if view.horizon && sight.angle < PI {
            let up = transform_dir(frame, &self.dir);
            let cos = -up.dot(&sight.center) / sight.distance.max(1e-6);
            let top = self.base.norm() + bound * self.scale;
            let reach = (sight.floor / top).min(1.0).acos();
            if cos.clamp(-1.0, 1.0).acos() > sight.angle + reach {
                return Pick::Hidden {
                    low: level == Pick::Low,
                };
            }
        }
        level
    }
}

fn candidate(j: u32, n: u32, spacing: f32, jitter: [f32; 2], turn: &Mat4) -> Vec3 {
    let z = 1.0 - (2 * j + 1) as f32 / n as f32;
    let ring = (1.0 - z * z).max(0.0).sqrt();
    let (s, c) = (j as f32 * GOLDEN).sin_cos();
    let p = Vec3::new(ring * c, z, ring * s);
    let (t, b) = tangents(p);
    let shift = (t * (jitter[0] - 0.5) + b * (jitter[1] - 0.5)) * spacing * JITTER;
    transform_dir(turn, &(p + shift).normalize())
}

fn lattice_turn(salt: u32) -> Mat4 {
    let z = hash(salt, 0xFFFF_0001) * 2.0 - 1.0;
    let phi = hash(salt, 0xFFFF_0002) * TAU;
    let ring = (1.0 - z * z).max(0.0).sqrt();
    let axis = Vec3::new(ring * phi.cos(), z, ring * phi.sin());
    glm::rotation(hash(salt, 0xFFFF_0003) * TAU, &axis)
}

fn coprime(n: u32) -> u32 {
    let mut s = ((n as f32 * 0.618_034) as u32).max(1);
    while gcd(s, n) != 1 {
        s += 1;
    }
    s
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn tangents(n: Vec3) -> (Vec3, Vec3) {
    let axis = if n.y.abs() < 0.9 {
        Vec3::y()
    } else {
        Vec3::x()
    };
    let t = axis.cross(&n).normalize();
    (t, n.cross(&t))
}

fn terrain_normal(planet: &Planet, dir: Vec3) -> Vec3 {
    let (t, b) = tangents(dir);
    let at = |d: Vec3| {
        let d = d.normalize();
        d * planet.surface_height(d)
    };
    let dx = at(dir + t * STEP) - at(dir - t * STEP);
    let dy = at(dir + b * STEP) - at(dir - b * STEP);
    let n = dx.cross(&dy).normalize();
    if n.dot(&dir) < 0.0 {
        -n
    } else {
        n
    }
}

fn crowded(placed: &[Instance], dir: Vec3, scale: f32, surface: f32) -> bool {
    placed
        .iter()
        .any(|o| (o.dir - dir).norm() * surface < FOOTPRINT * (scale + o.scale))
}

fn orient(dir: Vec3, normal: Vec3, base: Vec3, scale: f32, yaw: f32) -> Mat4 {
    let up = (dir * UPRIGHT + normal * (1.0 - UPRIGHT)).normalize();
    let (t, b) = tangents(up);
    let (s, c) = yaw.sin_cos();
    let x = t * c + b * s;
    let z = x.cross(&up);
    let col = |v: Vec3| Vec4::new(v.x, v.y, v.z, 0.0) * scale;
    Mat4::from_columns(&[
        col(x),
        col(up),
        col(z),
        Vec4::new(base.x, base.y, base.z, 1.0),
    ])
}
