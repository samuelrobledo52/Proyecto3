mod bench;
mod camera;
mod config;
mod hud;
mod math;
mod render;
mod ship;
mod world;

use std::fmt::Write as _;

use raylib::prelude::*;

use bench::Bench;
use camera::Camera;
use config::Config;
use hud::{Hud, Readout, Target};
use math::{perspective, Mat4, Vec3, Vec4};
use render::framebuffer::Framebuffer;
use render::mesh::{load_obj, Mesh};
use render::pipeline::Pipeline;
use render::post::{sun_spot, Post};
use render::raster::Raster;
use render::shaders::{ring_bands, ShaderKind, Uniforms};
use ship::flight::{Flight, FlightParams, Input};
use ship::ship::Ship;
use world::noise::Noise;
use world::planet::Planet;
use world::scatter::{census, sight, Pick, View, MODELS};
use world::skybox::Skybox;
use world::system::System;

const WINDOW_WIDTH: i32 = 1280;
const WINDOW_HEIGHT: i32 = 720;
const RENDER_WIDTH: usize = 960;
const RENDER_HEIGHT: usize = 540;
const BACKGROUND: [u8; 4] = [12, 10, 30, 255];
const KIND_NAMES: [&str; 4] = ["árboles", "rocas", "torres", "cristales"];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str| args.iter().any(|arg| arg == name);
    let bench_mode = flag("--bench");

    unsafe {
        raylib::ffi::SetConfigFlags(raylib::consts::ConfigFlags::FLAG_WINDOW_HIGHDPI as u32);
    }
    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Proyecto 3 - Sistema solar")
        .build();
    rl.set_exit_key(Some(KeyboardKey::KEY_ESCAPE));

    let mut config = Config::load();
    let ship_mesh = match load_obj("assets/models/nave.obj") {
        Ok(mesh) => mesh,
        Err(error) => {
            eprintln!("No se pudo cargar la nave: {error}");
            return;
        }
    };
    let models = match read_models() {
        Ok(models) => models,
        Err(error) => {
            eprintln!("No se pudo cargar un modelo: {error}");
            return;
        }
    };
    let started = std::time::Instant::now();
    let mut system = System::new(&config);
    let mut skybox = Skybox::new(system.seed);
    println!(
        "Mundo generado en {:.0} ms",
        started.elapsed().as_secs_f32() * 1000.0
    );
    report(&system, &models);
    let mut noise = Noise::new(system.seed);
    let mut bands = ring_bands(&noise);
    let mut lod_px = lod_thresholds(&config);
    let mut inst_view = instance_view(&config);
    let mut bench = bench_mode.then(|| Bench::new(&system));

    let mut flight = Flight::new(FlightParams::from(&config));
    let mut ship = Ship::new(Vec3::zeros(), config.get("ship.scale", 0.3));
    let start = system
        .planets
        .iter()
        .position(|p| p.name == "Verdania")
        .unwrap_or(0);
    flight.jump(&mut ship, &system, start, 4.0);
    let mut camera = Camera::new(&config);
    camera.snap(&ship);

    let mut fb = match Framebuffer::new(&mut rl, &thread, RENDER_WIDTH, RENDER_HEIGHT) {
        Ok(fb) => fb,
        Err(error) => {
            eprintln!("No se pudo crear el framebuffer: {error}");
            return;
        }
    };
    let mut pipeline = Pipeline::new();
    pipeline.culling = !flag("--no-cull");
    inst_view.lod = !flag("--no-lod");
    let mut raster = Raster::new();
    let mut post = Post::new(RENDER_WIDTH, RENDER_HEIGHT);
    let mut hud = Hud::new(WINDOW_WIDTH, WINDOW_HEIGHT);
    let mut bump = true;

    while !rl.window_should_close() {
        let frame_time = rl.get_frame_time();
        let dt = frame_time.min(0.05);

        if rl.is_key_pressed(KeyboardKey::KEY_F5) {
            config = Config::load();
            flight.params = FlightParams::from(&config);
            camera.configure(&config);
            ship.scale = config.get("ship.scale", 0.3);
            lod_px = lod_thresholds(&config);
            let lod = inst_view.lod;
            inst_view = instance_view(&config);
            inst_view.lod = lod;
            let time = system.time;
            system = System::new(&config);
            system.update(time);
            skybox = Skybox::new(system.seed);
            noise = Noise::new(system.seed);
            bands = ring_bands(&noise);
            report(&system, &models);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_C) {
            pipeline.culling = !pipeline.culling;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_L) {
            inst_view.lod = !inst_view.lod;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_H) {
            hud.help = !hud.help;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_TAB) {
            hud.stats = !hud.stats;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_B) {
            bump = !bump;
        }
        inst_view.horizon = pipeline.culling;

        system.update(dt);
        match &mut bench {
            Some(bench) => {
                bench.time += dt;
                if bench.time > bench.seconds {
                    break;
                }
                bench::fly(&mut camera, &system, bench.time);
            }
            None => {
                let input = Input::read(&rl);
                flight.update(&mut ship, &input, &system, dt);
                camera.follow(&ship, &input, &system, dt);
            }
        }

        let aspect = RENDER_WIDTH as f32 / RENDER_HEIGHT as f32;
        let projection = perspective(camera.fov, aspect, 0.05, 6000.0);
        let view = camera.view();

        fb.clear(BACKGROUND);
        skybox.draw(
            &mut fb.color,
            RENDER_WIDTH,
            RENDER_HEIGHT,
            &(projection * view),
            system.time,
        );
        let uniforms = Uniforms {
            light: -camera.position,
            time: system.time,
            noise: &noise,
            ring: &bands,
        };
        pipeline.begin(&view, &projection, RENDER_WIDTH, RENDER_HEIGHT);
        let scene = Scene {
            system: &system,
            camera: &camera,
            lod_px: &lod_px,
            inst_view: &inst_view,
            models: &models,
            bump,
        };
        let lod = submit_bodies(&mut pipeline, &scene, &uniforms);
        pipeline.submit(
            &ship_mesh,
            &ship.model(camera.position),
            ShaderKind::Lit {
                color: Vec3::repeat(1.0),
            },
            &uniforms,
        );
        raster.draw(&mut fb, &pipeline.tris, &uniforms);
        let sun = sun_spot(
            &view,
            &projection,
            -camera.position,
            system.sun.radius,
            RENDER_WIDTH,
            RENDER_HEIGHT,
        );
        post.apply(&mut fb, sun, dt);
        fb.upload();

        let fps = rl.get_fps();
        let stats = pipeline.stats;
        if let Some(bench) = &mut bench {
            bench.record(frame_time, stats);
        }

        let readout = Readout {
            fps,
            speed: flight.speed(&ship),
            altitude: flight.altitude,
            mode: mode_text(&flight, &system),
            gravity: flight.gravity,
            prompt: prompt_text(&flight, &system),
            stats: &stats,
            culling: pipeline.culling,
            lod: inst_view.lod.then_some(lod),
            target: nearest(
                &system,
                if bench.is_some() {
                    camera.position
                } else {
                    ship.position
                },
                &(projection * view),
                camera.position,
            ),
        };

        let mut d = rl.begin_drawing(&thread);
        fb.draw(&mut d, WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32);
        hud.draw(&mut d, &readout);
        if let Some(bench) = &mut bench {
            bench.screenshot(&mut d, &thread);
        }
    }

    if let Some(bench) = &bench {
        bench.write();
    }
}

