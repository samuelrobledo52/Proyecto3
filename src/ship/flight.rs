use nalgebra_glm as glm;
use raylib::prelude::*;

use crate::config::Config;
use crate::math::Vec3;
use crate::ship::ship::Ship;
use crate::world::planet::Planet;
use crate::world::system::System;

const JUMP_KEYS: [KeyboardKey; 5] = [
    KeyboardKey::KEY_ONE,
    KeyboardKey::KEY_TWO,
    KeyboardKey::KEY_THREE,
    KeyboardKey::KEY_FOUR,
    KeyboardKey::KEY_FIVE,
];
const DEADZONE: f32 = 0.15;
const FLOOR_RATE: f32 = 25.0;

pub struct FlightParams {
    accel: f32,
    brake: f32,
    reverse: f32,
    max_speed: f32,
    turbo: f32,
    turn_rate: f32,
    turn_response: f32,
    drag: f32,
    grip: f32,
    approach: f32,
    min_speed: f32,
    margin: f32,
    nose: f32,
    gravity: f32,
    sun_mass: f32,
    orbit_range: f32,
    suggest_range: f32,
    orbit_accel: f32,
    orbit_max_speed: f32,
    orbit_climb: f32,
    orbit_altitude: f32,
    orbit_min_altitude: f32,
    orbit_max_altitude: f32,
}

impl From<&Config> for FlightParams {
    fn from(c: &Config) -> Self {
        FlightParams {
            accel: c.get("ship.accel", 14.0),
            brake: c.get("ship.brake", 22.0),
            reverse: c.get("ship.reverse", 0.25),
            max_speed: c.get("ship.max_speed", 45.0),
            turbo: c.get("ship.turbo", 2.5),
            turn_rate: c.get("ship.turn_rate", 1.4),
            turn_response: c.get("ship.turn_response", 4.0),
            drag: c.get("ship.drag", 0.25),
            grip: c.get("ship.grip", 1.2),
            approach: c.get("ship.approach", 1.5),
            min_speed: c.get("ship.min_speed", 8.0),
            margin: c.get("ship.margin", 0.6),
            nose: c.get("ship.nose", 1.0),
            gravity: c.get("gravity", 0.5),
            sun_mass: c.get("sun.mass", 6000.0),
            orbit_range: c.get("orbit.range", 4.0),
            suggest_range: c.get("orbit.suggest_range", 4.0),
            orbit_accel: c.get("orbit.accel", 6.0),
            orbit_max_speed: c.get("orbit.max_speed", 18.0),
            orbit_climb: c.get("orbit.climb", 5.0),
            orbit_altitude: c.get("orbit.altitude", 3.0),
            orbit_min_altitude: c.get("orbit.min_altitude", 2.5),
            orbit_max_altitude: c.get("orbit.max_altitude", 3.0),
        }
    }
}

#[derive(Default)]
pub struct Input {
    pub throttle: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub lift: f32,
    pub turbo: bool,
    pub orbit: bool,
    pub gravity: bool,
    pub jump: Option<usize>,
    pub look: bool,
    pub look_x: f32,
    pub look_y: f32,
    pub zoom: f32,
}

impl Input {
    pub fn read(rl: &RaylibHandle) -> Input {
        let key = |k: KeyboardKey| if rl.is_key_down(k) { 1.0 } else { 0.0 };
        let mut input = Input {
            throttle: key(KeyboardKey::KEY_W) - key(KeyboardKey::KEY_S),
            pitch: key(KeyboardKey::KEY_UP) - key(KeyboardKey::KEY_DOWN),
            yaw: key(KeyboardKey::KEY_A) - key(KeyboardKey::KEY_D),
            roll: key(KeyboardKey::KEY_RIGHT) - key(KeyboardKey::KEY_LEFT),
            lift: key(KeyboardKey::KEY_R) - key(KeyboardKey::KEY_F),
            turbo: rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
                || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT),
            orbit: rl.is_key_pressed(KeyboardKey::KEY_O),
            gravity: rl.is_key_pressed(KeyboardKey::KEY_G),
            jump: JUMP_KEYS.iter().position(|&k| rl.is_key_pressed(k)),
            look: rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT),
            zoom: rl.get_mouse_wheel_move(),
            ..Input::default()
        };
        if input.look {
            let delta = rl.get_mouse_delta();
            input.look_x = delta.x;
            input.look_y = delta.y;
        }
        if rl.is_gamepad_available(0) {
            gamepad(rl, &mut input);
        }
        input
    }
}

