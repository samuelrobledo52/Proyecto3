use nalgebra_glm as glm;

use crate::math::{Mat4, Vec3};

pub struct Ship {
    pub position: Vec3,
    pub velocity: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    right: Vec3,
    pub turn: Vec3,
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
            turn: Vec3::zeros(),
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

    pub fn model(&self, camera: Vec3) -> Mat4 {
        let s = self.scale;
        let t = self.position - camera;
        let r = self.right * s;
        let u = self.up * s;
        let b = -self.forward * s;
        Mat4::new(
            r.x, u.x, b.x, t.x, r.y, u.y, b.y, t.y, r.z, u.z, b.z, t.z, 0.0, 0.0, 0.0, 1.0,
        )
    }
}
