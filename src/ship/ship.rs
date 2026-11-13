use std::f32::consts::{PI, TAU};

use nalgebra_glm as glm;

use crate::math::{transform_point, Mat4, Vec3};

pub struct Ship {
    pub position: Vec3,
    pub velocity: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    right: Vec3,
    pub spin: Vec3,
    lean: [f32; 2],
    pub scale: f32,
}

impl Ship {
    pub fn new(position: Vec3, scale: f32) -> Self {
        Ship {
            position,
            velocity: Vec3::zeros(),
            forward: -Vec3::z(),
            up: Vec3::y(),
            right: Vec3::x(),
            spin: Vec3::zeros(),
            lean: [0.0; 2],
            scale,
        }
    }

    pub fn look_at(&mut self, target: Vec3) {
        let dir = target - self.position;
        if dir.norm() < 1e-4 {
            return;
        }
        self.forward = dir.normalize();
        self.up = if self.forward.y.abs() > 0.99 {
            Vec3::z()
        } else {
            Vec3::y()
        };
        self.lean = [0.0; 2];
        self.orthonormalize();
    }

    pub fn rotate(&mut self, pitch: f32, yaw: f32, roll: f32) {
        self.forward = glm::rotate_vec3(&self.forward, yaw, &self.up);
        self.right = glm::rotate_vec3(&self.right, yaw, &self.up);
        self.forward = glm::rotate_vec3(&self.forward, pitch, &self.right);
        self.up = glm::rotate_vec3(&self.up, pitch, &self.right);
        self.up = glm::rotate_vec3(&self.up, roll, &self.forward);
        self.right = glm::rotate_vec3(&self.right, roll, &self.forward);
        self.orthonormalize();
    }

    pub fn align(&mut self, forward: Vec3, up: Vec3, blend: f32) {
        let f = self.forward + (forward - self.forward) * blend;
        let u = self.up + (up - self.up) * blend;
        self.forward = if f.norm() > 1e-4 { f } else { forward };
        self.up = if u.norm() > 1e-4 { u } else { up };
        self.orthonormalize();
    }

    pub fn lean_by(&mut self, pitch: f32, roll: f32, held: [bool; 2], blend: f32) {
        self.lean[0] += pitch;
        self.lean[1] += roll;
        for (angle, held) in self.lean.iter_mut().zip(held) {
            *angle = (*angle + PI).rem_euclid(TAU) - PI;
            if !held {
                *angle -= *angle * blend;
            }
        }
    }

    pub fn settle(&mut self) {
        let (forward, up, right) = self.pose();
        self.forward = forward;
        self.up = up;
        self.right = right;
        self.lean = [0.0; 2];
        self.orthonormalize();
    }

    pub fn nose(&self) -> Vec3 {
        self.pose().0
    }

    fn pose(&self) -> (Vec3, Vec3, Vec3) {
        let [pitch, roll] = self.lean;
        if pitch == 0.0 && roll == 0.0 {
            return (self.forward, self.up, self.right);
        }
        let forward = glm::rotate_vec3(&self.forward, pitch, &self.right);
        let up = glm::rotate_vec3(&self.up, pitch, &self.right);
        let up = glm::rotate_vec3(&up, roll, &forward);
        let right = glm::rotate_vec3(&self.right, roll, &forward);
        (forward, up, right)
    }

    fn orthonormalize(&mut self) {
        self.forward = self.forward.normalize();
        let r = self.forward.cross(&self.up);
        self.right = if r.norm() > 1e-6 {
            r.normalize()
        } else {
            (self.right - self.forward * self.right.dot(&self.forward)).normalize()
        };
        self.up = self.right.cross(&self.forward).normalize();
    }

    pub fn point(&self, local: Vec3) -> Vec3 {
        transform_point(&self.model(Vec3::zeros()), &local)
    }

    pub fn model(&self, camera: Vec3) -> Mat4 {
        let (forward, up, right) = self.pose();
        let s = self.scale;
        let t = self.position - camera;
        let r = right * s;
        let u = up * s;
        let b = -forward * s;
        Mat4::new(
            r.x, u.x, b.x, t.x, r.y, u.y, b.y, t.y, r.z, u.z, b.z, t.z, 0.0, 0.0, 0.0, 1.0,
        )
    }
}
