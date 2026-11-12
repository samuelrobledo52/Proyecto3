use crate::math::Vec3;
use crate::world::noise::{floor, Noise};

const DISCARD: [u8; 4] = [0, 0, 0, 0];
const RING_SPAN: f32 = 6.0;
const RING_SAMPLES: usize = 4096;
const CRATER_REACH: f32 = 0.68;
const AMBIENT: Vec3 = Vec3::new(0.13, 0.12, 0.21);
const SUN_LIGHT: Vec3 = Vec3::new(1.0, 0.95, 0.86);
const RIM: Vec3 = Vec3::new(1.0, 0.62, 0.3);

#[derive(Clone, Copy)]
pub enum ShaderKind {
    Lit { color: Vec3 },
    Sun { intensity: f32 },
    Lava { amp: f32, intensity: f32 },
    Terrain { amp: f32, sea: f32 },
    Ice { amp: f32 },
    Gas { intensity: f32 },
    Ring { inner: f32, outer: f32 },
    Ruins { amp: f32, craters: f32, bump: f32 },
}

pub struct Uniforms<'a> {
    pub light: Vec3,
    pub time: f32,
    pub noise: &'a Noise,
    pub ring: &'a [[f32; 2]],
}

pub struct Frag {
    pub normal: Vec3,
    pub world: Vec3,
    pub local: Vec3,
    pub color: Vec3,
    pub scalar: f32,
}

struct Dirs {
    n: Vec3,
    l: Vec3,
    v: Vec3,
}

pub fn vertex_scalar(kind: ShaderKind, local: &Vec3, slope: f32, u: &Uniforms) -> f32 {
    match kind {
        ShaderKind::Sun { .. } => {
            let t = u.time;
            let drift = Vec3::new(t * 0.07, t * 0.04, -t * 0.05);
            u.noise.perlin3(unit(*local) * 2.2 + drift)
        }
        ShaderKind::Ice { .. } => u.noise.perlin3(unit(*local) * 3.0),
        ShaderKind::Ruins { .. } => u.noise.perlin3(unit(*local) * 9.0),
        _ => slope,
    }
}

pub fn ring_bands(noise: &Noise) -> Vec<[f32; 2]> {
    (0..=RING_SAMPLES)
        .map(|i| {
            let r = i as f32 / RING_SAMPLES as f32 * RING_SPAN;
            let fine = noise.perlin3(Vec3::new(r * 38.0, 0.31, 0.77));
            let coarse = noise.perlin3(Vec3::new(r * 7.0, 0.53, 0.19));
            [fine * 0.9 + coarse * 0.8, coarse]
        })
        .collect()
}

fn ring_sample(bands: &[[f32; 2]], r: f32) -> [f32; 2] {
    let x = (r * (RING_SAMPLES as f32 / RING_SPAN)).max(0.0);
    let i = x as usize;
    let k = x - i as f32;
    match (bands.get(i), bands.get(i + 1)) {
        (Some(a), Some(b)) => [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k],
        _ => bands.last().copied().unwrap_or([0.0; 2]),
    }
}

pub fn shade(kind: ShaderKind, f: &Frag, u: &Uniforms) -> [u8; 4] {
    match kind {
        ShaderKind::Lit { color } => lit(color, f, u),
        ShaderKind::Sun { intensity } => sun(intensity, f, u),
        ShaderKind::Lava { amp, intensity } => lava(amp, intensity, f, u),
        ShaderKind::Terrain { amp, sea } => terrain(amp, sea, f, u),
        ShaderKind::Ice { amp } => ice(amp, f, u),
        ShaderKind::Gas { intensity } => gas(intensity, f, u),
        ShaderKind::Ring { inner, outer } => ring(inner, outer, f, u),
        ShaderKind::Ruins { amp, craters, bump } => ruins(amp, craters, bump, f, u),
    }
}

