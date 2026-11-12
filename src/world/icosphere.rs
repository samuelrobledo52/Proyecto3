use std::collections::HashMap;

use crate::math::Vec3;

const LOD_SUBDIVISIONS: [u32; 4] = [2, 3, 4, 5];

pub struct Spheres {
    pub positions: Vec<Vec3>,
    pub levels: Vec<Level>,
}

pub struct Level {
    pub vertices: usize,
    pub indices: Vec<u32>,
}

impl Spheres {
    pub fn new() -> Self {
        let (mut positions, mut indices) = icosahedron();
        let mut levels = Vec::with_capacity(LOD_SUBDIVISIONS.len());
        let mut depth = 0;
        for &target in &LOD_SUBDIVISIONS {
            while depth < target {
                indices = subdivide(&mut positions, &indices);
                depth += 1;
            }
            levels.push(Level {
                vertices: positions.len(),
                indices: indices.clone(),
            });
        }
        Spheres { positions, levels }
    }
}

fn icosahedron() -> (Vec<Vec3>, Vec<u32>) {
    let t = (1.0 + 5.0f32.sqrt()) * 0.5;
    let positions: Vec<Vec3> = [
        (-1.0, t, 0.0),
        (1.0, t, 0.0),
        (-1.0, -t, 0.0),
        (1.0, -t, 0.0),
        (0.0, -1.0, t),
        (0.0, 1.0, t),
        (0.0, -1.0, -t),
        (0.0, 1.0, -t),
        (t, 0.0, -1.0),
        (t, 0.0, 1.0),
        (-t, 0.0, -1.0),
        (-t, 0.0, 1.0),
    ]
    .iter()
    .map(|&(x, y, z)| Vec3::new(x, y, z).normalize())
    .collect();

    let base: [[u32; 3]; 20] = [
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];

    let mut indices = Vec::with_capacity(60);
    for [a, b, c] in base {
        let pa = positions[a as usize];
        let face = (positions[b as usize] - pa).cross(&(positions[c as usize] - pa));
        if face.dot(&pa) >= 0.0 {
            indices.extend([a, b, c]);
        } else {
            indices.extend([a, c, b]);
        }
    }
    (positions, indices)
}

fn subdivide(positions: &mut Vec<Vec3>, indices: &[u32]) -> Vec<u32> {
    let mut cache: HashMap<(u32, u32), u32> = HashMap::with_capacity(indices.len() * 3 / 2);
    let mut next = Vec::with_capacity(indices.len() * 4);
    for tri in indices.chunks_exact(3) {
        let [a, b, c] = [tri[0], tri[1], tri[2]];
        let ab = midpoint(positions, &mut cache, a, b);
        let bc = midpoint(positions, &mut cache, b, c);
        let ca = midpoint(positions, &mut cache, c, a);
        next.extend([a, ab, ca, b, bc, ab, c, ca, bc, ab, bc, ca]);
    }
    next
}

fn midpoint(
    positions: &mut Vec<Vec3>,
    cache: &mut HashMap<(u32, u32), u32>,
    a: u32,
    b: u32,
) -> u32 {
    let key = if a < b { (a, b) } else { (b, a) };
    if let Some(&i) = cache.get(&key) {
        return i;
    }
    let p = ((positions[a as usize] + positions[b as usize]) * 0.5).normalize();
    let i = positions.len() as u32;
    positions.push(p);
    cache.insert(key, i);
    i
}
