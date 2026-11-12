use crate::math::Vec3;

const GRADS: [[f32; 3]; 16] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
    [1.0, 1.0, 0.0],
    [0.0, -1.0, 1.0],
    [-1.0, 1.0, 0.0],
    [0.0, -1.0, -1.0],
];

pub struct Noise {
    perm: [u8; 512],
}

impl Noise {
    pub fn new(seed: u32) -> Self {
        let mut table = [0u8; 256];
        for (i, v) in table.iter_mut().enumerate() {
            *v = i as u8;
        }
        let mut state = seed as u64 ^ 0x9E37_79B9_7F4A_7C15;
        for i in (1..256).rev() {
            state = splitmix(state);
            let j = (state % (i as u64 + 1)) as usize;
            table.swap(i, j);
        }
        let mut perm = [0u8; 512];
        for (i, v) in perm.iter_mut().enumerate() {
            *v = table[i & 255];
        }
        Noise { perm }
    }

    pub fn perlin3(&self, p: Vec3) -> f32 {
        let (ix, fx) = floor(p.x);
        let (iy, fy) = floor(p.y);
        let (iz, fz) = floor(p.z);
        let xi = (ix & 255) as usize;
        let yi = (iy & 255) as usize;
        let zi = (iz & 255) as usize;
        let x = p.x - fx;
        let y = p.y - fy;
        let z = p.z - fz;
        let u = fade(x);
        let v = fade(y);
        let w = fade(z);

        let perm = &self.perm;
        let at = |i: usize| perm[i & 511] as usize;
        let a = at(xi) + yi;
        let aa = at(a) + zi;
        let ab = at(a + 1) + zi;
        let b = at(xi + 1) + yi;
        let ba = at(b) + zi;
        let bb = at(b + 1) + zi;

        let g = |h: usize, x: f32, y: f32, z: f32| {
            let d = &GRADS[perm[h & 511] as usize & 15];
            d[0] * x + d[1] * y + d[2] * z
        };
        let x0 = lerp(u, g(aa, x, y, z), g(ba, x - 1.0, y, z));
        let x1 = lerp(u, g(ab, x, y - 1.0, z), g(bb, x - 1.0, y - 1.0, z));
        let x2 = lerp(u, g(aa + 1, x, y, z - 1.0), g(ba + 1, x - 1.0, y, z - 1.0));
        let x3 = lerp(
            u,
            g(ab + 1, x, y - 1.0, z - 1.0),
            g(bb + 1, x - 1.0, y - 1.0, z - 1.0),
        );
        lerp(w, lerp(v, x0, x1), lerp(v, x2, x3)).clamp(-1.0, 1.0)
    }

    pub fn fbm(&self, p: Vec3, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut norm = 0.0;
        for i in 0..octaves {
            let shift = Vec3::repeat(i as f32 * 17.31);
            sum += amp * self.perlin3(p * freq + shift);
            norm += amp;
            amp *= gain;
            freq *= lacunarity;
        }
        if norm > 0.0 {
            sum / norm
        } else {
            0.0
        }
    }

    pub fn ridged(&self, p: Vec3, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut norm = 0.0;
        let mut weight = 1.0;
        for i in 0..octaves {
            let shift = Vec3::repeat(i as f32 * 23.17);
            let ridge = 1.0 - self.perlin3(p * freq + shift).abs();
            let ridge = ridge * ridge * weight;
            weight = ridge.clamp(0.0, 1.0);
            sum += amp * ridge;
            norm += amp;
            amp *= gain;
            freq *= lacunarity;
        }
        if norm > 0.0 {
            sum / norm
        } else {
            0.0
        }
    }
}

pub fn hash(seed: u32, i: u32) -> f32 {
    let state = splitmix(((seed as u64) << 32) | i as u64);
    (state >> 40) as f32 / (1u64 << 24) as f32
}

fn splitmix(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub fn floor(x: f32) -> (i32, f32) {
    let i = x as i32;
    let f = i as f32;
    if f > x {
        (i - 1, f - 1.0)
    } else {
        (i, f)
    }
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}