fn lit(color: Vec3, f: &Frag, u: &Uniforms) -> [u8; 4] {
    let d = dirs(f, u);
    let base = color.component_mul(&f.color);
    let ndl = d.n.dot(&d.l);
    let diffuse = toon(ndl.max(0.0));
    let half = unit(d.l + d.v);
    let gloss = d.n.dot(&half).max(0.0).powi(48);
    let spec = smooth(0.45, 0.6, gloss) * 0.45 * smooth(0.0, 0.1, ndl);
    let rim = (1.0 - d.n.dot(&d.v).max(0.0)).powi(3) * 0.5;
    let rgb = base.component_mul(&light(0.85 * diffuse))
        + Vec3::repeat(spec)
        + (base * 0.4 + RIM * 0.6) * rim;
    to_rgba(rgb)
}

fn sun(intensity: f32, f: &Frag, u: &Uniforms) -> [u8; 4] {
    let noise = u.noise;
    let t = u.time;
    let p = unit(f.local) * 2.2;
    let drift = Vec3::new(t * 0.07, t * 0.04, -t * 0.05);
    let warp = f.scalar;
    let q = p * 1.6 + Vec3::new(warp, -warp, warp * 0.5) * 2.6 - drift * 1.3;
    let plasma = noise.fbm(q, 2, 2.3, 0.55);
    let pulse = 0.06 * (t * 1.7 + warp * 6.0).sin();
    let heat = (0.55 + plasma * 1.4 + pulse).clamp(0.0, 1.0);
    let n = unit(f.normal);
    let v = unit(-f.world);
    let mu = n.dot(&v).max(0.0);
    let limb = 0.55 + 0.45 * mu.sqrt();
    let glow = Vec3::new(1.0, 0.55, 0.15) * (1.0 - mu).powi(3) * 0.9;
    to_rgba((sun_palette(heat) * limb + glow) * intensity)
}

fn sun_palette(x: f32) -> Vec3 {
    let c0 = Vec3::new(0.35, 0.02, 0.0);
    let c1 = Vec3::new(0.85, 0.2, 0.02);
    let c2 = Vec3::new(1.0, 0.6, 0.1);
    let c3 = Vec3::new(1.0, 0.9, 0.45);
    let c4 = Vec3::new(1.0, 1.0, 0.9);
    if x < 0.35 {
        mix(c0, c1, x / 0.35)
    } else if x < 0.65 {
        mix(c1, c2, (x - 0.35) / 0.3)
    } else if x < 0.85 {
        mix(c2, c3, (x - 0.65) / 0.2)
    } else {
        mix(c3, c4, (x - 0.85) / 0.15)
    }
}

fn lava(amp: f32, intensity: f32, f: &Frag, u: &Uniforms) -> [u8; 4] {
    let d = dirs(f, u);
    let noise = u.noise;
    let t = u.time;
    let p = unit(f.local);
    let h = height(f.local, amp);
    let ndl = d.n.dot(&d.l).max(0.0);
    let molten = 1.0 - smooth(-0.2, 0.0, h);

    let mut rgb = Vec3::zeros();
    if molten < 1.0 {
        let c = noise.perlin3(p * 14.0);
        let crack = 1.0 - smooth(0.0, 0.045, c.abs());
        let half = unit(d.l + d.v);
        let spec = d.n.dot(&half).max(0.0).powi(16) * 0.08;
        let rock =
            Vec3::new(0.17, 0.13, 0.12).component_mul(&light(0.88 * ndl)) + Vec3::repeat(spec);
        let seam = Vec3::new(1.0, 0.35, 0.05) * crack * 0.9 * intensity;
        rgb = rock + seam;
    }
    if molten > 0.0 {
        let flow = noise.fbm(
            p * 5.0 + Vec3::new(t * 0.12, -t * 0.05, t * 0.09),
            2,
            2.0,
            0.5,
        );
        let pulse = 0.8 + 0.2 * (t * 2.4 + flow * 9.0).sin();
        let heat = (0.5 + flow * 1.6).clamp(0.0, 1.0);
        let hot = mix(Vec3::new(0.55, 0.04, 0.0), Vec3::new(1.0, 0.45, 0.05), heat);
        let hot = mix(hot, Vec3::new(1.0, 0.85, 0.3), heat * heat);
        rgb = mix(rgb, hot * pulse * intensity, molten);
    }
    to_rgba(rgb)
}

