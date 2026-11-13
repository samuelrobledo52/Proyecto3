use nalgebra_glm as glm;

use super::*;
use crate::camera::Camera;
use crate::math::transform_point;
use crate::render::mesh::load_obj;

struct World {
    system: System,
    flight: Flight,
    ship: Ship,
    config: Config,
}

fn world() -> World {
    let config = Config::load();
    World {
        system: System::new(&config),
        flight: Flight::new(FlightParams::from(&config)),
        ship: Ship::new(Vec3::zeros(), config.get("ship.scale", 0.3)),
        config,
    }
}

impl World {
    fn step(&mut self, input: &Input, dt: f32) {
        self.system.update(dt);
        self.flight.update(&mut self.ship, input, &self.system, dt);
    }

    fn run(&mut self, input: &Input, seconds: f32, dt: f32) {
        for _ in 0..(seconds / dt).round() as usize {
            self.step(input, dt);
        }
    }

    fn enter(&mut self, i: usize, dt: f32) {
        self.flight.jump(&mut self.ship, &self.system, i, 3.5);
        self.step(&Input::default(), dt);
        let press = Input {
            orbit: true,
            ..Input::default()
        };
        self.step(&press, dt);
        assert_eq!(self.flight.orbit_planet(), Some(i));
    }
}

fn height(planet: &Planet, pos: Vec3) -> f32 {
    let inv = glm::inverse(&planet.frame(Vec3::zeros()));
    let local = transform_point(&inv, &pos);
    local.norm() - planet.surface_height(local)
}

fn clearance(system: &System, pos: Vec3) -> f32 {
    let sun = pos.norm() - system.sun.radius;
    system
        .planets
        .iter()
        .map(|p| height(p, pos))
        .fold(sun, f32::min)
}

fn throttle(t: f32) -> Input {
    Input {
        throttle: t,
        ..Input::default()
    }
}

#[test]
fn ground_matches_frame() {
    let mut w = world();
    w.system.update(37.0);
    for planet in &mut w.system.planets {
        planet.tilt += 0.4;
        let mut worst: f32 = 0.0;
        for k in 0..500 {
            let a = k as f32 * 2.399;
            let z = 1.0 - 2.0 * (k as f32 + 0.5) / 500.0;
            let r = (1.0 - z * z).sqrt();
            let dir = Vec3::new(r * a.cos(), z, r * a.sin());
            let pos = planet.position + dir * planet.radius * 1.5;
            let by_ground = (pos - planet.position).norm() - ground(planet, dir);
            worst = worst.max((by_ground - height(planet, pos)).abs());
        }
        println!("ground {}: error máx {worst:.2e}", planet.name);
        assert!(worst < 1e-3);
    }
}

#[test]
fn collision_holds() {
    let dt = 1.0 / 60.0;
    let n = world().system.planets.len();
    for i in 0..=n {
        let mut w = world();
        let margin = w.flight.params.margin;
        if i < n {
            w.flight.jump(&mut w.ship, &w.system, i, 3.5);
        } else {
            w.ship.position = Vec3::new(10.0, 45.0, 5.0);
            w.ship.look_at(Vec3::zeros());
        }
        let name = w
            .system
            .planets
            .get(i)
            .map_or("Sol", |p| p.name.as_str())
            .to_string();
        let mut low = f32::MAX;
        for _ in 0..900 {
            w.step(&throttle(1.0), dt);
            assert!(w.ship.position.iter().all(|v| v.is_finite()));
            low = low.min(clearance(&w.system, w.ship.position));
        }
        println!("colisión {name}: altura mínima {low:.3} (margen {margin})");
        assert!(low >= margin * 0.9);
    }
}

fn route(dt: f32) -> (Vec3, f32) {
    let mut w = world();
    w.ship.position = Vec3::new(0.0, 220.0, 0.0);
    w.ship.look_at(Vec3::new(100.0, 220.0, 0.0));
    let start = w.ship.position;
    let mut path = 0.0;
    let turn = Input {
        throttle: 1.0,
        yaw: 1.0,
        ..Input::default()
    };
    for (input, seconds) in [(throttle(1.0), 3.0), (turn, 2.0), (throttle(0.0), 3.0)] {
        for _ in 0..(seconds / dt).round() as usize {
            let before = w.ship.position;
            w.step(&input, dt);
            path += (w.ship.position - before).norm();
        }
    }
    (w.ship.position - start, path)
}

#[test]
fn same_path_any_dt() {
    let (base, path) = route(1.0 / 144.0);
    for dt in [1.0 / 30.0, 1.0 / 60.0] {
        let (end, _) = route(dt);
        let error = (end - base).norm() / path;
        println!(
            "dt 1/{:.0}: diferencia {:.3}% de {path:.1}",
            1.0 / dt,
            error * 100.0
        );
        assert!(error < 0.02);
    }
}