fn gamepad(rl: &RaylibHandle, input: &mut Input) {
    let raw = |a: GamepadAxis| rl.get_gamepad_axis_movement(0, a);
    let axis = |a: GamepadAxis| {
        let v = raw(a);
        if v.abs() < DEADZONE {
            0.0
        } else {
            v
        }
    };
    let down = |b: GamepadButton| rl.is_gamepad_button_down(0, b);

    input.yaw = (input.yaw - axis(GamepadAxis::GAMEPAD_AXIS_LEFT_X)).clamp(-1.0, 1.0);
    input.pitch = (input.pitch - axis(GamepadAxis::GAMEPAD_AXIS_LEFT_Y)).clamp(-1.0, 1.0);
    let gas = (raw(GamepadAxis::GAMEPAD_AXIS_RIGHT_TRIGGER) + 1.0) * 0.5;
    let stop = (raw(GamepadAxis::GAMEPAD_AXIS_LEFT_TRIGGER) + 1.0) * 0.5;
    input.throttle = (input.throttle + gas - stop).clamp(-1.0, 1.0);
    if down(GamepadButton::GAMEPAD_BUTTON_RIGHT_TRIGGER_1) {
        input.roll = 1.0;
    }
    if down(GamepadButton::GAMEPAD_BUTTON_LEFT_TRIGGER_1) {
        input.roll = -1.0;
    }
    if down(GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
        input.lift = 1.0;
    }
    if down(GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
        input.lift = -1.0;
    }
    input.turbo |= down(GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT);
    input.orbit |= rl.is_gamepad_button_pressed(0, GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN);

    let look_x = axis(GamepadAxis::GAMEPAD_AXIS_RIGHT_X);
    let look_y = axis(GamepadAxis::GAMEPAD_AXIS_RIGHT_Y);
    if look_x != 0.0 || look_y != 0.0 {
        input.look = true;
        input.look_x += look_x * 8.0;
        input.look_y += look_y * 8.0;
    }
}

struct OrbitState {
    planet: usize,
    altitude: f32,
    speed: f32,
    dir: Vec3,
    heading: Vec3,
    radius: f32,
    floor: f32,
    height: f32,
}

enum Mode {
    Cruise,
    Orbit(OrbitState),
}

pub struct Flight {
    pub params: FlightParams,
    mode: Mode,
    pub gravity: bool,
    pub altitude: f32,
    pub suggestion: Option<usize>,
    notice: String,
    notice_time: f32,
    last: Vec<Vec3>,
    drift: Vec<Vec3>,
}

impl Flight {
    pub fn new(params: FlightParams) -> Self {
        Flight {
            params,
            mode: Mode::Cruise,
            gravity: true,
            altitude: 0.0,
            suggestion: None,
            notice: String::new(),
            notice_time: 0.0,
            last: Vec::new(),
            drift: Vec::new(),
        }
    }

    pub fn update(&mut self, ship: &mut Ship, input: &Input, system: &System, dt: f32) {
        self.notice_time = (self.notice_time - dt).max(0.0);
        self.track(system, dt);
        if input.gravity {
            self.gravity = !self.gravity;
        }
        if let Some(i) = input.jump {
            self.jump(ship, system, i, 3.5);
        }
        if input.orbit {
            self.toggle_orbit(ship, system);
        }

        let p = &self.params;
        let target = match self.mode {
            Mode::Orbit(_) => Vec3::new(0.0, input.yaw * p.turn_rate, 0.0),
            Mode::Cruise => Vec3::new(input.pitch, input.yaw, input.roll * 1.4) * p.turn_rate,
        };
        let before = ship.turn;
        ship.turn += (target - ship.turn) * (1.0 - (-p.turn_response * dt).exp());
        let spin = if p.turn_response > 1e-4 {
            target * dt - (ship.turn - before) / p.turn_response
        } else {
            ship.turn * dt
        };

        let lost = match &mut self.mode {
            Mode::Cruise => {
                cruise(ship, input, system, p, self.gravity, spin, dt);
                false
            }
            Mode::Orbit(state) => match system.planets.get(state.planet) {
                Some(planet) => {
                    orbit(state, ship, input, planet, p, spin.y, dt);
                    false
                }
                None => true,
            },
        };
        if lost {
            self.mode = Mode::Cruise;
        }
        collide(ship, system, &self.params);

        self.altitude = nearest_altitude(ship.position, system);
        let range = self.params.suggest_range.min(self.params.orbit_range);
        self.suggestion = match self.mode {
            Mode::Cruise => closest(ship.position, system)
                .filter(|&(_, ratio)| ratio < range)
                .map(|(i, _)| i),
            Mode::Orbit(_) => None,
        };
    }