fn terrain(amp: f32, sea: f32, f: &Frag, u: &Uniforms) -> [u8; 4] {
    let d = dirs(f, u);
    let p = unit(f.local);
    let jitter = u.noise.perlin3(p * 18.0) * 0.06;
    let h = height(f.local, amp) + jitter;
    let ndl_raw = d.n.dot(&d.l);
    let ndl = ndl_raw.max(0.0);

    let mut rgb = if h < sea {
        let depth = ((sea - h) / (sea + 1.0).max(0.05)).clamp(0.0, 1.0);
        let water = mix(
            Vec3::new(0.05, 0.38, 0.55),
            Vec3::new(0.02, 0.07, 0.28),
            smooth(0.0, 0.5, depth),
        );
        let half = unit(d.l + d.v);
        let spec = d.n.dot(&half).max(0.0).powi(80) * 0.9 * smooth(0.0, 0.1, ndl_raw);
        water.component_mul(&light(0.88 * ndl)) + Vec3::repeat(spec)
    } else {
        let land = ((h - sea) / (1.0 - sea).max(0.05)).clamp(0.0, 1.0);
        let slope = f.scalar;
        let mut c = Vec3::new(0.86, 0.78, 0.5);
        c = mix(c, Vec3::new(0.32, 0.62, 0.25), smooth(0.04, 0.1, land));
        c = mix(c, Vec3::new(0.1, 0.36, 0.15), smooth(0.3, 0.45, land));
        let rocky = smooth(0.6, 0.75, land).max(smooth(0.06, 0.16, slope));
        c = mix(c, Vec3::new(0.46, 0.41, 0.37), rocky);
        let snow = smooth(0.82, 0.9, land) * (1.0 - smooth(0.2, 0.35, slope));
        c = mix(c, Vec3::new(0.95, 0.96, 0.98), snow);
        c.component_mul(&light(0.88 * ndl))
    };

    let haze = (1.0 - d.n.dot(&d.v).max(0.0)).powi(3) * 0.4 * smooth(-0.3, 0.4, ndl_raw);
    rgb += Vec3::new(0.35, 0.55, 1.0) * haze;
    to_rgba(rgb)
}

fn ice(amp: f32, f: &Frag, u: &Uniforms) -> [u8; 4] {
    let d = dirs(f, u);
    let noise = u.noise;
    let p = unit(f.local);
    let h = height(f.local, amp);
    let tint = f.scalar;
    let line = noise.perlin3(p * 11.0 + Vec3::repeat(tint * 0.6));
    let crack = 1.0 - smooth(0.0, 0.035, line.abs());
    let deep = 1.0 - smooth(-0.6, 0.2, h + tint * 0.3);

    let mut base = mix(
        Vec3::new(0.82, 0.9, 0.97),
        Vec3::new(0.18, 0.45, 0.8),
        deep * 0.8,
    );
    base = mix(base, Vec3::new(0.08, 0.25, 0.45), crack * 0.8);

    let ndl = d.n.dot(&d.l);
    let wrap = ((ndl + 0.3) / 1.3).max(0.0);
    let sss = Vec3::new(0.05, 0.25, 0.5) * deep * 0.35 * (0.4 + 0.6 * wrap);
    let lit = base.component_mul(&light(0.88 * wrap)) + sss;

    let cos = d.n.dot(&d.v).max(0.0);
    let fresnel = 0.04 + 0.96 * (1.0 - cos).powi(5);
    let edge = Vec3::new(0.85, 0.95, 1.0) * (0.35 + 0.65 * wrap);
    let half = unit(d.l + d.v);
    let spec = d.n.dot(&half).max(0.0).powi(220) * 1.6 * smooth(0.0, 0.1, ndl);
    to_rgba(mix(lit, edge, fresnel) + Vec3::repeat(spec))
}

