use macroquad::camera::Camera;
use macroquad::prelude::*;
use skylands_core::{FlyingIsland, RunState, TileCoord};

pub const TILE_SIZE: f32 = 1.0;
pub const TILE_HEIGHT: f32 = 0.25;

const EMPTY_TILE_PICK_RADIUS_PADDING: i32 = 10;
const MAX_MEDIUM_TERRAIN_CAMERA_DISTANCE: f32 = 22.0;

#[derive(Debug, Clone)]
pub struct CityCamera {
    pub target: Vec3,
    pub distance: f32,
    yaw: f32,
}

impl Default for CityCamera {
    fn default() -> Self {
        Self {
            target: vec3(0.5, 0.0, 0.5),
            distance: 13.0,
            yaw: -std::f32::consts::FRAC_PI_4,
        }
    }
}

impl CityCamera {
    pub fn update(&mut self) {
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

        let rotation_speed = 1.6 * get_frame_time();
        if is_key_down(KeyCode::Q) {
            self.yaw -= rotation_speed;
        }
        if is_key_down(KeyCode::E) {
            self.yaw += rotation_speed;
        }
    }

    pub fn to_macroquad(&self) -> Camera3D {
        let horizontal = vec3(self.yaw.cos(), 0.0, self.yaw.sin()) * self.distance;
        Camera3D {
            position: self.target + horizontal + vec3(0.0, self.distance * 0.75, 0.0),
            up: vec3(0.0, 1.0, 0.0),
            target: self.target,
            fovy: 45.0,
            projection: Projection::Perspective,
            ..Default::default()
        }
    }

    fn target_tile(&self) -> TileCoord {
        TileCoord::new(self.target.x.round() as i32, self.target.z.round() as i32)
    }

    fn visible_tile_radius(&self) -> i32 {
        (self.distance * 1.6).ceil() as i32 + EMPTY_TILE_PICK_RADIUS_PADDING
    }
}

pub fn tile_center(coord: TileCoord, height: i32) -> Vec3 {
    vec3(
        coord.x as f32 * TILE_SIZE,
        height as f32 * TILE_HEIGHT,
        coord.z as f32 * TILE_SIZE,
    )
}

pub fn pick_tile(run: &RunState, camera: &CityCamera) -> Option<TileCoord> {
    let mouse = mouse_position();
    let ray = screen_ray(vec2(mouse.0, mouse.1), &camera.to_macroquad());

    let island_hit = run
        .islands
        .iter()
        .filter(|island| island_is_near_camera(island, camera))
        .flat_map(|island| island.tiles())
        .filter_map(|tile| {
            let center = tile_center(tile.coord, tile.height);
            let min = center + vec3(-0.5, -TILE_HEIGHT, -0.5);
            let max = center + vec3(0.5, 0.08, 0.5);
            ray_box_distance(ray.origin, ray.direction, min, max)
                .map(|distance| (distance, tile.coord))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, coord)| coord);

    island_hit.or_else(|| pick_empty_tile(ray, camera))
}

fn pick_empty_tile(ray: Ray, camera: &CityCamera) -> Option<TileCoord> {
    if ray.direction.y.abs() < f32::EPSILON {
        return None;
    }

    let distance = -ray.origin.y / ray.direction.y;
    if distance < 0.0 {
        return None;
    }

    let hit = ray.origin + ray.direction * distance;
    let coord = TileCoord::new(hit.x.round() as i32, hit.z.round() as i32);
    if coord.manhattan_distance_to(camera.target_tile()) <= camera.visible_tile_radius() {
        Some(coord)
    } else {
        None
    }
}

pub fn island_is_near_camera(island: &FlyingIsland, camera: &CityCamera) -> bool {
    let Some((min, max)) = island.bounds() else {
        return false;
    };

    let target = camera.target_tile();
    let radius = camera.visible_tile_radius();

    min.x <= target.x + radius
        && max.x >= target.x - radius
        && min.z <= target.z + radius
        && max.z >= target.z - radius
}

pub fn terrain_wires_visible(camera: &CityCamera) -> bool {
    camera.distance <= MAX_MEDIUM_TERRAIN_CAMERA_DISTANCE
}

pub fn terrain_tile_draw_count(
    run: &RunState,
    camera: &CityCamera,
    _hovered_tile: Option<TileCoord>,
) -> usize {
    visible_terrain_tile_count(run, camera)
}

pub fn terrain_wire_draw_count(run: &RunState, camera: &CityCamera) -> usize {
    if !terrain_wires_visible(camera) {
        return 0;
    }

    run.islands
        .iter()
        .filter(|island| island_is_near_camera(island, camera))
        .flat_map(|island| island.tiles())
        .count()
}

pub fn visible_terrain_tile_count(run: &RunState, camera: &CityCamera) -> usize {
    run.islands
        .iter()
        .filter(|island| island_is_near_camera(island, camera))
        .map(|island| island.tiles().count())
        .sum()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera_culls_most_generated_island_tiles() {
        let run = RunState::start(7);
        let camera = CityCamera::default();
        let draw_count = terrain_tile_draw_count(&run, &camera, None);

        assert!(
            draw_count < 2_000,
            "terrain tile draw count was {draw_count}"
        );
    }

    #[test]
    fn zoomed_out_camera_skips_terrain_wires_without_skipping_tiles() {
        let run = RunState::start(7);
        let camera = CityCamera {
            distance: 28.0,
            ..CityCamera::default()
        };
        let visible_tile_count = visible_terrain_tile_count(&run, &camera);
        let draw_count = terrain_tile_draw_count(&run, &camera, None);
        let wire_count = terrain_wire_draw_count(&run, &camera);

        assert!(
            visible_tile_count > 2_000,
            "test should cover zoomed-out terrain growth; visible tile count was {visible_tile_count}"
        );
        assert_eq!(draw_count, visible_tile_count);
        assert_eq!(wire_count, 0);
    }

    #[test]
    fn zoomed_out_camera_still_draws_every_visible_terrain_tile() {
        let run = RunState::start(7);
        let camera = CityCamera {
            distance: 28.0,
            ..CityCamera::default()
        };
        let visible_tile_count = visible_terrain_tile_count(&run, &camera);
        let draw_count = terrain_tile_draw_count(&run, &camera, None);

        assert_eq!(draw_count, visible_tile_count);
    }
}