    fn track(&mut self, system: &System, dt: f32) {
        let n = system.planets.len();
        if self.last.len() != n || dt <= 0.0 {
            self.last = system.planets.iter().map(|p| p.position).collect();
            self.drift = vec![Vec3::zeros(); n];
            return;
        }
        for (i, planet) in system.planets.iter().enumerate() {
            self.drift[i] = (planet.position - self.last[i]) / dt;
            self.last[i] = planet.position;
        }
    }

    pub fn jump(&mut self, ship: &mut Ship, system: &System, index: usize, factor: f32) {
        let Some(planet) = system.planets.get(index) else {
            return;
        };
        let out = if planet.position.norm() > 1e-3 {
            -planet.position.normalize()
        } else {
            Vec3::z()
        };
        let dir = (out + Vec3::y() * 0.35).normalize();
        ship.position = planet.position + dir * planet.radius * factor;
        ship.velocity = Vec3::zeros();
        ship.turn = Vec3::zeros();
        ship.look_at(planet.position);
        self.mode = Mode::Cruise;
    }

    pub fn speed(&self, ship: &Ship) -> f32 {
        match &self.mode {
            Mode::Orbit(state) => state.speed.abs(),
            Mode::Cruise => ship.velocity.norm(),
        }
    }

    pub fn orbit_planet(&self) -> Option<usize> {
        match &self.mode {
            Mode::Orbit(state) => Some(state.planet),
            Mode::Cruise => None,
        }
    }

    pub fn notice(&self) -> Option<&str> {
        (self.notice_time > 0.0).then_some(self.notice.as_str())
    }

    fn toggle_orbit(&mut self, ship: &Ship, system: &System) {
        if let Mode::Orbit(_) = self.mode {
            self.mode = Mode::Cruise;
            return;
        }
        match closest(ship.position, system) {
            Some((i, ratio)) if ratio < self.params.orbit_range => {
                let drift = self.drift.get(i).copied().unwrap_or_else(Vec3::zeros);
                self.mode = Mode::Orbit(enter_orbit(
                    ship,
                    &system.planets[i],
                    i,
                    drift,
                    &self.params,
                ));
            }
            _ => {
                self.notice = "Ningún planeta cerca para orbitar".to_string();
                self.notice_time = 2.5;
            }
        }
    }
}

pub fn ground(planet: &Planet, dir: Vec3) -> f32 {
    let untilted = glm::rotate_x_vec3(&dir, -planet.tilt);
    planet.surface_height(glm::rotate_y_vec3(&untilted, -planet.spin))
}