fn bend(gravity: bool) -> (f32, f32) {
    let mut w = world();
    w.flight.gravity = gravity;
    let planet = &w.system.planets[1];
    w.ship.position = planet.position + Vec3::y() * planet.radius * 2.0;
    w.ship.velocity = Vec3::x() * 6.0;
    let v0 = w.ship.velocity;
    w.run(&Input::default(), 3.0, 1.0 / 60.0);
    let v1 = w.ship.velocity;
    let toward = (w.system.planets[1].position - w.ship.position).normalize();
    let angle = v0.normalize().dot(&v1.normalize()).clamp(-1.0, 1.0).acos();
    let side = toward - v0.normalize() * toward.dot(&v0.normalize());
    (angle.to_degrees(), v1.dot(&side.normalize()))
}

#[test]
fn gravity_bends() {
    let (on, pull) = bend(true);
    let (off, still) = bend(false);
    println!("gravedad sí: giro {on:.2}°, v lateral hacia el planeta {pull:.2}");
    println!("gravedad no: giro {off:.4}°, v lateral hacia el planeta {still:.4}");
    assert!(on > 5.0 && pull > 0.5);
    assert!(off < 0.01 && still.abs() < 0.01);
}

#[test]
fn orbit_holds() {
    let dt = 1.0 / 60.0;
    for i in 0..world().system.planets.len() {
        let mut w = world();
        w.enter(i, dt);
        let p = &w.flight.params;
        let (margin, goal, base) = (p.margin, p.orbit_altitude, p.orbit_min_altitude);
        let top = p.orbit_max_altitude * w.system.planets[i].radius;
        let mut low = f32::MAX;
        let mut drift: f32 = 0.0;
        let mut far: f32 = 0.0;
        for k in 0..1200 {
            w.step(&throttle(1.0), dt);
            let planet = &w.system.planets[i];
            let h = height(planet, w.ship.position);
            assert!(h.is_finite());
            low = low.min(h);
            far = far.max((w.ship.position - planet.position).norm());
            if k >= 480 {
                drift = drift.max((h - goal).abs() / goal);
            }
        }
        let name = w.system.planets[i].name.clone();
        let climb = Input {
            throttle: 1.0,
            lift: 1.0,
            ..Input::default()
        };
        w.run(&climb, 4.0, dt);
        let high = height(&w.system.planets[i], w.ship.position);
        w.run(&climb, 40.0, dt);
        let ceiling = height(&w.system.planets[i], w.ship.position);
        let descend = Input {
            lift: -1.0,
            ..climb
        };
        w.run(&descend, 60.0, dt);
        let floor = height(&w.system.planets[i], w.ship.position);
        let before = w.ship.velocity;
        w.step(&throttle(1.0), dt);
        let last = w.ship.velocity;
        let exit = Input {
            orbit: true,
            throttle: 1.0,
            ..Input::default()
        };
        w.step(&exit, dt);
        let jump = (w.ship.velocity - last).norm();
        let usual = (last - before).norm();
        let speed = last.norm();
        println!(
            "órbita {name}: altura mín {low:.2}, desvío {:.1}% de {goal}, radio máx {far:.1}",
            drift * 100.0
        );
        println!(
            "órbita {name}: R 4 s {high:.2}, techo {ceiling:.2}/{top:.1}, piso {floor:.2}/{base}"
        );
        println!(
            "órbita {name}: salto de v al salir {jump:.3} (paso normal {usual:.3}, |v| {speed:.1})"
        );
        assert!(w.flight.orbit_planet().is_none());
        assert!(low >= margin * 0.9);
        assert!(drift < 0.1);
        assert!(high > goal + 5.0);
        assert!(ceiling < top * 1.1 + 1.0 && ceiling > top * 0.85);
        assert!((floor - base).abs() < base * 0.15);
        assert!(jump < 1.0);
    }
}

#[test]
fn jump_faces_planet() {
    let mut w = world();
    for i in 0..w.system.planets.len() {
        w.flight.jump(&mut w.ship, &w.system, i, 3.5);
        let planet = &w.system.planets[i];
        let to = (planet.position - w.ship.position).normalize();
        let h = height(planet, w.ship.position);
        println!(
            "salto {}: altura {h:.1}, alineación {:.4}",
            planet.name,
            w.ship.forward.dot(&to)
        );
        assert!(h > planet.radius && w.ship.forward.dot(&to) > 0.999);
        w.step(&Input::default(), 1.0 / 60.0);
        assert_eq!(w.flight.suggestion, Some(i));
    }
    w.flight.jump(&mut w.ship, &w.system, 1, 5.0);
    w.step(&Input::default(), 1.0 / 60.0);
    assert_eq!(w.flight.suggestion, None);
}

