use crate::math::{Vec3, Vec4};

#[derive(Clone, Copy)]
pub struct ClipVertex {
    pub pos: Vec4,
    pub normal: Vec3,
    pub world: Vec3,
    pub local: Vec3,
    pub color: Vec3,
    pub scalar: f32,
}

impl ClipVertex {
    fn lerp(&self, other: &ClipVertex, t: f32) -> ClipVertex {
        ClipVertex {
            pos: self.pos + (other.pos - self.pos) * t,
            normal: self.normal + (other.normal - self.normal) * t,
            world: self.world + (other.world - self.world) * t,
            local: self.local + (other.local - self.local) * t,
            color: self.color + (other.color - self.color) * t,
            scalar: self.scalar + (other.scalar - self.scalar) * t,
        }
    }

    pub fn near_distance(&self) -> f32 {
        self.pos.z + self.pos.w
    }
}

pub fn clip_near(tri: &[ClipVertex; 3], out: &mut [ClipVertex; 4]) -> usize {
    let mut count = 0;
    for i in 0..3 {
        let current = &tri[i];
        let next = &tri[(i + 1) % 3];
        let d_current = current.near_distance();
        let d_next = next.near_distance();

        if d_current >= 0.0 {
            out[count] = *current;
            count += 1;
        }
        if (d_current >= 0.0) != (d_next >= 0.0) {
            let t = d_current / (d_current - d_next);
            out[count] = current.lerp(next, t);
            count += 1;
        }
    }
    count
}
