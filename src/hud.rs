use raylib::prelude::*;

use crate::render::stats::{short_count, Stats};

const TEXT: Color = Color::new(236, 238, 255, 255);
const MUTED: Color = Color::new(160, 168, 214, 255);
const ACCENT: Color = Color::new(255, 198, 116, 255);
const PANEL: Color = Color::new(14, 16, 40, 170);
const SIZE: i32 = 18;
const ROW: i32 = 22;
const PAD: i32 = 12;

const HELP: [(&str, &str); 24] = [
    ("W / S", "Acelerar / frenar"),
    ("A / D", "Girar"),
    ("Arriba / Abajo", "Cabeceo"),
    ("Q / E", "Alabeo izquierda / derecha"),
    ("X", "Air roll (A / D alabean)"),
    ("Shift", "Turbo"),
    ("Clic derecho", "Mirar alrededor"),
    ("Rueda", "Distancia de cámara"),
    ("O", "Entrar / salir de órbita"),
    ("R / F", "Subir / bajar en órbita"),
    ("G", "Gravedad"),
    ("1 - 5", "Saltar a un planeta"),
    ("C", "Culling"),
    ("L", "Niveles de detalle"),
    ("Tab", "Estadísticas"),
    ("F5", "Recargar config.txt"),
    ("B", "Normal mapping en Ruinas"),
    ("H", "Ayuda"),
    ("Esc", "Salir"),
    ("En órbita", "W/S velocidad  A/D rumbo"),
    ("", "R/F altura  O salir"),
    ("", "Cabeceo y alabeo solo inclinan"),
    ("Planetas", "1 Ignis  2 Verdania  3 Glacia"),
    ("", "4 Aether  5 Ruinas"),
];

pub struct Target<'a> {
    pub name: &'a str,
    pub distance: f32,
    pub screen: Option<(f32, f32, f32)>,
    pub edge: (f32, f32),
}

pub struct Readout<'a> {
    pub fps: u32,
    pub speed: f32,
    pub altitude: f32,
    pub mode: String,
    pub gravity: bool,
    pub air_roll: bool,
    pub prompt: Option<String>,
    pub stats: &'a Stats,
    pub culling: bool,
    pub lod: Option<usize>,
    pub target: Option<Target<'a>>,
}

pub struct Hud {
    pub help: bool,
    pub stats: bool,
    width: i32,
    height: i32,
}

impl Hud {
    pub fn new(width: i32, height: i32) -> Self {
        Hud {
            help: false,
            stats: true,
            width,
            height,
        }
    }

    pub fn draw(&self, d: &mut RaylibDrawHandle, r: &Readout) {
        if let Some(target) = &r.target {
            self.marker(d, target);
        }
        self.ship_panel(d, r);
        self.top_right(d, r);
        if self.stats {
            self.stats_panel(d, r);
        }
        if let Some(prompt) = &r.prompt {
            self.prompt(d, prompt);
        }
        let hint = "H  Ayuda";
        let w = d.measure_text(hint, 16);
        d.draw_text(hint, self.width - w - PAD - 4, self.height - 26, 16, MUTED);
        if self.help {
            self.help_panel(d);
        }
    }

    fn ship_panel(&self, d: &mut RaylibDrawHandle, r: &Readout) {
        let rows = [
            ("Velocidad", format!("{:.1}", r.speed)),
            ("Altitud", format!("{:.1}", r.altitude)),
            ("Modo", r.mode.clone()),
            ("Gravedad", yes_no(r.gravity).to_string()),
            ("Air roll", yes_no(r.air_roll).to_string()),
        ];
        panel(d, PAD, PAD, 250, rows.len() as i32 * ROW + PAD + 4);
        for (i, (label, value)) in rows.iter().enumerate() {
            let y = PAD + 8 + i as i32 * ROW;
            d.draw_text(label, PAD * 2, y, SIZE, MUTED);
            d.draw_text(value, PAD * 2 + 104, y, SIZE, TEXT);
        }
    }

    fn top_right(&self, d: &mut RaylibDrawHandle, r: &Readout) {
        let fps = format!("FPS {}", r.fps);
        let w = d.measure_text(&fps, SIZE);
        d.draw_text(&fps, self.width - w - PAD * 2, PAD + 8, SIZE, TEXT);
        if let Some(target) = &r.target {
            let line = format!("{}  {:.0} u", target.name, target.distance);
            let w = d.measure_text(&line, SIZE);
            d.draw_text(&line, self.width - w - PAD * 2, PAD + 8 + ROW, SIZE, ACCENT);
            let label = "Planeta más cercano";
            let w = d.measure_text(label, 14);
            d.draw_text(
                label,
                self.width - w - PAD * 2,
                PAD + 10 + ROW * 2,
                14,
                MUTED,
            );
        }
    }

