use std::fmt::Write as _;
use std::fs;

use raylib::prelude::*;

use crate::camera::Camera;
use crate::math::Vec3;
use crate::render::stats::Stats;
use crate::world::system::System;

const INTRO: f32 = 2.5;
const VISIT: f32 = 3.5;
const PATH: &str = "docs/bench.csv";

struct Frame {
    time: f32,
    frame_time: f32,
    after_shot: bool,
    stats: Stats,
}

pub struct Bench {
    pub time: f32,
    pub seconds: f32,
    shots: Vec<(f32, String)>,
    shots_taken: usize,
    after_shot: bool,
    frames: Vec<Frame>,
}

impl Bench {
    pub fn new(system: &System) -> Self {
        let mut shots = vec![(INTRO - 0.5, "screenshot_bench_sistema.png".to_string())];
        for (i, planet) in system.planets.iter().enumerate() {
            shots.push((
                INTRO + VISIT * (i as f32 + 0.8),
                format!("screenshot_bench_{}.png", planet.name.to_lowercase()),
            ));
        }
        Bench {
            time: 0.0,
            seconds: INTRO + VISIT * system.planets.len() as f32,
            shots,
            shots_taken: 0,
            after_shot: false,
            frames: Vec::new(),
        }
    }

    pub fn record(&mut self, frame_time: f32, stats: Stats) {
        self.frames.push(Frame {
            time: self.time,
            frame_time,
            after_shot: self.after_shot,
            stats,
        });
        self.after_shot = false;
    }

    pub fn screenshot(&mut self, d: &mut RaylibDrawHandle, thread: &RaylibThread) {
        let shot = self
            .shots
            .get(self.shots_taken)
            .filter(|(at, _)| self.time > *at);
        if let Some((_, name)) = shot {
            d.take_screenshot(thread, name);
            self.shots_taken += 1;
            self.after_shot = true;
        }
    }

    pub fn write(&self) {
        let mut csv = String::from(
            "time,frame_ms,fps,after_shot,objects,culled,culled_tris,lod_saved,submitted,outside,backface,clipped,rasterized,inst_high,inst_low,inst_hidden,inst_far\n",
        );
        for f in &self.frames {
            let s = &f.stats;
            let _ = writeln!(
                csv,
                "{:.3},{:.3},{:.1},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                f.time,
                f.frame_time * 1000.0,
                1.0 / f.frame_time.max(1e-6),
                u8::from(f.after_shot),
                s.objects,
                s.culled,
                s.culled_tris,
                s.lod_saved,
                s.submitted,
                s.outside,
                s.backface,
                s.clipped,
                s.rasterized,
                s.inst_high,
                s.inst_low,
                s.inst_hidden,
                s.inst_far
            );
        }
        if let Err(error) = fs::write(PATH, csv) {
            eprintln!("No se pudo escribir {PATH}: {error}");
        }
        self.summary();
    }

    fn summary(&self) {
        let measured: Vec<&Frame> = self
            .frames
            .iter()
            .filter(|f| f.time > 0.5 && !f.after_shot)
            .collect();
        if measured.is_empty() {
            return;
        }
        let total: f32 = measured.iter().map(|f| f.frame_time).sum();
        let mut times: Vec<f32> = measured.iter().map(|f| f.frame_time).collect();
        times.sort_by(|a, b| b.total_cmp(a));
        let worst = times[0];
        let slow_count = (times.len() / 100).max(1);
        let slow_avg = times[..slow_count].iter().sum::<f32>() / slow_count as f32;
        let avg = |pick: fn(&Stats) -> u32| {
            measured.iter().map(|f| pick(&f.stats) as f32).sum::<f32>() / measured.len() as f32
        };

        println!("Frames: {}", measured.len());
        println!("FPS promedio: {:.1}", measured.len() as f32 / total);
        println!("FPS mínimo: {:.1}", 1.0 / worst);
        println!("FPS 1% bajo: {:.1}", 1.0 / slow_avg);
        println!(
            "Triángulos promedio -> ahorrados por culling {:.0}, ahorrados por LOD {:.0}, enviados {:.0}, fuera {:.0}, traseros {:.0}, recortados {:.0}, rasterizados {:.0}",
            avg(|s| s.culled_tris),
            avg(|s| s.lod_saved),
            avg(|s| s.submitted),
            avg(|s| s.outside),
            avg(|s| s.backface),
            avg(|s| s.clipped),
            avg(|s| s.rasterized)
        );
        println!(
            "Instancias promedio -> alto {:.0}, bajo {:.0}, horizonte {:.0}, lejanas {:.0}",
            avg(|s| s.inst_high),
            avg(|s| s.inst_low),
            avg(|s| s.inst_hidden),
            avg(|s| s.inst_far)
        );
    }
}

pub fn fly(camera: &mut Camera, system: &System, t: f32) {
    if t < INTRO {
        let angle = 0.6 + t * 0.08;
        camera.position = Vec3::new(330.0 * angle.sin(), 150.0, 330.0 * angle.cos());
        camera.look_at(Vec3::zeros());
        return;
    }
    let visit = (t - INTRO) / VISIT;
    let index = (visit as usize).min(system.planets.len().saturating_sub(1));
    let Some(planet) = system.planets.get(index) else {
        return;
    };
    let local = (visit - index as f32).clamp(0.0, 1.0);
    let approach = (local / 0.7).min(1.0);
    let eased = approach * approach * (3.0 - 2.0 * approach);
    let (near, tilt) = if planet.ring.is_some() {
        (3.6, 0.0)
    } else {
        (1.35, 0.8)
    };
    let factor = 6.0 + (near - 6.0) * eased;
    let out = if planet.position.norm() > 1e-3 {
        -planet.position.normalize()
    } else {
        Vec3::z()
    };
    let swing = -0.7 + 1.1 * local;
    let (s, c) = swing.sin_cos();
    let side = Vec3::new(out.x * c + out.z * s, 0.0, -out.x * s + out.z * c);
    let dir = (side + Vec3::y() * 0.4).normalize();
    camera.position = planet.position + dir * planet.radius * factor;
    let horizon = (Vec3::y() - dir * dir.y).normalize();
    camera.look_at(planet.position + horizon * planet.radius * tilt * eased);
}