fn gas(intensity: f32, f: &Frag, u: &Uniforms) -> [u8; 4] {
    let d = dirs(f, u);
    let t = u.time;
    let p = unit(f.local);
    let turb = u.noise.fbm(
        Vec3::new(p.x * 2.5 + t * 0.03, p.y * 7.0, p.z * 2.5 - t * 0.02),
        2,
        2.0,
        0.5,
    );
    let lat = p.y + turb * 0.13;

    let palette = [
        Vec3::new(0.96, 0.91, 0.78),
        Vec3::new(0.98, 0.74, 0.6),
        Vec3::new(0.78, 0.68, 0.9),
        Vec3::new(0.62, 0.82, 0.95),
        Vec3::new(0.93, 0.8, 0.86),
    ];
    let x = lat * 4.0 + 10.0;
    let (k, floor_x) = floor(x);
    let i = k.rem_euclid(5) as usize;
    let band = mix(
        palette[i],
        palette[(i + 1) % 5],
        smooth(0.35, 0.65, x - floor_x),
    );
    let band = band * (0.92 + 0.08 * (lat * 60.0).sin());

    let wrap = ((d.n.dot(&d.l) + 0.45) / 1.45).max(0.0);
    let rim = 1.0 - d.n.dot(&d.v).max(0.0);
    let haze = rim * rim * rim.sqrt() * 0.25 * wrap;
    let rgb = band.component_mul(&light(0.92 * wrap)) * intensity + Vec3::new(0.8, 0.7, 1.0) * haze;
    to_rgba(rgb)
}

fn ring(inner: f32, outer: f32, f: &Frag, u: &Uniforms) -> [u8; 4] {
    let r = (f.local.x * f.local.x + f.local.z * f.local.z).sqrt();
    if r < inner || r > outer {
        return DISCARD;
    }
    let t = (r - inner) / (outer - inner);
    if (t - 0.6).abs() < 0.035 {
        return DISCARD;
    }
    let [bands, coarse] = ring_sample(u.ring, r);
    let fade = smooth(0.0, 0.06, t) * (1.0 - smooth(0.94, 1.0, t));
    let density = (0.55 + bands) * fade;
    if density < 0.3 {
        return DISCARD;
    }

    let color = mix(
        Vec3::new(0.85, 0.8, 0.7),
        Vec3::new(0.7, 0.65, 0.82),
        (t + coarse * 0.3).clamp(0.0, 1.0),
    ) * (0.55 + 0.45 * density.min(1.0));
    let n = unit(f.normal);
    let l = unit(u.light - f.world);
    let diffuse = n.dot(&l).abs();
    to_rgba(color.component_mul(&(AMBIENT * 1.8 + SUN_LIGHT * (0.75 * diffuse))))
}

fn ruins(amp: f32, craters: f32, bump: f32, f: &Frag, u: &Uniforms) -> [u8; 4] {
    let d = dirs(f, u);
    let dir = unit(f.local);
    let (dent, grad) = crater_field(dir * craters);

    let (t, b) = tangent(d.n);
    let (to, bo) = tangent(dir);
    let gt = grad.dot(&to);
    let gb = grad.dot(&bo);
    let bent = unit(d.n - (t * gt + b * gb) * bump);

    let h = height(f.local, amp);
    let grain = f.scalar * 0.5 + 0.5;
    let mut base = mix(Vec3::new(0.55, 0.3, 0.18), Vec3::new(0.72, 0.5, 0.3), grain);
    base *= 1.0 + 0.15 * h.max(0.0);
    base *= 1.0 + 0.2 * dent.clamp(-0.5, 0.3);

    let ndl = bent.dot(&d.l).max(0.0) * smooth(-0.05, 0.1, d.n.dot(&d.l));
    let half = unit(d.l + d.v);
    let spec = bent.dot(&half).max(0.0).powi(24) * 0.15 * ndl;
    to_rgba(base.component_mul(&light(0.88 * ndl)) + Vec3::repeat(spec))
}