    fn stats_panel(&self, d: &mut RaylibDrawHandle, r: &Readout) {
        let s = r.stats;
        let lod = match r.lod {
            Some(level) => format!("Sí (nivel {level})"),
            None => "No".to_string(),
        };
        let rows = [
            ("Culling", yes_no(r.culling).to_string()),
            ("LOD", lod),
            (
                "Objetos",
                format!("{} (descartados {})", s.objects, s.culled),
            ),
            (
                "Instancias",
                format!(
                    "{} alto / {} bajo / {} omitidas",
                    s.inst_high,
                    s.inst_low,
                    s.inst_hidden + s.inst_far
                ),
            ),
            ("Ahorro culling", short_count(s.culled_tris)),
            ("Ahorro LOD", short_count(s.lod_saved)),
            ("Enviados", short_count(s.submitted)),
            ("Fuera de vista", short_count(s.outside)),
            ("Traseros", short_count(s.backface)),
            ("Recortados", short_count(s.clipped)),
            ("Rasterizados", short_count(s.rasterized)),
        ];
        let h = rows.len() as i32 * ROW + PAD + 26;
        let y0 = self.height - h - 40;
        panel(d, PAD, y0, 380, h);
        d.draw_text("Triángulos por etapa", PAD * 2, y0 + 8, 16, ACCENT);
        for (i, (label, value)) in rows.iter().enumerate() {
            let y = y0 + 32 + i as i32 * ROW;
            d.draw_text(label, PAD * 2, y, 16, MUTED);
            d.draw_text(value, PAD * 2 + 136, y, 16, TEXT);
        }
    }

    fn prompt(&self, d: &mut RaylibDrawHandle, text: &str) {
        let w = d.measure_text(text, 20) + 32;
        let x = (self.width - w) / 2;
        let y = self.height - 84;
        d.draw_rectangle_rounded(
            Rectangle::new(x as f32, y as f32, w as f32, 36.0),
            0.5,
            8,
            PANEL,
        );
        d.draw_text(text, x + 16, y + 8, 20, ACCENT);
    }

    fn marker(&self, d: &mut RaylibDrawHandle, target: &Target) {
        match target.screen {
            Some((x, y, radius)) => {
                let r = radius.clamp(14.0, 260.0) + 8.0;
                let arm = (r * 0.35).min(22.0);
                for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                    let cx = x + sx * r;
                    let cy = y + sy * r;
                    d.draw_line_ex(
                        Vector2::new(cx, cy),
                        Vector2::new(cx - sx * arm, cy),
                        2.0,
                        ACCENT,
                    );
                    d.draw_line_ex(
                        Vector2::new(cx, cy),
                        Vector2::new(cx, cy - sy * arm),
                        2.0,
                        ACCENT,
                    );
                }
                if r < 200.0 {
                    let label = format!("{}  {:.0} u", target.name, target.distance);
                    let w = d.measure_text(&label, 16);
                    d.draw_text(
                        &label,
                        (x - w as f32 / 2.0) as i32,
                        (y + r + 6.0) as i32,
                        16,
                        ACCENT,
                    );
                }
            }
            None => {
                let (dx, dy) = target.edge;
                let cx = self.width as f32 / 2.0;
                let cy = self.height as f32 / 2.0;
                let reach = (cx - 40.0) / dx.abs().max(1e-3);
                let reach = reach.min((cy - 40.0) / dy.abs().max(1e-3));
                let tip = Vector2::new(cx + dx * reach, cy + dy * reach);
                let back = Vector2::new(tip.x - dx * 18.0, tip.y - dy * 18.0);
                let side = Vector2::new(-dy * 9.0, dx * 9.0);
                d.draw_triangle(
                    tip,
                    Vector2::new(back.x - side.x, back.y - side.y),
                    Vector2::new(back.x + side.x, back.y + side.y),
                    ACCENT,
                );
            }
        }
    }

    fn help_panel(&self, d: &mut RaylibDrawHandle) {
        let w = 560;
        let h = HELP.len() as i32 * ROW + 70;
        let x = (self.width - w) / 2;
        let y = (self.height - h) / 2;
        d.draw_rectangle(0, 0, self.width, self.height, Color::new(6, 8, 20, 215));
        panel(d, x, y, w, h);
        d.draw_text("Controles", x + 24, y + 16, 24, ACCENT);
        for (i, (key, action)) in HELP.iter().enumerate() {
            let row = y + 56 + i as i32 * ROW;
            d.draw_text(key, x + 24, row, SIZE, TEXT);
            d.draw_text(action, x + 190, row, SIZE, MUTED);
        }
    }
}

fn panel(d: &mut RaylibDrawHandle, x: i32, y: i32, w: i32, h: i32) {
    d.draw_rectangle_rounded(
        Rectangle::new(x as f32, y as f32, w as f32, h as f32),
        0.12,
        6,
        PANEL,
    );
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "Sí"
    } else {
        "No"
    }
}