fn closest(pos: Vec3, system: &System) -> Option<(usize, f32)> {
    system
        .planets
        .iter()
        .enumerate()
        .map(|(i, p)| (i, (pos - p.position).norm() / p.radius))
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

fn nearest_altitude(pos: Vec3, system: &System) -> f32 {
    let mut best = pos.norm() - system.sun.radius;
    for planet in &system.planets {
        let d = pos - planet.position;
        let dist = d.norm();
        if dist > 1e-4 {
            best = best.min(dist - ground(planet, d / dist));
        }
    }
    best.max(0.0)
}

fn gravity_at(pos: Vec3, system: &System, p: &FlightParams) -> Vec3 {
    let mut accel = Vec3::zeros();
    let mut pull = |center: Vec3, radius: f32, mass: f32| {
        let d = center - pos;
        let dist = d.norm();
        if dist > 1e-3 {
            let r = dist.max(radius);
            accel += d / dist * (p.gravity * mass / (r * r));
        }
    };
    pull(Vec3::zeros(), system.sun.radius, p.sun_mass);
    for planet in &system.planets {
        pull(planet.position, planet.radius, planet.mass);
    }
    accel
}

fn cruise(
    ship: &mut Ship,
    input: &Input,
    system: &System,
    p: &FlightParams,
    gravity: bool,
    spin: Vec3,
    dt: f32,
) {
    ship.rotate(spin.x, spin.y, spin.z);

    let boost = if input.turbo { p.turbo } else { 1.0 };
    let along = ship.velocity.dot(&ship.forward);
    let mut accel = Vec3::zeros();
    if input.throttle > 0.0 {
        accel += ship.forward * (p.accel * boost * input.throttle);
    } else if input.throttle < 0.0 {
        let push = if along > 0.0 {
            p.brake
        } else if along > -p.max_speed * p.reverse {
            p.accel * p.reverse
        } else {
            0.0
        };
        accel += ship.forward * (push * input.throttle);
    }
    if gravity {
        accel += gravity_at(ship.position, system, p);
    }

    let decay = (-p.drag * dt).exp();
    let gain = if p.drag > 1e-4 {
        (1.0 - decay) / p.drag
    } else {
        dt
    };
    ship.velocity = ship.velocity * decay + accel * gain;
    if input.throttle > 0.0 {
        let lateral = ship.velocity - ship.forward * ship.velocity.dot(&ship.forward);
        ship.velocity -= lateral * (1.0 - (-p.grip * dt).exp());
    }

    let altitude = nearest_altitude(ship.position, system);
    let limit = (p.max_speed * boost).min((altitude * p.approach).max(p.min_speed));
    let speed = ship.velocity.norm();
    if speed > limit {
        let eased = limit + (speed - limit) * (-3.0 * dt).exp();
        ship.velocity *= eased / speed;
    }

    ship.position += ship.velocity * dt;
}

fn enter_orbit(
    ship: &Ship,
    planet: &Planet,
    index: usize,
    drift: Vec3,
    p: &FlightParams,
) -> OrbitState {
    let rel = ship.position - planet.position;
    let radius = rel.norm().max(1e-3);
    let dir = rel / radius;
    let velocity = ship.velocity - drift;
    let base = if velocity.norm() > 0.5 {
        velocity
    } else {
        ship.forward
    };
    let candidates = [base, ship.forward, ship.up, Vec3::y(), Vec3::x()];
    let heading = candidates
        .iter()
        .map(|v| v - dir * v.dot(&dir))
        .find(|t| t.norm() > 1e-3)
        .map(|t| t.normalize())
        .unwrap_or_else(Vec3::z);
    let altitude = p
        .orbit_altitude
        .clamp(p.orbit_min_altitude, p.orbit_max_altitude * planet.radius);
    let speed = velocity
        .dot(&heading)
        .clamp(-p.orbit_max_speed * 0.5, p.orbit_max_speed * p.turbo);
    let floor = ground(planet, dir);
    OrbitState {
        planet: index,
        altitude,
        speed,
        dir,
        heading,
        radius,
        floor,
        height: radius - floor,
    }
}

fn orbit(
    s: &mut OrbitState,
    ship: &mut Ship,
    input: &Input,
    planet: &Planet,
    p: &FlightParams,
    yaw: f32,
    dt: f32,
) {
    let boost = if input.turbo { p.turbo } else { 1.0 };
    s.speed += input.throttle * p.orbit_accel * boost * dt;
    let top = p.orbit_max_speed * boost;
    if s.speed > top {
        s.speed = top + (s.speed - top) * (-3.0 * dt).exp();
    }
    s.speed = s.speed.max(-p.orbit_max_speed * 0.5);

    s.heading = glm::rotate_vec3(&s.heading, yaw, &s.dir);
    let arc = s.speed * dt / s.radius.max(0.1);
    let (sin, cos) = arc.sin_cos();
    let dir = s.dir * cos + s.heading * sin;
    let heading = s.heading * cos - s.dir * sin;
    s.dir = dir.normalize();
    s.heading = (heading - s.dir * heading.dot(&s.dir)).normalize();

    s.altitude = (s.altitude + input.lift * p.orbit_climb * boost * dt)
        .clamp(p.orbit_min_altitude, p.orbit_max_altitude * planet.radius);
    let floor = ground(planet, s.dir);
    s.floor += (floor - s.floor) * (1.0 - (-FLOOR_RATE * dt).exp());
    let step = (s.altitude - s.height) * (1.0 - (-4.0 * dt).exp());
    let cap = p.orbit_climb * 3.0 * dt;
    s.height += step.clamp(-cap, cap);
    s.radius = (s.floor + s.height).max(floor + p.margin);

    let old = ship.position;
    ship.position = planet.position + s.dir * s.radius;
    if dt > 0.0 {
        ship.velocity = (ship.position - old) / dt;
    }
    ship.align(s.heading, s.dir, 1.0 - (-5.0 * dt).exp());
}

fn collide(ship: &mut Ship, system: &System, p: &FlightParams) {
    let clearance = |ship: &Ship, dir: Vec3| p.margin + p.nose * ship.forward.dot(&dir).abs();
    for planet in &system.planets {
        let d = ship.position - planet.position;
        let dist = d.norm();
        if dist < 1e-4 {
            continue;
        }
        let dir = d / dist;
        let floor = ground(planet, dir) + clearance(ship, dir);
        push_out(ship, planet.position, dir, dist, floor);
    }
    let dist = ship.position.norm();
    if dist > 1e-4 {
        let dir = ship.position / dist;
        let floor = system.sun.radius + clearance(ship, dir);
        push_out(ship, Vec3::zeros(), dir, dist, floor);
    }
}

fn push_out(ship: &mut Ship, center: Vec3, dir: Vec3, dist: f32, floor: f32) {
    if dist >= floor {
        return;
    }
    ship.position = center + dir * floor;
    let radial = ship.velocity.dot(&dir);
    if radial < 0.0 {
        ship.velocity -= dir * radial;
    }
}

#[cfg(test)]
mod tests;
