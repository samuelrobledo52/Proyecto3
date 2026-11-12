use raylib::prelude::*;
use rayon::prelude::*;

const CLEAR_ROWS: usize = 16;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub color: Vec<[u8; 4]>,
    pub depth: Vec<f32>,
    texture: Texture2D,
}

impl Framebuffer {
    pub fn new(
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        width: usize,
        height: usize,
    ) -> Result<Self, String> {
        let image = Image::gen_image_color(width as i32, height as i32, Color::BLACK);
        let texture = rl
            .load_texture_from_image(thread, &image)
            .map_err(|e| e.to_string())?;
        texture.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_POINT);
        Ok(Framebuffer {
            width,
            height,
            color: vec![[0, 0, 0, 255]; width * height],
            depth: vec![0.0; width * height],
            texture,
        })
    }

    pub fn clear(&mut self, color: [u8; 4]) {
        let chunk = self.width * CLEAR_ROWS;
        self.color
            .par_chunks_mut(chunk)
            .zip(self.depth.par_chunks_mut(chunk))
            .for_each(|(c, d)| {
                c.fill(color);
                d.fill(0.0);
            });
    }

    pub fn upload(&mut self) {
        let _ = self.texture.update_texture(self.color.as_flattened());
    }

    pub fn draw(&self, d: &mut RaylibDrawHandle, window_width: f32, window_height: f32) {
        let source = Rectangle::new(0.0, 0.0, self.width as f32, self.height as f32);
        let dest = Rectangle::new(0.0, 0.0, window_width, window_height);
        d.draw_texture_pro(
            &self.texture,
            source,
            dest,
            Vector2::zero(),
            0.0,
            Color::WHITE,
        );
    }
}