struct Scene<'a> {
    system: &'a System,
    camera: &'a Camera,
    lod_px: &'a [f32; 3],
    inst_view: &'a View,
    models: &'a [[Mesh; 2]],
    bump: bool,
}

fn submit_bodies(pipeline: &mut Pipeline, scene: &Scene, uniforms: &Uniforms) -> usize {
    let (system, camera, lod_px) = (scene.system, scene.camera, scene.lod_px);
    let pixel_scale = RENDER_HEIGHT as f32 / (2.0 * (camera.fov * 0.5).tan());
    let level = |lod: usize, count: usize| {
        if scene.inst_view.lod {
            lod
        } else {
            count.saturating_sub(1)
        }
    };
    let sun = &system.sun;
    let sun_lod = level(
        sun.lod_level(camera.position.norm(), pixel_scale, lod_px),
        sun.lods.len(),
    );
    pipeline.stats.lod_saved += lod_savings(&sun.lods, sun_lod);
    pipeline.submit(
        &sun.lods[sun_lod],
        &sun.model(camera.position),
        sun.shader,
        uniforms,
    );

    let mut nearest = (f32::MAX, 0);
    for planet in &system.planets {
        let distance = (planet.position - camera.position).norm();
        let lod = level(
            planet.lod_level(distance, pixel_scale, lod_px),
            planet.lods.len(),
        );
        if distance / planet.radius < nearest.0 {
            nearest = (distance / planet.radius, lod);
        }
        pipeline.stats.lod_saved += lod_savings(&planet.lods, lod);
        let model = planet.model(camera.position);
        let shader = match planet.shader {
            ShaderKind::Ruins { amp, craters, .. } if !scene.bump => ShaderKind::Ruins {
                amp,
                craters,
                bump: 0.0,
            },
            shader => shader,
        };
        pipeline.submit(&planet.lods[lod], &model, shader, uniforms);
        if let Some(ring) = &planet.ring {
            pipeline.submit(&ring.mesh, &model, ring.shader, uniforms);
        }
        let frame = planet.frame(camera.position);
        submit_instances(pipeline, planet, &frame, scene, uniforms);
    }
    nearest.1
}

