use macroquad::camera::Camera;
use macroquad::prelude::*;
use skylands_core::{
    BuildingKind, Command, CommandError, CommandOutcome, GameState, Road, RoadKind, RunState,
    RunStatus, TileCoord,
};

const TILE_SIZE: f32 = 1.0;
const TILE_HEIGHT: f32 = 0.25;
const ROAD_HEIGHT: f32 = 0.06;
const MAX_CATCH_UP_TICKS: u32 = 3;
const EMPTY_TILE_PICK_COORD_LIMIT: i32 = 32;
const TOOLBAR_HEIGHT: f32 = 76.0;

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = GameState::empty();
    game.apply(Command::StartRun { seed: 1 })
        .expect("initial Run should start");

    let mut camera = CityCamera::default();
    let mut ui = AppUi::default();
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
        handle_tool_hotkeys(&mut ui);

        let hovered_tile = {
            let run = game.current_run.as_ref().expect("Run should exist");
            pick_tile(run, &camera)
        };
        let pointer_over_toolbar = toolbar_rect().contains(mouse_vec());

        handle_toolbar_mouse(&mut ui);
        let run_status = game.current_run.as_ref().expect("Run should exist").status;
        handle_world_input(
            &mut game,
            &mut ui,
            hovered_tile,
            pointer_over_toolbar,
            run_status,
        );

        let run = game.current_run.as_ref().expect("Run should exist");
        let preview = Preview::from_state(run, &ui, hovered_tile);

        set_camera(&camera.to_macroquad());
        draw_run(run, hovered_tile, &preview);
        set_default_camera();

        draw_hud(run, hovered_tile, &ui, &preview);
        draw_toolbar(run, ui.selected_tool);

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
struct AppUi {
    selected_tool: Tool,
    road_drag: Option<RoadDrag>,
    last_message: String,
}

