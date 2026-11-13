# Proyecto 3 — Sistema solar con software renderer

Un sistema solar pequeño, con estilo low-poly inspirado en Mario Galaxy, que se explora en una nave. Todo el 3D pasa por un pipeline de software propio escrito en Rust: matrices de modelo, vista y proyección, recorte contra el near plane, rasterizado con baricéntricas, z-buffer y fragment shaders. raylib solo abre la ventana, lee el teclado, el mouse y el gamepad, y muestra el framebuffer como una única textura.

Hay un sol con plasma animado y cinco planetas con identidad propia: **Ignis** (volcánico con lava), **Verdania** (océano y bosques), **Glacia** (hielo y cristales), **Aether** (gigante gaseoso con anillo) y **Ruinas** (cráteres y torres). La nave puede volar entre ellos, acercarse hasta ver la superficie, orbitarlos y alejarse sin pantallas de carga.

Universidad del Valle de Guatemala, Gráficas por Computadora.

## Video

> Enlace al video: https://youtu.be/ctcXOWJu3BI?si=EvG8yEYAX4jj_v_O

## Cómo correr

Requisitos: Rust estable (edición 2021) y las herramientas para compilar raylib (CMake y un compilador de C; en Windows, Visual Studio Build Tools).

```
cargo run --release
```

Siempre en `--release`: en debug el renderer por software es muy lento.

Opciones:

| Comando | Qué hace |
|---|---|
| `cargo run --release -- --bench` | Recorrido fijo de 20 s que pasa cerca de cada planeta; escribe `docs/bench.csv` e imprime FPS y triángulos por etapa |
| `cargo run --release -- --bench --no-cull` | Igual, sin frustum culling (comparación) |
| `cargo run --release -- --bench --no-lod` | Igual, sin niveles de detalle (comparación) |
| `cargo test --release` | 7 pruebas del vuelo: colisión, mismo recorrido con distintos dt, gravedad, órbita, saltos y cámara |

Los modelos de superficie se regeneran con Blender 5.2:

```
"C:/Program Files/Blender Foundation/Blender 5.2/blender.exe" -b --python tools/blender/build_all.py
```

## Controles

| Tecla | Acción |
|---|---|
| W / S | Acelerar / frenar (y reversa lenta) |
| A / D | Girar (guiñada) |
| Flecha arriba / abajo | Cabeceo |
| Q / E | Alabeo a la izquierda / derecha |
| X | Air roll sí / no: mientras está activo, A/D alabean en vez de girar (se ve en el HUD) |
| Shift | Turbo |
| Clic derecho + mouse | Mirar alrededor de la nave |
| Rueda | Distancia de la cámara |
| O | Entrar / salir de órbita (aparece "O  Orbitar <planeta>" a menos de 4 radios) |
| R / F | Subir / bajar en órbita |
| G | Gravedad sí / no |
| 1 – 5 | Saltar cerca de Ignis, Verdania, Glacia, Aether, Ruinas |
| C | Frustum culling sí / no |
| L | Niveles de detalle (LOD) sí / no |
| B | Normal mapping de Ruinas sí / no |
| Tab | Mostrar / ocultar el panel "Triángulos por etapa" |
| H | Ayuda |
| F5 | Recargar `assets/config.txt` y reconstruir el sistema |
| Esc | Salir |

La rotación funciona por velocidad angular en los tres ejes locales de la nave: cada eje acelera mientras se mantiene su tecla hasta un máximo y se amortigua al soltarla, como un auto en el aire. La orientación es una base ortonormal que se re-ortonormaliza cada frame, así que se pueden dar vueltas completas en cualquier eje sin bloqueos. El empuje siempre sale hacia la punta de la nave, de modo que tras un giro se puede acelerar en cualquier dirección. La cámara de persecución interpola su vector "arriba" hacia el de la nave para seguir los giros sin marear.

En órbita: W/S cambian la velocidad orbital, A/D el rumbo sobre la superficie y R/F la altura. El cabeceo y el alabeo solo inclinan la nave visualmente (vuelve a nivelarse al soltar) y no cambian la trayectoria; al salir de la órbita la nave conserva la inclinación que tenía. El air roll (X) se mantiene al entrar en órbita: A/D inclinan la nave en vez de cambiar el rumbo.

Gamepad (opcional): stick izquierdo para girar y cabecear, gatillos para acelerar y frenar, A para órbita, B para turbo, LB/RB para el alabeo, cruceta para la altura y stick derecho para mirar.

## Configuración en vivo

`assets/config.txt` controla la semilla, los radios, las órbitas, las velocidades orbitales y de rotación, la intensidad de los shaders, la densidad de objetos, las distancias de LOD, la gravedad y los parámetros de la nave y la cámara. Al guardar y pulsar **F5**, se reconstruyen los planetas, las mallas y las instancias sin reiniciar y conservando el tiempo de la simulación. Ejemplos: `verdania.radius = 11`, `scatter.seed = 99`, `ignis.orbit_speed = 0.2`, `ruinas.bump = 0`, `gravity = 2`.

## Integrante y contribución