fn submit_instances(
    pipeline: &mut Pipeline,
    planet: &Planet,
    frame: &Mat4,
    scene: &Scene,
    uniforms: &Uniforms,
) {
    let sight = sight(frame, planet.floor());
    let shader = ShaderKind::Lit {
        color: Vec3::repeat(1.0),
    };
    for inst in &planet.instances {
        let Some([high, low]) = scene.models.get(inst.kind) else {
            continue;
        };
        let mesh = match inst.pick(frame, &sight, high.bound, scene.inst_view) {
            Pick::High => {
                pipeline.stats.inst_high += 1;
                high
            }
            Pick::Low => {
                pipeline.stats.inst_low += 1;
                pipeline.stats.lod_saved += high.tris() - low.tris();
                low
            }
            Pick::Hidden { low: lod_low } => {
                pipeline.stats.inst_hidden += 1;
                let chosen = if lod_low { low } else { high };
                pipeline.stats.lod_saved += high.tris() - chosen.tris();
                pipeline.stats.culled_tris += chosen.tris();
                continue;
            }
            Pick::Far => {
                pipeline.stats.inst_far += 1;
                pipeline.stats.lod_saved += high.tris();
                continue;
            }
        };
        pipeline.submit(mesh, &(frame * inst.local), shader, uniforms);
    }
}

fn lod_savings(lods: &[Mesh], level: usize) -> u32 {
    let full = lods.last().map_or(0, Mesh::tris);
    full - lods.get(level).map_or(full, Mesh::tris)
}

fn read_models() -> Result<Vec<[Mesh; 2]>, String> {
    MODELS
        .iter()
        .map(|name| {
            Ok([
                load_obj(&format!("assets/models/{name}.obj"))?,
                load_obj(&format!("assets/models/{name}_lo.obj"))?,
            ])
        })
        .collect()
}

fn report(system: &System, models: &[[Mesh; 2]]) {
    for planet in &system.planets {
        let count = census(&planet.instances);
        if count.iter().all(|&c| c == 0) {
            continue;
        }
        let mut line = format!("{}:", planet.name);
        for (i, &c) in count.iter().enumerate().filter(|(_, &c)| c > 0) {
            let tris = models.get(i).map_or([0, 0], |m| [m[0].tris(), m[1].tris()]);
            let _ = write!(
                line,
                " {} {} ({}/{} tris)",
                KIND_NAMES[i], c, tris[0], tris[1]
            );
        }
        println!("{line}");
    }
}

fn instance_view(config: &Config) -> View {
    View {
        near: config.get("lod.instance_near", 16.0),
        far: config.get("lod.instance_far", 70.0),
        lod: true,
        horizon: true,
    }
}

fn lod_thresholds(config: &Config) -> [f32; 3] {
    [
        config.get("lod.px1", 30.0),
        config.get("lod.px2", 80.0),
        config.get("lod.px3", 180.0),
    ]
}

fn mode_text(flight: &Flight, system: &System) -> String {
    match flight.orbit_planet().and_then(|i| system.planets.get(i)) {
        Some(planet) => format!("Órbita ({})", planet.name),
        None => "Crucero".to_string(),
    }
}

fn prompt_text(flight: &Flight, system: &System) -> Option<String> {
    if let Some(text) = flight.notice() {
        return Some(text.to_string());
    }
    let planet = system.planets.get(flight.suggestion?)?;
    Some(format!("O  Orbitar {}", planet.name))
}

fn nearest<'a>(
    system: &'a System,
    ship: Vec3,
    view_projection: &Mat4,
    camera: Vec3,
) -> Option<Target<'a>> {
    let planet = system.planets.iter().min_by(|a, b| {
        let da = (a.position - ship).norm() - a.radius;
        let db = (b.position - ship).norm() - b.radius;
        da.total_cmp(&db)
    })?;
    let offset = planet.position - camera;
    let clip = view_projection * Vec4::new(offset.x, offset.y, offset.z, 1.0);
    let (w, h) = (WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32);
    let screen = (clip.w > 0.0)
        .then(|| {
            let x = (clip.x / clip.w * 0.5 + 0.5) * w;
            let y = (0.5 - clip.y / clip.w * 0.5) * h;
            let radius = planet.radius / clip.w * view_projection[(1, 1)] * h * 0.5;
            (x, y, radius)
        })
        .filter(|&(x, y, _)| (0.0..w).contains(&x) && (0.0..h).contains(&y));
    let edge = Vec3::new(clip.x * clip.w.signum(), -clip.y * clip.w.signum(), 0.0);
    let edge = if edge.norm() > 1e-6 {
        edge.normalize()
    } else {
        Vec3::y()
    };
    Some(Target {
        name: &planet.name,
        distance: ((planet.position - ship).norm() - planet.radius).max(0.0),
        screen,
        edge: (edge.x, edge.y),
    })
}
