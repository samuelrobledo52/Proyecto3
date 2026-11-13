use std::f32::consts::PI;

use nalgebra_glm as glm;

use crate::config::Config;
use crate::math::{look_dir, Mat4, Vec3};
use crate::ship::flight::{ground, Input};
use crate::ship::ship::Ship;
use crate::world::system::System;

const CLEARANCE: f32 = 0.3;
const UP_FOLLOW: f32 = 3.0;

pub struct Camera {
    pub position: Vec3,
    forward: Vec3,
    up: Vec3,
    pub fov: f32,
    distance: f32,
    offset: Vec3,
    yaw: f32,
    pitch: f32,
    follow: f32,
    sensitivity: f32,
    min_distance: f32,
    max_distance: f32,
}

impl Camera {
    pub fn new(config: &Config) -> Self {
        let mut camera = Camera {
            position: Vec3::zeros(),
            forward: -Vec3::z(),
            up: Vec3::y(),
            fov: 70f32.to_radians(),
            distance: 4.0,
            offset: Vec3::new(0.0, 1.2, 4.0),
            yaw: 0.0,
            pitch: 0.0,
            follow: 6.0,
            sensitivity: 0.005,
            min_distance: 1.5,
            max_distance: 40.0,
        };
        camera.configure(config);
        camera
    }

    pub fn configure(&mut self, config: &Config) {
        self.fov = config
            .get("camera.fov", 70.0)
            .clamp(20.0, 120.0)
            .to_radians();
        self.distance = config.get("camera.distance", 4.0);
        self.follow = config.get("camera.follow", 6.0);
        self.sensitivity = config.get("camera.sensitivity", 0.005);
        self.min_distance = config.get("camera.min_distance", 1.5);
        self.max_distance = config.get("camera.max_distance", 40.0);
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);
    }

    pub fn view(&self) -> Mat4 {
        look_dir(self.forward, self.up)
    }

    pub fn look_at(&mut self, target: Vec3) {
        let dir = target - self.position;
        if dir.norm() > 1e-4 {
            self.forward = dir.normalize();
        }
        self.up = Vec3::y();
    }

    pub fn snap(&mut self, ship: &Ship) {
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.up = ship.up;
        self.offset = self.target_offset(ship);
        self.position = ship.position + self.offset;
        self.aim(ship);
    }

    pub fn follow(&mut self, ship: &Ship, input: &Input, system: &System, dt: f32) {
        if input.look {
            self.yaw -= input.look_x * self.sensitivity;
            self.pitch -= input.look_y * self.sensitivity;
            self.pitch = self.pitch.clamp(-1.2, 1.2);
            self.yaw = (self.yaw + PI).rem_euclid(2.0 * PI) - PI;
        } else {
            let decay = (-3.0 * dt).exp();
            self.yaw *= decay;
            self.pitch *= decay;
        }
        if input.zoom != 0.0 {
            self.distance = (self.distance * (-input.zoom * 0.12).exp())
                .clamp(self.min_distance, self.max_distance);
        }

        self.up = toward(
            self.up,
            ship.up,
            1.0 - (-UP_FOLLOW * dt).exp(),
            ship.forward,
        );
        let blend = 1.0 - (-self.follow * dt).exp();
        let target = self.target_offset(ship);
        if (target - self.offset).norm() > self.max_distance * 2.0 {
            self.offset = target;
            self.up = ship.up;
        }
        let length = self.offset.norm() + (target.norm() - self.offset.norm()) * blend;
        self.offset += (target - self.offset) * blend;
        let norm = self.offset.norm();
        self.offset = if norm > 1e-4 {
            self.offset * (length / norm)
        } else {
            target
        };
        self.position = ship.position + self.offset;
        if let Some(normal) = self.lift_out(system) {
            self.keep_away(ship, normal);
            self.lift_out(system);
        }
        self.offset = self.position - ship.position;
        self.aim(ship);
    }

    fn target_offset(&self, ship: &Ship) -> Vec3 {
        let back = -ship.forward * self.distance + self.up * (self.distance * 0.3);
        let turned = glm::rotate_vec3(&back, self.yaw, &self.up);
        let axis = turned.cross(&self.up);
        if axis.norm() < 1e-4 {
            return turned;
        }
        glm::rotate_vec3(&turned, self.pitch, &axis.normalize())
    }

    fn aim(&mut self, ship: &Ship) {
        let ahead = self.distance * 0.35 * self.yaw.cos().max(0.0) * self.pitch.cos();
        let dir = ship.position + ship.forward * ahead - self.position;
        if dir.norm() > 1e-4 {
            self.forward = dir.normalize();
        }
        if self.forward.cross(&self.up).norm() < 1e-3 {
            self.up = if self.forward.cross(&ship.up).norm() > 1e-3 {
                ship.up
            } else {
                -ship.forward
            };
        }
    }

    fn keep_away(&mut self, ship: &Ship, normal: Vec3) {
        let rel = self.position - ship.position;
        let keep = self.min_distance;
        if rel.norm() >= keep {
            return;
        }
        let rise = rel.dot(&normal);
        let flat = rel - normal * rise;
        let reach = keep * keep - rise * rise;
        if reach > 0.0 {
            let side = if flat.norm() > 1e-3 {
                flat.normalize()
            } else {
                -ship.forward
            };
            self.position = ship.position + normal * rise + side * reach.sqrt();
        }
    }

    fn lift_out(&mut self, system: &System) -> Option<Vec3> {
        let mut pushed = None;
        for planet in &system.planets {
            let d = self.position - planet.position;
            let dist = d.norm();
            if dist < 1e-4 {
                continue;
            }
            let dir = d / dist;
            let floor = ground(planet, dir) + CLEARANCE;
            if dist < floor {
                self.position = planet.position + dir * floor;
                pushed = Some(dir);
            }
        }
        let dist = self.position.norm();
        let floor = system.sun.radius + CLEARANCE;
        if dist > 1e-4 && dist < floor {
            pushed = Some(self.position / dist);
            self.position *= floor / dist;
        }
        pushed
    }
}

fn toward(from: Vec3, to: Vec3, blend: f32, spare: Vec3) -> Vec3 {
    let cross = from.cross(&to);
    let angle = cross.norm().atan2(from.dot(&to));
    let axis = if cross.norm() > 1e-5 {
        cross.normalize()
    } else if angle > 1.0 {
        spare.normalize()
    } else {
        return to;
    };
    glm::rotate_vec3(&from, angle * blend, &axis).normalize()
}