| Integrante | Contribución |
|---|---|
| Samuel Robledo | Todo el proyecto: pipeline de software (MVP, recorte, rasterizado, z-buffer, shaders, post-proceso), generación de planetas (icosfera, ruido, deformación, LOD), modelos de superficie con scripts de Blender y su distribución procedural, control de vuelo (crucero, órbita, gravedad, colisión), HUD, benchmark y documentación. La nave es mi modelo del Lab 3. |

## Features del menú y dónde verlas



| Feature | Pts | Dónde verla | Video | Código |
|---|---|---|---|---|
| 5 planetas distinguibles | 50 | Teclas 1–5 | 0:30, 1:25, 1:50, 2:05, 2:15 | `src/world/system.rs`, `src/world/planet.rs` |
| 4 tipos de modelo propios (árbol, roca, torre, cristal) | 20 | Verdania (árboles y rocas), Glacia (cristales), Ruinas (torres y rocas), Ignis (rocas) | 0:50 | `tools/blender/*.py`, `assets/models/` |
| Distribución procedural con semilla | 10 | `scatter.seed` + F5 | 2:50 (opcional) | `src/world/scatter.rs` → `scatter` |
| Orientación sobre superficie que rota | 5 | Órbita en Verdania: los árboles giran con el planeta | 0:50 | `src/world/scatter.rs` → `orient`, `Planet::frame` |
| 8 shaders con implementación distinta | 24 | Sol, Ignis, Verdania, Glacia, Aether y su anillo, Ruinas, nave | 0:08–2:15 | `src/render/shaders.rs` |
| Shader animado | 8 | Plasma del sol, lava de Ignis, bandas de Aether | 0:08, 1:50 | `shaders.rs` → `sun`, `lava`, `gas` |
| Normal mapping | 8 | Ruinas, tecla B | 2:15 | `shaders.rs` → `ruins`, `crater_field` |
| Deformación con ruido | 10 | Relieve de todos los planetas rocosos | 1:50 | `src/world/planet.rs` → `lod_meshes`, `src/world/noise.rs` |
| Subdivisión de geometría | 10 | Icosfera de 4 niveles (HUD "LOD Sí (nivel n)") | 0:30 | `src/world/icosphere.rs` |
| Modo órbita local | 8 | Tecla O cerca de un planeta | 0:50 | `src/ship/flight.rs` → `toggle_orbit`, `orbit` |
| Aceleración, frenado y giro progresivos con dt | 8 | W / S / A / D | 0:08 | `flight.rs` → `cruise`; test `same_path_any_dt` |
| Gravedad | 10 | Tecla G cerca de Glacia | 1:25 | `flight.rs` → `gravity_at`; test `gravity_bends` |
| Colisión | 4 | Volar directo contra cualquier planeta | — | `flight.rs` → `collide`; test `collision_holds` |
| Frustum culling con contadores | 7 | Tecla C con el panel Tab | 2:30 | `src/render/pipeline.rs` → `visible`; `docs/bench_no_cull.csv` |
| LOD con contadores | 8 | Tecla L con el panel Tab | 2:30 | `planet.rs` → `pick_lod`, `scatter.rs` → `pick`; `docs/bench_no_lod.csv` |
| Presentación | 10 | Paleta, halo del sol, vignette, HUD, ayuda H, marcador del planeta cercano | todo el video | `src/render/post.rs`, `src/hud.rs` |

## Rendimiento

Framebuffer interno de 960×540, rasterizado en paralelo por bandas con rayon. Benchmark de 20 s en un i7-9750H (6 núcleos / 12 hilos):

| Bench | FPS prom | Triángulos enviados | Ahorro por culling | Ahorro por LOD |
|---|---|---|---|---|
| normal | 203 (9 nov) / 169 con batería (12 nov) | 8.8k | 5.1k | 195k |
| `--no-cull` | 143 (batería) | 13.8k | 0 | 195k |
| `--no-lod` | 100 (batería) | 37.2k | 171k | 0 |

En cada frame, triángulos enviados + ahorro por culling + ahorro por LOD = 208 588, el total de la escena. Detalle en `docs/PROGRESO.md` y en `docs/bench*.csv`.

## Estructura

```
src/
  main.rs  hud.rs  bench.rs  config.rs  math.rs  camera.rs
  render/  framebuffer.rs pipeline.rs clip.rs raster.rs shaders.rs post.rs mesh.rs stats.rs
  world/   noise.rs icosphere.rs planet.rs scatter.rs system.rs skybox.rs
  ship/    ship.rs flight.rs flight/tests.rs
assets/models/  nave.obj + tree, rock, tower, crystal (alto y _lo)
assets/config.txt
tools/blender/  scripts de Python que generan los modelos
docs/  enunciado, guía de estudio, guion del video, rúbrica, progreso, benchmarks
```

## Recursos externos

- [raylib](https://www.raylib.com/) (crate `raylib`): ventana, entrada y presentación de una textura.
- `nalgebra-glm` (matemática), `rayon` (paralelismo), `tobj` (lectura de .obj).
- El ruido Perlin, la icosfera, el pipeline y todos los modelos son propios.

## Uso de IA

Se utilizó Claude como asistente en el desarrollo, corrección y revisión del código. Todo el código fue revisado, probado, hecho y comprendido por mí.
