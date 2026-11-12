use crate::math::Vec3;

const DEFAULT_KD: [f32; 3] = [0.8, 0.8, 0.8];

pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub colors: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub slopes: Vec<f32>,
    pub bound: f32,
}

impl Mesh {
    pub fn new(positions: Vec<Vec3>, normals: Vec<Vec3>, indices: Vec<u32>) -> Self {
        let colors = vec![Vec3::repeat(1.0); positions.len()];
        Mesh::with_colors(positions, normals, colors, indices)
    }

    fn with_colors(
        positions: Vec<Vec3>,
        normals: Vec<Vec3>,
        colors: Vec<Vec3>,
        indices: Vec<u32>,
    ) -> Self {
        let slopes = positions
            .iter()
            .zip(&normals)
            .map(|(p, n)| slope(p, n))
            .collect();
        let bound = positions.iter().map(|p| p.norm()).fold(0.0, f32::max);
        Mesh {
            positions,
            normals,
            colors,
            indices,
            slopes,
            bound,
        }
    }

    pub fn tris(&self) -> u32 {
        (self.indices.len() / 3) as u32
    }
}

pub fn load_obj(path: &str) -> Result<Mesh, String> {
    let options = tobj::LoadOptions {
        single_index: true,
        triangulate: true,
        ..Default::default()
    };
    let (models, materials) = tobj::load_obj(path, &options).map_err(|e| format!("{path}: {e}"))?;
    let materials = materials.unwrap_or_default();

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for model in models {
        let mesh = model.mesh;
        let kd = mesh
            .material_id
            .and_then(|id| materials.get(id))
            .and_then(|m| m.diffuse)
            .unwrap_or(DEFAULT_KD);
        let color = Vec3::new(kd[0], kd[1], kd[2]);
        let base = positions.len() as u32;
        let count = mesh.positions.len() / 3;
        for i in 0..count {
            positions.push(Vec3::new(
                mesh.positions[i * 3],
                mesh.positions[i * 3 + 1],
                mesh.positions[i * 3 + 2],
            ));
            colors.push(color);
            if mesh.normals.len() == mesh.positions.len() {
                normals.push(Vec3::new(
                    mesh.normals[i * 3],
                    mesh.normals[i * 3 + 1],
                    mesh.normals[i * 3 + 2],
                ));
            }
        }
        indices.extend(mesh.indices.iter().map(|i| i + base));
    }

    if positions.is_empty() {
        return Err(format!("{path}: no tiene vértices"));
    }
    if normals.len() == positions.len() {
        fix_winding(&positions, &normals, &mut indices);
    } else {
        normals = smooth_normals(&positions, &indices);
    }

    Ok(Mesh::with_colors(positions, normals, colors, indices))
}

fn fix_winding(positions: &[Vec3], normals: &[Vec3], indices: &mut [u32]) {
    for tri in indices.chunks_exact_mut(3) {
        let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        let face = (positions[b] - positions[a]).cross(&(positions[c] - positions[a]));
        if face.dot(&(normals[a] + normals[b] + normals[c])) < 0.0 {
            tri.swap(1, 2);
        }
    }
}

fn slope(p: &Vec3, n: &Vec3) -> f32 {
    let len = p.norm() * n.norm();
    if len > 1e-8 {
        (1.0 - p.dot(n) / len).max(0.0)
    } else {
        0.0
    }
}

pub fn smooth_normals(positions: &[Vec3], indices: &[u32]) -> Vec<Vec3> {
    let mut normals = vec![Vec3::zeros(); positions.len()];
    for tri in indices.chunks_exact(3) {
        let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        let face = (positions[b] - positions[a]).cross(&(positions[c] - positions[a]));
        normals[a] += face;
        normals[b] += face;
        normals[c] += face;
    }
    for n in &mut normals {
        if n.norm() > 0.0 {
            *n = n.normalize();
        }
    }
    normals
}
