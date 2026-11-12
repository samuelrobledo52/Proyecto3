use nalgebra_glm as glm;

pub type Vec3 = glm::Vec3;
pub type Vec4 = glm::Vec4;
pub type Mat4 = glm::Mat4;

pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y * 0.5).tan();
    Mat4::new(
        f / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        f,
        0.0,
        0.0,
        0.0,
        0.0,
        (far + near) / (near - far),
        2.0 * far * near / (near - far),
        0.0,
        0.0,
        -1.0,
        0.0,
    )
}

pub fn look_dir(forward: Vec3, up: Vec3) -> Mat4 {
    let f = forward.normalize();
    let r = f.cross(&up).normalize();
    let u = r.cross(&f);
    Mat4::new(
        r.x, r.y, r.z, 0.0, u.x, u.y, u.z, 0.0, -f.x, -f.y, -f.z, 0.0, 0.0, 0.0, 0.0, 1.0,
    )
}

pub fn model_matrix(offset: Vec3, rotation: Vec3, scale: f32) -> Mat4 {
    let translate = glm::translation(&offset);
    let rotate = glm::rotation(rotation.y, &Vec3::y_axis())
        * glm::rotation(rotation.x, &Vec3::x_axis())
        * glm::rotation(rotation.z, &Vec3::z_axis());
    translate * rotate * glm::scaling(&Vec3::repeat(scale))
}

pub fn transform_point(m: &Mat4, p: &Vec3) -> Vec3 {
    (m * Vec4::new(p.x, p.y, p.z, 1.0)).xyz()
}

pub fn transform_dir(m: &Mat4, d: &Vec3) -> Vec3 {
    (m * Vec4::new(d.x, d.y, d.z, 0.0)).xyz()
}