#[test]
fn camera_stays_out() {
    let dt = 1.0 / 60.0;
    let bound = load_obj("assets/models/nave.obj").map_or(4.0, |m| m.bound);
    for i in 0..world().system.planets.len() {
        let mut w = world();
        let mut camera = Camera::new(&w.config);
        w.enter(i, dt);
        camera.snap(&w.ship);
        let reach = bound * w.ship.scale;
        let mut low = f32::MAX;
        let mut near = f32::MAX;
        for k in 0..2400 {
            let t = k as f32 * dt;
            let input = Input {
                throttle: 1.0,
                lift: -1.0,
                yaw: (t * 0.7).sin(),
                look: true,
                look_x: (t * 1.3).sin() * 30.0,
                look_y: (t * 0.9).cos() * 40.0,
                zoom: if k < 120 { 2.0 } else { 0.0 },
                ..Input::default()
            };
            w.step(&input, dt);
            camera.follow(&w.ship, &input, &w.system, dt);
            low = low.min(clearance(&w.system, camera.position));
            near = near.min((camera.position - w.ship.position).norm() - reach);
        }
        println!(
            "cámara {}: altura mín {low:.3}, holgura a la nave {near:.3}",
            w.system.planets[i].name
        );
        assert!(low > 0.25 && near > 0.05);
    }
}

#[test]
fn spin_loops_without_drift() {
    let dt = 1.0 / 60.0;
    let mut w = world();
    w.flight.gravity = false;
    w.ship.position = Vec3::new(0.0, 260.0, 0.0);
    w.ship.look_at(Vec3::new(100.0, 260.0, 0.0));
    let tumble = Input {
        pitch: 1.0,
        yaw: 1.0,
        roll: -1.0,
        ..Input::default()
    };
    let top = w.flight.params.spin_max;
    let mut turned = 0.0;
    let mut last = w.ship.forward;
    for _ in 0..1200 {
        w.step(&tumble, dt);
        turned += last.dot(&w.ship.forward).clamp(-1.0, 1.0).acos();
        last = w.ship.forward;
        let s = &w.ship;
        assert!((0..3).all(|i| s.spin[i].abs() <= top[i] + 1e-4));
    }
    let s = &w.ship;
    let skew = (s.forward.norm() - 1.0)
        .abs()
        .max((s.up.norm() - 1.0).abs())
        .max(s.forward.dot(&s.up).abs());
    println!(
        "giro libre: {:.1} vueltas de la punta, error de base {skew:.2e}",
        turned / std::f32::consts::TAU
    );
    assert!(skew < 1e-4 && turned > 2.0 * std::f32::consts::TAU);
    w.run(&Input::default(), 1.5, dt);
    println!("giro libre: ω tras 1.5 s suelto {:.4}", w.ship.spin.norm());
    assert!(w.ship.spin.norm() < 0.01);
}

#[test]
fn orbit_lean_keeps_path() {
    let dt = 1.0 / 60.0;
    let mut calm = world();
    let mut lean = world();
    calm.enter(1, dt);
    lean.enter(1, dt);
    let tilt = Input {
        throttle: 1.0,
        pitch: 1.0,
        roll: 1.0,
        ..Input::default()
    };
    for _ in 0..180 {
        calm.step(&throttle(1.0), dt);
        lean.step(&tilt, dt);
    }
    let gap = (calm.ship.position - lean.ship.position).norm();
    let tilted = lean.ship.nose().dot(&lean.ship.forward);
    lean.run(&throttle(1.0), 2.0, dt);
    let level = lean.ship.nose().dot(&lean.ship.forward);
    println!("órbita inclinada: desvío {gap:.2e}, punta {tilted:.3} -> {level:.4}");
    assert!(gap < 1e-4 && tilted < 0.99 && level > 0.999);
}

#[test]
fn air_roll_toggle() {
    let dt = 1.0 / 60.0;
    let mut w = world();
    w.flight.gravity = false;
    w.ship.position = Vec3::new(0.0, 260.0, 0.0);
    w.ship.look_at(Vec3::new(100.0, 260.0, 0.0));
    let press = Input {
        roll_mode: true,
        ..Input::default()
    };
    let left = Input {
        yaw: 1.0,
        ..Input::default()
    };
    w.step(&press, dt);
    assert!(w.flight.air_roll);
    w.run(&left, 0.5, dt);
    let rolled = w.ship.spin;
    w.run(&Input::default(), 2.0, dt);
    w.step(&press, dt);
    assert!(!w.flight.air_roll);
    w.run(&left, 0.5, dt);
    let turned = w.ship.spin;
    println!("air roll: con X ω {rolled:?}, sin X ω {turned:?}");
    assert!(rolled.y.abs() < 1e-6 && rolled.z < -1.0);
    assert!(turned.y > 1.0 && turned.z.abs() < 0.01);
}
