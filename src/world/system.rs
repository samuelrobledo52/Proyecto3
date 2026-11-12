use std::f32::consts::TAU;

use crate::config::Config;
use crate::math::{model_matrix, Mat4, Vec3};
use crate::render::mesh::Mesh;
use crate::render::shaders::ShaderKind;
use crate::world::icosphere::Spheres;
use crate::world::planet::{craters, lod_meshes, pick_lod, ring, Orbit, Planet, Relief, Spec};
use crate::world::scatter::{scatter, Rule, CRYSTAL, ROCK, TOWER, TREE};

pub struct Sun {
    pub radius: f32,
    pub lods: Vec<Mesh>,
    pub spin: f32,
    pub spin_speed: f32,
    pub shader: ShaderKind,
}

impl Sun {
    pub fn model(&self, camera: Vec3) -> Mat4 {
        model_matrix(-camera, Vec3::new(0.0, self.spin, 0.0), self.radius)
    }

    pub fn lod_level(&self, distance: f32, pixel_scale: f32, thresholds: &[f32; 3]) -> usize {
        pick_lod(
            self.radius,
            distance,
            pixel_scale,
            thresholds,
            self.lods.len(),
        )
    }
}

pub struct System {
    pub sun: Sun,
    pub planets: Vec<Planet>,
    pub time: f32,
    pub seed: u32,
}

struct Defaults {
    radius: f32,
    orbit: f32,
    orbit_speed: f32,
    spin_speed: f32,
    phase: f32,
    amp: f32,
}