impl Default for AppUi {
    fn default() -> Self {
        Self {
            selected_tool: Tool::Road,
            road_drag: None,
            last_message: "Select a tool or inspect a tile".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
struct RoadDrag {
    tiles: Vec<TileCoord>,
    current_hover: TileCoord,
}

impl RoadDrag {
    fn start(coord: TileCoord) -> Self {
        Self {
            tiles: vec![coord],
            current_hover: coord,
        }
    }

    fn push_to(&mut self, coord: TileCoord) {
        if self.current_hover == coord {
            return;
        }

        let mut current = self.current_hover;
        while current.x != coord.x {
            current = TileCoord::new(current.x + (coord.x - current.x).signum(), current.z);
            push_road_drag_step(&mut self.tiles, current);
        }

        while current.z != coord.z {
            current = TileCoord::new(current.x, current.z + (coord.z - current.z).signum());
            push_road_drag_step(&mut self.tiles, current);
        }

        self.current_hover = coord;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Road,
    Demolish,
    Building(BuildingKind),
}

const TOOLS: [Tool; 7] = [
    Tool::Road,
    Tool::Demolish,
    Tool::Building(BuildingKind::House),
    Tool::Building(BuildingKind::Farm),
    Tool::Building(BuildingKind::Workshop),
    Tool::Building(BuildingKind::Market),
    Tool::Building(BuildingKind::Monument),
];

#[derive(Debug, Clone)]
enum Preview {
    None,
    Building {
        footprint: [TileCoord; 4],
        valid: bool,
        cost: i64,
        reason: Option<String>,
    },
    Road {
        tiles: Vec<TileCoord>,
        new_roads: Vec<Road>,
        valid: bool,
        cost: i64,
        reason: Option<String>,
    },
}

impl Preview {
    fn from_state(run: &RunState, ui: &AppUi, hovered_tile: Option<TileCoord>) -> Self {
        match (ui.selected_tool, hovered_tile) {
            (Tool::Building(kind), Some(origin)) => {
                let quote = run.quote_building(kind, origin);
                Self::Building {
                    footprint: quote.footprint,
                    valid: quote.valid,
                    cost: quote.cost,
                    reason: quote.invalid_reason,
                }
            }
            (Tool::Road, _) => {
                let tiles = ui
                    .road_drag
                    .as_ref()
                    .map(|drag| drag.tiles.clone())
                    .or_else(|| hovered_tile.map(|coord| vec![coord]));

                let Some(tiles) = tiles else {
                    return Self::None;
                };

                let quote = run.quote_road_path(&tiles);
                Self::Road {
                    tiles: quote.tiles,
                    new_roads: quote.new_roads,
                    valid: quote.valid,
                    cost: quote.total_cost,
                    reason: quote.invalid_reason,
                }
            }
            _ => Self::None,
        }
    }

    fn status_line(&self) -> Option<String> {
        match self {
            Self::None => None,
            Self::Building {
                valid,
                cost,
                reason,
                ..
            } => Some(format!(
                "Building cost: {cost} SkyCoin | {}",
                validity_text(*valid, reason)
            )),
            Self::Road {
                valid,
                cost,
                reason,
                ..
            } => Some(format!(
                "Road path cost: {cost} SkyCoin | {}",
                validity_text(*valid, reason)
            )),
        }
    }
}

fn validity_text(valid: bool, reason: &Option<String>) -> String {
    if valid {
        "valid".to_owned()
    } else {
        reason.clone().unwrap_or_else(|| "invalid".to_owned())
    }
}

#[derive(Debug, Clone)]
struct CityCamera {
    target: Vec3,
    distance: f32,
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

        let rotation_speed = 1.6 * get_frame_time();
        if is_key_down(KeyCode::Q) {
            self.yaw -= rotation_speed;
        }
        if is_key_down(KeyCode::E) {
            self.yaw += rotation_speed;
        }
    }

    fn to_macroquad(&self) -> Camera3D {
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
}

fn handle_tool_hotkeys(ui: &mut AppUi) {
    for (index, tool) in TOOLS.iter().enumerate() {
        if is_key_pressed(number_key(index)) {
            ui.selected_tool = *tool;
            ui.road_drag = None;
        }
    }
}

fn number_key(index: usize) -> KeyCode {
    match index {
        0 => KeyCode::Key1,
        1 => KeyCode::Key2,
        2 => KeyCode::Key3,
        3 => KeyCode::Key4,
        4 => KeyCode::Key5,
        5 => KeyCode::Key6,
        _ => KeyCode::Key7,
    }
}

fn handle_toolbar_mouse(ui: &mut AppUi) {
    if !is_mouse_button_pressed(MouseButton::Left) {
        return;
    }

    let mouse = mouse_vec();
    for (index, tool) in TOOLS.iter().enumerate() {
        if tool_rect(index).contains(mouse) {
            ui.selected_tool = *tool;
            ui.road_drag = None;
            return;
        }
    }
}

fn handle_world_input(
    game: &mut GameState,
    ui: &mut AppUi,
    hovered_tile: Option<TileCoord>,
    pointer_over_toolbar: bool,
    run_status: RunStatus,
) {
    if run_status != RunStatus::Running {
        return;
    }

    match ui.selected_tool {
        Tool::Building(kind) => {
            if !pointer_over_toolbar
                && is_mouse_button_pressed(MouseButton::Left)
                && let Some(origin) = hovered_tile
            {
                apply_command_to_ui(game, ui, Command::PlaceBuilding { kind, origin }, |_| {
                    format!("Placed {}", building_kind_name(kind))
                });
            }
        }
        Tool::Road => handle_road_input(game, ui, hovered_tile, pointer_over_toolbar),
        Tool::Demolish => {
            if !pointer_over_toolbar
                && is_mouse_button_pressed(MouseButton::Left)
                && let Some(coord) = hovered_tile
            {
                apply_command_to_ui(
                    game,
                    ui,
                    Command::DemolishTile { coord },
                    command_outcome_text,
                );
            }
        }
    }
}

fn handle_road_input(
    game: &mut GameState,
    ui: &mut AppUi,
    hovered_tile: Option<TileCoord>,
    pointer_over_toolbar: bool,
) {
    if !pointer_over_toolbar
        && is_mouse_button_pressed(MouseButton::Left)
        && let Some(coord) = hovered_tile
    {
        ui.road_drag = Some(RoadDrag::start(coord));
    }

    if !pointer_over_toolbar
        && is_mouse_button_down(MouseButton::Left)
        && let (Some(drag), Some(coord)) = (&mut ui.road_drag, hovered_tile)
    {
        drag.push_to(coord);
    }

    if is_mouse_button_released(MouseButton::Left)
        && let Some(drag) = ui.road_drag.take()
    {
        apply_command_to_ui(
            game,
            ui,
            Command::PlaceRoadPath { path: drag.tiles },
            command_outcome_text,
        );
    }
}

fn apply_command_to_ui(
    game: &mut GameState,
    ui: &mut AppUi,
    command: Command,
    success_message: impl FnOnce(CommandOutcome) -> String,
) {
    match game.apply(command) {
        Ok(outcome) => ui.last_message = success_message(outcome),
        Err(error) => ui.last_message = command_error_text(error),
    }
}

fn command_outcome_text(outcome: CommandOutcome) -> String {
    match outcome {
        CommandOutcome::RunStarted => "Run started".to_owned(),
        CommandOutcome::Ticked => "Run advanced".to_owned(),
        CommandOutcome::BuildingPlaced { .. } => "Building placed".to_owned(),
        CommandOutcome::RoadPathPlaced { coords } => {
            let tile_text = if coords.len() == 1 { "tile" } else { "tiles" };
            format!("Placed {} Road {tile_text}", coords.len())
        }
        CommandOutcome::RoadDemolished { coord } => {
            format!("Demolished Road at ({}, {})", coord.x, coord.z)
        }
        CommandOutcome::BuildingDemolished { kind, .. } => {
            format!("Demolished {}", building_kind_name(kind))
        }
    }
}

fn command_error_text(error: CommandError) -> String {
    match error {
        CommandError::PlacementRejected(reason) => reason,
        other => other.to_string(),
    }
}

fn push_road_drag_step(tiles: &mut Vec<TileCoord>, coord: TileCoord) {
    if let Some(existing_index) = tiles.iter().position(|tile| *tile == coord) {
        tiles.truncate(existing_index + 1);
    } else {
        tiles.push(coord);
    }
}

fn draw_run(run: &RunState, hovered_tile: Option<TileCoord>, preview: &Preview) {
    for island in &run.islands {
        for tile in island.tiles() {
            let is_hovered = hovered_tile == Some(tile.coord);
            draw_tile(tile.coord, tile.height, is_hovered);
        }
    }

    for road in run.roads.values() {
        draw_road(
            road.coord,
            road.height,
            run.road_kind_at(road.coord).unwrap_or(RoadKind::Sky),
        );
    }

    for building in &run.buildings {
        let mut color = building_color(building.kind);
        if !run.is_building_active(building.id) {
            color = darken(color);
        }
        draw_building(
            building.origin,
            run.tile_height(building.origin).unwrap_or(0),
            color,
        );
    }

    draw_preview(run, preview);
}

fn draw_preview(run: &RunState, preview: &Preview) {
    match preview {
        Preview::Building {
            footprint, valid, ..
        } => {
            let color = if *valid {
                Color::from_rgba(72, 220, 126, 120)
            } else {
                Color::from_rgba(232, 74, 72, 130)
            };
            for coord in footprint {
                draw_preview_tile(*coord, run.tile_height(*coord).unwrap_or(0), color);
            }
        }
        Preview::Road {
            tiles,
            new_roads,
            valid,
            ..
        } => {
            for coord in tiles {
                let height = preview_road_height(run, new_roads, *coord);
                let color = preview_road_color(run, *coord, *valid);
                draw_preview_tile(*coord, height, color);
            }
        }
        Preview::None => {}
    }
}

fn preview_road_height(run: &RunState, new_roads: &[Road], coord: TileCoord) -> i32 {
    new_roads
        .iter()
        .find(|road| road.coord == coord)
        .map(|road| road.height)
        .or_else(|| run.roads.get(&coord).map(|road| road.height))
        .or_else(|| run.tile_height(coord))
        .unwrap_or(0)
}

fn preview_road_color(run: &RunState, coord: TileCoord, valid: bool) -> Color {
    if !valid {
        return Color::from_rgba(232, 74, 72, 180);
    }

    match preview_road_kind(run, coord) {
        RoadKind::Island => Color::from_rgba(242, 213, 92, 180),
        RoadKind::Sky => Color::from_rgba(74, 158, 222, 180),
    }
}

fn preview_road_kind(run: &RunState, coord: TileCoord) -> RoadKind {
    run.road_kind_at(coord).unwrap_or_else(|| {
        if run.tile_height(coord).is_some() {
            RoadKind::Island
        } else {
            RoadKind::Sky
        }
    })
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

fn draw_road(coord: TileCoord, height: i32, kind: RoadKind) {
    let center = tile_center(coord, height) + vec3(0.0, ROAD_HEIGHT * 0.5 + 0.02, 0.0);
    let color = match kind {
        RoadKind::Island => Color::from_rgba(91, 84, 78, 255),
        RoadKind::Sky => Color::from_rgba(78, 132, 182, 255),
    };
    draw_cube(center, vec3(0.82, ROAD_HEIGHT, 0.82), None, color);
    draw_cube_wires(center, vec3(0.82, ROAD_HEIGHT, 0.82), BLACK);
}

fn draw_preview_tile(coord: TileCoord, height: i32, color: Color) {
    let center = tile_center(coord, height) + vec3(0.0, 0.08, 0.0);
    draw_cube(center, vec3(0.92, 0.08, 0.92), None, color);
    draw_cube_wires(center, vec3(0.92, 0.08, 0.92), BLACK);
}

fn draw_building(origin: TileCoord, height: i32, color: Color) {
    let center = tile_center(origin, height) + vec3(TILE_SIZE * 0.5, 0.45, TILE_SIZE * 0.5);
    draw_cube(center, vec3(1.75, 0.9, 1.75), None, color);
    draw_cube_wires(center, vec3(1.75, 0.9, 1.75), BLACK);
}

fn building_color(kind: BuildingKind) -> Color {
    match kind {
        BuildingKind::CityCore => Color::from_rgba(210, 75, 64, 255),
        BuildingKind::House => Color::from_rgba(236, 198, 110, 255),
        BuildingKind::Farm => Color::from_rgba(92, 168, 95, 255),
        BuildingKind::Workshop => Color::from_rgba(133, 129, 120, 255),
        BuildingKind::Market => Color::from_rgba(98, 113, 191, 255),
        BuildingKind::Monument => Color::from_rgba(186, 184, 196, 255),
    }
}

fn darken(color: Color) -> Color {
    Color::new(color.r * 0.42, color.g * 0.42, color.b * 0.42, color.a)
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

    let island_hit = run
        .islands
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
        .map(|(_, coord)| coord);

    island_hit.or_else(|| pick_empty_tile(ray))
}

fn pick_empty_tile(ray: Ray) -> Option<TileCoord> {
    if ray.direction.y.abs() < f32::EPSILON {
        return None;
    }

    let distance = -ray.origin.y / ray.direction.y;
    if distance < 0.0 {
        return None;
    }

    let hit = ray.origin + ray.direction * distance;
    let coord = TileCoord::new(hit.x.round() as i32, hit.z.round() as i32);
    if coord.x.abs() <= EMPTY_TILE_PICK_COORD_LIMIT && coord.z.abs() <= EMPTY_TILE_PICK_COORD_LIMIT
    {
        Some(coord)
    } else {
        None
    }
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

fn draw_hud(run: &RunState, hovered_tile: Option<TileCoord>, ui: &AppUi, preview: &Preview) {
    let mut lines = vec![
        format!("Run time: {}s", run.elapsed_seconds),
        format!("SkyCoin: {}", run.sky_coin),
        format!("SkyCoin drain: -{:.1}/s", run.sky_coin_drain_per_second),
        format!("Tax Income: +{:.1}/s", sky_coin_tax_income_per_second(run)),
        format!("Net SkyCoin: {:+.1}/s", sky_coin_net_per_second(run)),
        format!("Food: {}/{}", run.food.current, run.food.cap),
        format!("Food production: +{:.1}/s", run.food.production_per_second),
        format!(
            "Food consumption: -{:.1}/s",
            run.food.consumption_per_second
        ),
        format!("Net Food: {:+.1}/s", food_net_per_second(run)),
        format!("Citizens: {}/{}", run.citizens, run.citizen_capacity),
        format!("Tool: {}", tool_name(ui.selected_tool)),
    ];

    if let Some(line) = preview.status_line() {
        lines.push(line);
    }

    lines.push(format!("Message: {}", ui.last_message));

    if let Some(inspection) = inspect_tile(run, hovered_tile) {
        lines.push(inspection);
    }

    let panel_color = Color::from_rgba(23, 29, 33, 220);
    let panel_height = 32.0 + lines.len() as f32 * 24.0;
    draw_rectangle(16.0, 16.0, 440.0, panel_height, panel_color);

    for (index, line) in lines.iter().enumerate() {
        draw_text(line, 32.0, 48.0 + index as f32 * 24.0, 20.0, WHITE);
    }
}

fn sky_coin_tax_income_per_second(_run: &RunState) -> f64 {
    0.0
}

fn sky_coin_net_per_second(run: &RunState) -> f64 {
    sky_coin_tax_income_per_second(run) - run.sky_coin_drain_per_second
}

fn food_net_per_second(run: &RunState) -> f64 {
    run.food.production_per_second - run.food.consumption_per_second
}

fn inspect_tile(run: &RunState, hovered_tile: Option<TileCoord>) -> Option<String> {
    let coord = hovered_tile?;

    if let Some(road) = run.roads.get(&coord) {
        let kind = run.road_kind_at(coord).unwrap_or(RoadKind::Sky);
        let connection = if run.is_road_connected(coord) {
            "connected"
        } else {
            "disconnected"
        };
        return Some(format!(
            "Inspect: {} Road, {connection}, h{}, ({}, {})",
            road_kind_name(kind),
            road.height,
            coord.x,
            coord.z
        ));
    }

    if let Some(building) = run.building_at(coord) {
        let state = if run.is_building_active(building.id) {
            "active"
        } else {
            "disconnected"
        };
        return Some(format!(
            "Inspect: {}, {state}, origin ({}, {})",
            building_kind_name(building.kind),
            building.origin.x,
            building.origin.z
        ));
    }

    Some(format!("Inspect: tile ({}, {})", coord.x, coord.z))
}

fn draw_toolbar(run: &RunState, selected_tool: Tool) {
    let rect = toolbar_rect();
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::from_rgba(23, 29, 33, 235),
    );

    for (index, tool) in TOOLS.iter().enumerate() {
        let tool_rect = tool_rect(index);
        let selected = selected_tool == *tool;
        let fill = if selected {
            Color::from_rgba(72, 118, 168, 255)
        } else {
            Color::from_rgba(49, 57, 64, 255)
        };
        draw_rectangle(tool_rect.x, tool_rect.y, tool_rect.w, tool_rect.h, fill);
        draw_rectangle_lines(
            tool_rect.x,
            tool_rect.y,
            tool_rect.w,
            tool_rect.h,
            2.0,
            Color::from_rgba(218, 223, 228, 255),
        );

        let label = format!("{} {}", index + 1, tool_name(*tool));
        draw_text(&label, tool_rect.x + 10.0, tool_rect.y + 24.0, 18.0, WHITE);

        if let Tool::Building(kind) = tool {
            let cost = run.building_cost(*kind);
            let cost_color = if run.sky_coin >= cost {
                Color::from_rgba(214, 232, 188, 255)
            } else {
                Color::from_rgba(242, 132, 124, 255)
            };
            draw_text(
                &format!("{cost} SkyCoin"),
                tool_rect.x + 10.0,
                tool_rect.y + 48.0,
                16.0,
                cost_color,
            );
        }
    }
}

fn toolbar_rect() -> Rect {
    Rect::new(
        0.0,
        screen_height() - TOOLBAR_HEIGHT,
        screen_width(),
        TOOLBAR_HEIGHT,
    )
}

fn tool_rect(index: usize) -> Rect {
    let width = 148.0_f32.min((screen_width() - 32.0) / TOOLS.len() as f32 - 8.0);
    let x = 16.0 + index as f32 * (width + 8.0);
    let y = screen_height() - TOOLBAR_HEIGHT + 12.0;
    Rect::new(x, y, width, 52.0)
}

fn mouse_vec() -> Vec2 {
    let (x, y) = mouse_position();
    vec2(x, y)
}

fn tool_name(tool: Tool) -> &'static str {
    match tool {
        Tool::Road => "Road",
        Tool::Demolish => "Demolish",
        Tool::Building(kind) => building_kind_name(kind),
    }
}

fn building_kind_name(kind: BuildingKind) -> &'static str {
    match kind {
        BuildingKind::CityCore => "City Core",
        BuildingKind::House => "House",
        BuildingKind::Farm => "Farm",
        BuildingKind::Workshop => "Workshop",
        BuildingKind::Market => "Market",
        BuildingKind::Monument => "Monument",
    }
}

fn road_kind_name(kind: RoadKind) -> &'static str {
    match kind {
        RoadKind::Island => "Island",
        RoadKind::Sky => "Sky",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn road_drag_fills_manhattan_gaps_and_truncates_backtracking() {
        let mut drag = RoadDrag::start(TileCoord::new(0, 0));

        drag.push_to(TileCoord::new(3, 2));
        assert_eq!(
            drag.tiles,
            vec![
                TileCoord::new(0, 0),
                TileCoord::new(1, 0),
                TileCoord::new(2, 0),
                TileCoord::new(3, 0),
                TileCoord::new(3, 1),
                TileCoord::new(3, 2),
            ]
        );

        drag.push_to(TileCoord::new(1, 2));
        assert_eq!(
            drag.tiles,
            vec![
                TileCoord::new(0, 0),
                TileCoord::new(1, 0),
                TileCoord::new(2, 0),
                TileCoord::new(3, 0),
                TileCoord::new(3, 1),
                TileCoord::new(3, 2),
                TileCoord::new(2, 2),
                TileCoord::new(1, 2),
            ]
        );

        drag.push_to(TileCoord::new(2, 0));

        assert_eq!(
            drag.tiles,
            vec![
                TileCoord::new(0, 0),
                TileCoord::new(1, 0),
                TileCoord::new(2, 0),
            ]
        );
    }
}
