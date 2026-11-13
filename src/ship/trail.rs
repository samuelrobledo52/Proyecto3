use crate::math::{Mat4, Vec3, Vec4};
use crate::render::framebuffer::Framebuffer;
use crate::render::sprite::{discs, Sprite};
use crate::ship::ship::Ship;

const POOL: usize = 768;
const NOZZLES: [Vec3; 2] = [Vec3::new(-0.7, -0.35, 2.7), Vec3::new(0.7, -0.35, 2.7)];
const MIN_SPEED: f32 = 2.0;
const BASE_RATE: f32 = 60.0;
const RATE_PER_SPEED: f32 = 3.0;
const MAX_RATE: f32 = 450.0;
const BASE_LIFE: f32 = 0.25;
const LIFE_PER_SPEED: f32 = 0.004;
const MAX_LIFE: f32 = 0.75;
const BASE_SIZE: f32 = 0.25;
const SIZE_PER_SPEED: f32 = 0.002;
const INHERIT: f32 = 0.85;
const EJECT: f32 = 3.0;
const SPREAD: f32 = 0.6;
const DRAG: f32 = 2.5;
const MAX_GAP: f32 = 20.0;
const NEAR: f32 = 0.05;
const SHRINK: f32 = 0.85;
const GLOW: f32 = 0.85;
const CYAN: Vec3 = Vec3::new(0.35, 0.95, 1.0);
const PURPLE: Vec3 = Vec3::new(0.55, 0.12, 0.9);

#[derive(Clone, Copy)]
struct Particle {
    position: Vec3,
    velocity: Vec3,
    age: f32,
    life: f32,
    size: f32,
}

impl Particle {
    const DEAD: Particle = Particle {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 0.0, 0.0),
        age: 1.0,
        life: 0.0,
        size: 0.0,
    };

    fn alive(&self) -> bool {
        self.age < self.life
    }
}

pub struct Trail {
    particles: [Particle; POOL],
    sprites: Vec<Sprite>,
    next: usize,
    carry: f32,
    side: usize,
    last: Option<[Vec3; 2]>,
    seed: u32,
}

impl Trail {
    pub fn new() -> Self {
        Trail {
            particles: [Particle::DEAD; POOL],
            sprites: Vec::with_capacity(POOL),
            next: 0,
            carry: 0.0,
            side: 0,
            last: None,
            seed: 0x9e37_79b9,
        }
    }

    pub fn update(&mut self, ship: &Ship, boost: bool, speed: f32, dt: f32) {
        let drag = (-DRAG * dt).exp();
        for p in self.particles.iter_mut().filter(|p| p.alive()) {
            p.age += dt;
            p.position += p.velocity * dt;
            p.velocity *= drag;
        }

        let nozzles = NOZZLES.map(|n| ship.point(n));
        if !boost || speed < MIN_SPEED || dt <= 0.0 {
            self.last = None;
            self.carry = 0.0;
            return;
        }
        let from = match self.last {
            Some(prev) if (prev[0] - nozzles[0]).norm() < MAX_GAP => prev,
            _ => nozzles,
        };
        self.last = Some(nozzles);

        self.carry += (BASE_RATE + speed * RATE_PER_SPEED).min(MAX_RATE) * dt;
        let count = self.carry as usize;
        self.carry -= count as f32;
        let life = (BASE_LIFE + speed * LIFE_PER_SPEED).min(MAX_LIFE);
        let size = ship.scale * (BASE_SIZE + speed * SIZE_PER_SPEED);
        let push = ship.velocity * INHERIT - ship.nose() * EJECT;

        for k in 0..count {
            let f = (k as f32 + 0.5) / count as f32;
            let side = self.side;
            self.side = 1 - side;
            let start = from[side] + (nozzles[side] - from[side]) * f;
            let velocity = push + self.jitter() * SPREAD;
            let age = (1.0 - f) * dt;
            let life = life * (0.75 + 0.5 * self.random());
            self.particles[self.next] = Particle {
                position: start + velocity * age,
                velocity,
                age,
                life,
                size,
            };
            self.next = (self.next + 1) % POOL;
        }
    }

    pub fn draw(&mut self, fb: &mut Framebuffer, view: &Mat4, projection: &Mat4, camera: Vec3) {
        let view_projection = projection * view;
        let (width, height) = (fb.width as f32, fb.height as f32);
        let focal = projection[(1, 1)] * height * 0.5;
        self.sprites.clear();
        for p in self.particles.iter().filter(|p| p.alive()) {
            let offset = p.position - camera;
            let clip = view_projection * Vec4::new(offset.x, offset.y, offset.z, 1.0);
            if clip.w < NEAR {
                continue;
            }
            let inv_w = 1.0 / clip.w;
            let x = (clip.x * inv_w * 0.5 + 0.5) * width;
            let y = (0.5 - clip.y * inv_w * 0.5) * height;
            let t = (p.age / p.life).clamp(0.0, 1.0);
            let radius = p.size * (1.0 - SHRINK * t) * focal * inv_w;
            let tint = t * t * (3.0 - 2.0 * t);
            let fade = (1.0 - t) * (1.0 - t).sqrt();
            let rgb = (CYAN + (PURPLE - CYAN) * tint) * (fade * GLOW);
            self.sprites.extend(Sprite::new(x, y, radius, inv_w, rgb));
        }
        discs(fb, &self.sprites);
    }

    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }

    fn jitter(&mut self) -> Vec3 {
        Vec3::new(
            self.random() * 2.0 - 1.0,
            self.random() * 2.0 - 1.0,
            self.random() * 2.0 - 1.0,
        )
    }
}
