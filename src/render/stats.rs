#[derive(Default, Clone, Copy)]
pub struct Stats {
    pub objects: u32,
    pub culled: u32,
    pub culled_tris: u32,
    pub lod_saved: u32,
    pub submitted: u32,
    pub outside: u32,
    pub backface: u32,
    pub clipped: u32,
    pub rasterized: u32,
    pub inst_high: u32,
    pub inst_low: u32,
    pub inst_hidden: u32,
    pub inst_far: u32,
}

impl Stats {
    pub fn add_tris(&mut self, other: &Stats) {
        self.submitted += other.submitted;
        self.outside += other.outside;
        self.backface += other.backface;
        self.clipped += other.clipped;
        self.rasterized += other.rasterized;
    }
}

pub fn short_count(n: u32) -> String {
    if n >= 1000 {
        format!("{:.1}k", n as f32 / 1000.0)
    } else {
        n.to_string()
    }
}
