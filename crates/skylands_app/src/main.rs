use macroquad::camera::Camera;
use macroquad::prelude::*;
use skylands_core::{BuildingKind, Command, GameState, RunState, TileCoord};

const TILE_SIZE: f32 = 1.0;
const TILE_HEIGHT: f32 = 0.25;
const MAX_CATCH_UP_TICKS: u32 = 3;

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = GameState::empty();
    game.apply(Command::StartRun { seed: 1 })
        .expect("initial Run should start");

    let mut camera = CityCamera::default();
    let mut tick_accumulator = 0.0_f32;

    loop {
        clear_background(Color::from_rgba(139, 198, 218, 255));

        let frame_time = get_frame_time();
        tick_accumulator += frame_time;
        let mut catch_up_ticks = 0;

        while tick_accumulator >= 1.0 && catch_up_ticks < MAX_CATCH_UP_TICKS {
            game.apply(Command::Tick).expect("Run should keep ticking");
            tick_accumulator -= 1.0;
            catch_up_ticks += 1;
        }

        if catch_up_ticks == MAX_CATCH_UP_TICKS {
            tick_accumulator = 0.0;
        }

        camera.update();

        let run = game.current_run.as_ref().expect("Run should exist");
        let hovered_tile = pick_tile(run, &camera);

        set_camera(&camera.to_macroquad());
        draw_run(run, hovered_tile);
        set_default_camera();

        draw_hud(run);

        next_frame().await;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Skylands".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        ..Default::default()
    }
}

#[derive(Debug, Clone)]
struct CityCamera {
    target: Vec3,
    distance: f32,
}

impl Default for CityCamera {
    fn default() -> Self {
        Self {
            target: vec3(0.5, 0.0, 0.5),
            distance: 13.0,
        }
    }
}

impl CityCamera {
    fn update(&mut self) {
        let pan_speed = 8.0 * get_frame_time() * self.distance / 13.0;

        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            self.target.x -= pan_speed;
            self.target.z += pan_speed;
        }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            self.target.x += pan_speed;
            self.target.z -= pan_speed;
        }
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            self.target.x -= pan_speed;
            self.target.z -= pan_speed;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            self.target.x += pan_speed;
            self.target.z += pan_speed;
        }

        let (_wheel_x, wheel_y) = mouse_wheel();
        if wheel_y != 0.0 {
            self.distance = (self.distance - wheel_y * 0.8).clamp(5.0, 28.0);
        }
    }

    fn to_macroquad(&self) -> Camera3D {
        Camera3D {
            position: self.target + vec3(-self.distance, self.distance * 0.75, -self.distance),
            up: vec3(0.0, 1.0, 0.0),
            target: self.target,
            fovy: 45.0,
            projection: Projection::Perspective,
            ..Default::default()
        }
    }
}

fn draw_run(run: &RunState, hovered_tile: Option<TileCoord>) {
    for island in &run.islands {
        for tile in island.tiles() {
            let is_hovered = hovered_tile == Some(tile.coord);
            draw_tile(tile.coord, tile.height, is_hovered);
        }
    }

    for building in &run.buildings {
        let color = match building.kind {
            BuildingKind::CityCore => Color::from_rgba(210, 75, 64, 255),
            BuildingKind::House => Color::from_rgba(236, 198, 110, 255),
            BuildingKind::Farm => Color::from_rgba(92, 168, 95, 255),
            BuildingKind::Workshop => Color::from_rgba(133, 129, 120, 255),
            BuildingKind::Market => Color::from_rgba(98, 113, 191, 255),
            BuildingKind::Monument => Color::from_rgba(186, 184, 196, 255),
        };
        draw_building(
            building.origin,
            run.tile_height(building.origin).unwrap_or(0),
            color,
        );
    }
}