fn crater_field(p: Vec3) -> (f32, Vec3) {
    let start = |x: f32| {
        let (i, f) = floor(x);
        if x - f < 0.5 {
            i - 1
        } else {
            i
        }
    };
    let (ox, oy, oz) = (start(p.x), start(p.y), start(p.z));
    let gap = |x: f32, o: i32| {
        let l = x - o as f32;
        let a = (l - 0.7).max(0.0);
        let b = (1.3 - l).max(0.0);
        [a * a, b * b]
    };
    let (gx, gy, gz) = (gap(p.x, ox), gap(p.y, oy), gap(p.z, oz));

    let mut h = 0.0;
    let mut g = Vec3::zeros();
    for k in 0..8i32 {
        let (i, j, l) = ((k & 1) as usize, ((k >> 1) & 1) as usize, (k >> 2) as usize);
        if gx[i] + gy[j] + gz[l] >= CRATER_REACH * CRATER_REACH {
            continue;
        }
        let (cx, cy, cz) = (ox + i as i32, oy + j as i32, oz + l as i32);
        let bits = hash3(cx, cy, cz);
        if bits & 3 == 0 {
            continue;
        }
        let c = Vec3::new(cx as f32, cy as f32, cz as f32);
        let jitter = Vec3::new(byte(bits >> 8), byte(bits >> 16), byte(bits >> 24)) * 0.4
            + Vec3::repeat(0.3);
        let radius = 0.22 + 0.26 * byte(bits.wrapping_mul(0x9e37_79b1) >> 24);
        let delta = p - (c + jitter);
        let reach = radius * 1.4;
        let d2 = delta.norm_squared();
        if d2 >= reach * reach {
            continue;
        }
        let dist = d2.sqrt();
        let s = dist / radius;
        let (v, dv) = crater_profile(s);
        h += v * radius;
        g += delta * (dv / dist.max(1e-5));
    }
    (h, g)
}

fn crater_profile(s: f32) -> (f32, f32) {
    let mut v = 0.0;
    let mut dv = 0.0;
    if s < 1.0 {
        let q = 1.0 - s * s;
        v -= q * q;
        dv += 4.0 * s * q;
    }
    let w = 0.4;
    let k = 0.35;
    let r = (s - 1.0) / w;
    if r.abs() < 1.0 {
        let q = 1.0 - r * r;
        v += k * q * q;
        dv -= 4.0 * k * r * q / w;
    }
    (v, dv)
}

fn hash3(x: i32, y: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ (h >> 15)
}

fn byte(bits: u32) -> f32 {
    (bits & 255) as f32 * (1.0 / 255.0)
}

fn tangent(n: Vec3) -> (Vec3, Vec3) {
    let s = n.x * n.x + n.z * n.z;
    let t = if s < 1e-8 {
        Vec3::x()
    } else {
        Vec3::new(n.z, 0.0, -n.x) / s.sqrt()
    };
    (t, n.cross(&t))
}

fn dirs(f: &Frag, u: &Uniforms) -> Dirs {
    Dirs {
        n: unit(f.normal),
        l: unit(u.light - f.world),
        v: unit(-f.world),
    }
}

fn height(local: Vec3, amp: f32) -> f32 {
    ((local.norm() - 1.0) / amp.max(1e-4)).clamp(-1.0, 1.0)
}

fn light(k: f32) -> Vec3 {
    AMBIENT + SUN_LIGHT * k
}

fn toon(d: f32) -> f32 {
    let x = d * 3.0;
    let (_, step) = floor(x);
    ((step + smooth(0.42, 0.58, x - step)) / 3.0).min(1.0)
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

fn unit(v: Vec3) -> Vec3 {
    let len2 = v.norm_squared();
    if len2 > 1e-24 {
        v * (1.0 / len2.sqrt())
    } else {
        v
    }
}

fn to_rgba(rgb: Vec3) -> [u8; 4] {
    [
        (rgb.x.clamp(0.0, 1.0) * 255.0) as u8,
        (rgb.y.clamp(0.0, 1.0) * 255.0) as u8,
        (rgb.z.clamp(0.0, 1.0) * 255.0) as u8,
        255,
    ]
}
