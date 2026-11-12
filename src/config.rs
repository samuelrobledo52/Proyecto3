use std::collections::HashMap;
use std::fs;

const PATH: &str = "assets/config.txt";

#[derive(Clone, Default)]
pub struct Config {
    values: HashMap<String, f32>,
}

impl Config {
    pub fn load() -> Config {
        match fs::read_to_string(PATH) {
            Ok(text) => Config::parse(&text),
            Err(error) => {
                eprintln!("No se pudo leer {PATH}: {error}");
                Config::default()
            }
        }
    }

    fn parse(text: &str) -> Config {
        let mut values = HashMap::new();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match value.trim().parse::<f32>() {
                Ok(number) => {
                    values.insert(key.trim().to_lowercase(), number);
                }
                Err(_) => eprintln!("Valor inválido en {PATH}: {line}"),
            }
        }
        Config { values }
    }

    pub fn get(&self, key: &str, default: f32) -> f32 {
        self.values.get(key).copied().unwrap_or(default)
    }

    pub fn seed(&self) -> u32 {
        self.get("seed", 7.0) as u32
    }
}