fn draw_tile(coord: TileCoord, height: i32, is_hovered: bool) {
    let center = tile_center(coord, height);
    let color = if is_hovered {
        Color::from_rgba(247, 229, 104, 255)
    } else {
        Color::from_rgba(91, 152, 86, 255)
    };

    draw_cube(
        center - vec3(0.0, TILE_HEIGHT * 0.5, 0.0),
        vec3(TILE_SIZE, TILE_HEIGHT, TILE_SIZE),
        None,
        color,
    );
    draw_cube_wires(
        center - vec3(0.0, TILE_HEIGHT * 0.5, 0.0),
        vec3(TILE_SIZE, TILE_HEIGHT, TILE_SIZE),
        Color::from_rgba(57, 92, 65, 255),
    );
}

fn draw_building(origin: TileCoord, height: i32, color: Color) {
    let center = tile_center(origin, height) + vec3(TILE_SIZE * 0.5, 0.45, TILE_SIZE * 0.5);
    draw_cube(center, vec3(1.75, 0.9, 1.75), None, color);
    draw_cube_wires(center, vec3(1.75, 0.9, 1.75), BLACK);
}

fn tile_center(coord: TileCoord, height: i32) -> Vec3 {
    vec3(
        coord.x as f32 * TILE_SIZE,
        height as f32 * TILE_HEIGHT,
        coord.z as f32 * TILE_SIZE,
    )
}

fn pick_tile(run: &RunState, camera: &CityCamera) -> Option<TileCoord> {
    let mouse = mouse_position();
    let ray = screen_ray(vec2(mouse.0, mouse.1), &camera.to_macroquad());

    run.islands
        .iter()
        .flat_map(|island| island.tiles())
        .filter_map(|tile| {
            let center = tile_center(tile.coord, tile.height);
            let min = center + vec3(-0.5, -TILE_HEIGHT, -0.5);
            let max = center + vec3(0.5, 0.08, 0.5);
            ray_box_distance(ray.origin, ray.direction, min, max)
                .map(|distance| (distance, tile.coord))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, coord)| coord)
}

#[derive(Debug, Clone, Copy)]
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

fn screen_ray(screen_position: Vec2, camera: &Camera3D) -> Ray {
    let normalized = vec2(
        (screen_position.x / screen_width()) * 2.0 - 1.0,
        1.0 - (screen_position.y / screen_height()) * 2.0,
    );
    let inverse = camera.matrix().inverse();
    let near = inverse.project_point3(vec3(normalized.x, normalized.y, -1.0));
    let far = inverse.project_point3(vec3(normalized.x, normalized.y, 1.0));

    Ray {
        origin: near,
        direction: (far - near).normalize(),
    }
}

fn ray_box_distance(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let inverse = vec3(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z);

    let t1 = (min.x - origin.x) * inverse.x;
    let t2 = (max.x - origin.x) * inverse.x;
    let t3 = (min.y - origin.y) * inverse.y;
    let t4 = (max.y - origin.y) * inverse.y;
    let t5 = (min.z - origin.z) * inverse.z;
    let t6 = (max.z - origin.z) * inverse.z;

    let t_min = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
    let t_max = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

    if t_max >= 0.0 && t_min <= t_max {
        Some(t_min.max(0.0))
    } else {
        None
    }
}

fn draw_hud(run: &RunState) {
    let panel_color = Color::from_rgba(23, 29, 33, 220);
    draw_rectangle(16.0, 16.0, 330.0, 132.0, panel_color);

    let lines = [
        format!("Run time: {}s", run.elapsed_seconds),
        format!(
            "SkyCoin: {} (-{:.1}/s)",
            run.sky_coin, run.sky_coin_drain_per_second
        ),
        format!(
            "Food: {}/{} ({:+.1}/s)",
            run.food.current,
            run.food.cap,
            run.food.production_per_second - run.food.consumption_per_second
        ),
        format!("Citizens: {}/{}", run.citizens, run.citizen_capacity),
    ];

    for (index, line) in lines.iter().enumerate() {
        draw_text(line, 32.0, 48.0 + index as f32 * 28.0, 24.0, WHITE);
    }
}