impl System {
    pub fn new(config: &Config) -> Self {
        let seed = config.seed();
        let spheres = Spheres::new();
        let sun = Sun {
            radius: config.get("sun.radius", 18.0),
            lods: lod_meshes(&spheres, |_| 1.0),
            spin: 0.0,
            spin_speed: config.get("sun.spin_speed", 0.05),
            shader: ShaderKind::Sun {
                intensity: config.get("sun.intensity", 1.0),
            },
        };

        let ignis_amp = config.get("ignis.amp", 0.10);
        let verdania_amp = config.get("verdania.amp", 0.08);
        let sea = config.get("verdania.sea", 0.0);
        let glacia_amp = config.get("glacia.amp", 0.05);
        let ruinas_amp = config.get("ruinas.amp", 0.06);
        let crater_count = config.get("ruinas.crater_count", 7.0).max(0.0) as u32;

        let specs = [
            spec(
                config,
                "Ignis",
                Defaults {
                    radius: 6.0,
                    orbit: 75.0,
                    orbit_speed: 0.045,
                    spin_speed: 0.3,
                    phase: 0.4,
                    amp: ignis_amp,
                },
                Relief::Volcanic,
                ShaderKind::Lava {
                    amp: ignis_amp,
                    intensity: config.get("ignis.intensity", 1.0),
                },
            ),
            spec(
                config,
                "Verdania",
                Defaults {
                    radius: 8.0,
                    orbit: 125.0,
                    orbit_speed: 0.032,
                    spin_speed: 0.25,
                    phase: 2.3,
                    amp: verdania_amp,
                },
                Relief::Continents { sea },
                ShaderKind::Terrain {
                    amp: verdania_amp,
                    sea,
                },
            ),
            spec(
                config,
                "Glacia",
                Defaults {
                    radius: 7.0,
                    orbit: 185.0,
                    orbit_speed: 0.023,
                    spin_speed: 0.2,
                    phase: 4.1,
                    amp: glacia_amp,
                },
                Relief::Glacial,
                ShaderKind::Ice { amp: glacia_amp },
            ),
            spec(
                config,
                "Aether",
                Defaults {
                    radius: 14.0,
                    orbit: 265.0,
                    orbit_speed: 0.015,
                    spin_speed: 0.35,
                    phase: 1.1,
                    amp: 0.0,
                },
                Relief::Smooth,
                ShaderKind::Gas {
                    intensity: config.get("aether.intensity", 1.0),
                },
            ),
            spec(
                config,
                "Ruinas",
                Defaults {
                    radius: 7.5,
                    orbit: 350.0,
                    orbit_speed: 0.01,
                    spin_speed: 0.15,
                    phase: 5.4,
                    amp: ruinas_amp,
                },
                Relief::Cratered(craters(seed, crater_count)),
                ShaderKind::Ruins {
                    amp: ruinas_amp,
                    craters: config.get("ruinas.craters", 6.0),
                    bump: config.get("ruinas.bump", 1.0),
                },
            ),
        ];

        let mut planets: Vec<Planet> = specs
            .into_iter()
            .enumerate()
            .map(|(i, mut s)| {
                s.seed = seed.wrapping_add(1 + i as u32 * 7919);
                Planet::new(s, &spheres)
            })
            .collect();

        if let Some(aether) = planets.iter_mut().find(|p| p.name == "Aether") {
            aether.tilt = config.get("aether.tilt", 26.0).to_radians();
            aether.ring = Some(ring(
                config.get("aether.ring_inner", 1.5),
                config.get("aether.ring_outer", 2.4),
            ));
        }

        let scatter_seed = config.get("scatter.seed", seed as f32) as u32;
        for (i, planet) in planets.iter_mut().enumerate() {
            let rules = rules(&planet.name, config, sea);
            planet.instances = scatter(
                planet,
                &rules,
                scatter_seed.wrapping_add(i as u32 * 104_729),
            );
        }

        System {
            sun,
            planets,
            time: 0.0,
            seed,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.sun.spin = (self.sun.spin + self.sun.spin_speed * dt) % TAU;
        for planet in &mut self.planets {
            planet.advance(dt);
        }
    }
}

fn spec(
    config: &Config,
    name: &'static str,
    d: Defaults,
    relief: Relief,
    shader: ShaderKind,
) -> Spec {
    let key = |k: &str, default: f32| config.get(&format!("{}.{k}", name.to_lowercase()), default);
    let radius = key("radius", d.radius);
    Spec {
        name,
        radius,
        mass: key("mass", radius * radius * radius),
        orbit: Orbit {
            radius: key("orbit", d.orbit),
            speed: key("orbit_speed", d.orbit_speed),
            phase: key("phase", d.phase),
        },
        spin_speed: key("spin_speed", d.spin_speed),
        tilt: 0.0,
        amp: d.amp,
        relief,
        shader,
        seed: 0,
    }
}

fn rules(name: &str, config: &Config, sea: f32) -> Vec<Rule> {
    let density = config.get("scatter.density", 1.0).max(0.0);
    let lower = name.to_lowercase();
    let count = |k: &str, d: f32| {
        (config.get(&format!("{lower}.{k}"), d) * density)
            .round()
            .max(0.0) as u32
    };
    let size = |k: &str, d: f32| config.get(&format!("scatter.{k}_size"), d);
    let land = |x: f32| sea + x * (1.0 - sea);
    match name {
        "Ignis" => vec![Rule {
            kind: ROCK,
            count: count("rocks", 80.0),
            size: size("rock", 0.7),
            heights: [0.1, 1.0],
            max_slope: 0.2,
            noise_freq: 2.0,
            noise_min: -0.2,
        }],
        "Verdania" => vec![
            Rule {
                kind: TREE,
                count: count("trees", 250.0),
                size: size("tree", 0.9),
                heights: [land(0.14), land(0.6)],
                max_slope: 0.08,
                noise_freq: 2.5,
                noise_min: -0.12,
            },
            Rule {
                kind: ROCK,
                count: count("rocks", 60.0),
                size: size("rock", 0.7),
                heights: [land(0.06), land(0.85)],
                max_slope: 0.12,
                noise_freq: 3.0,
                noise_min: -0.1,
            },
        ],
        "Glacia" => vec![Rule {
            kind: CRYSTAL,
            count: count("crystals", 120.0),
            size: size("crystal", 0.8),
            heights: [-1.0, 0.3],
            max_slope: 0.05,
            noise_freq: 2.5,
            noise_min: -0.05,
        }],
        "Ruinas" => vec![
            Rule {
                kind: TOWER,
                count: count("towers", 40.0),
                size: size("tower", 1.0),
                heights: [0.25, 1.0],
                max_slope: 0.03,
                noise_freq: 1.8,
                noise_min: -0.15,
            },
            Rule {
                kind: ROCK,
                count: count("rocks", 80.0),
                size: size("rock", 0.7),
                heights: [-1.0, 1.0],
                max_slope: 0.2,
                noise_freq: 3.0,
                noise_min: -0.2,
            },
        ],
        _ => Vec::new(),
    }
}
