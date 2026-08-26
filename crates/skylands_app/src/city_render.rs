use crate::city_ui::{AppUi, CommandBanner, Preview, TOOLS, Tool};
use crate::city_view::{
    CityCamera, TILE_HEIGHT, TILE_SIZE, island_is_near_camera, terrain_wires_visible, tile_center,
};
use macroquad::prelude::*;
use skylands_core::{BuildingKind, CityConnectivity, Road, RoadKind, RunState, TileCoord};

const ROAD_HEIGHT: f32 = 0.06;
const TOOLBAR_HEIGHT: f32 = 76.0;
const TOP_BAR_HEIGHT: f32 = 40.0;
const CURSOR_TOOLTIP_MAX_LINES: usize = 3;

pub fn draw_run(
    run: &RunState,
    camera: &CityCamera,
    hovered_tile: Option<TileCoord>,
    preview: &Preview,
) {
    let connectivity = run.city_connectivity();

    for island in &run.islands {
        if !island_is_near_camera(island, camera) {
            continue;
        }

        for tile in island.tiles() {
            let is_hovered = hovered_tile == Some(tile.coord);
            draw_tile(
                tile.coord,
                tile.height,
                is_hovered,
                terrain_wires_visible(camera),
            );
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
        if !connectivity.is_building_active(building) {
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
        Preview::Building(quote) => {
            let color = if quote.valid {
                Color::from_rgba(72, 220, 126, 120)
            } else {
                Color::from_rgba(232, 74, 72, 130)
            };
            for coord in &quote.footprint {
                draw_preview_tile(*coord, run.tile_height(*coord).unwrap_or(0), color);
            }
        }
        Preview::Road(quote) => {
            for coord in &quote.tiles {
                let height = preview_road_height(run, &quote.new_roads, *coord);
                let color = preview_road_color(run, *coord, quote.valid);
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

fn draw_tile(coord: TileCoord, height: i32, is_hovered: bool, draw_wires: bool) {
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
    if draw_wires {
        draw_cube_wires(
            center - vec3(0.0, TILE_HEIGHT * 0.5, 0.0),
            vec3(TILE_SIZE, TILE_HEIGHT, TILE_SIZE),
            Color::from_rgba(57, 92, 65, 255),
        );
    }
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

pub fn draw_hud(run: &RunState, hovered_tile: Option<TileCoord>, ui: &AppUi, preview: &Preview) {
    let connectivity = run.city_connectivity();

    draw_top_bar(run);
    draw_command_banner(ui.command_banner());
    draw_cursor_tooltip(cursor_tooltip_lines(
        preview.quote_line(),
        inspect_tile(run, &connectivity, hovered_tile),
    ));
}

fn draw_top_bar(run: &RunState) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        TOP_BAR_HEIGHT,
        Color::from_rgba(23, 29, 33, 232),
    );

    let y = 26.0;
    let resource_color = Color::from_rgba(238, 242, 244, 255);
    let sky_coin = format!(
        "SkyCoin {} {:+.1}/s",
        run.sky_coin,
        run.sky_coin_net_per_second()
    );
    let food = format!(
        "Food {}/{} {:+.1}/s",
        run.food.current,
        run.food.cap,
        run.food_net_per_second()
    );
    let citizens = format!("Citizens {}/{}", run.citizens, run.citizen_capacity);

    draw_text(&sky_coin, 18.0, y, 20.0, resource_color);
    draw_text(&food, 210.0, y, 20.0, resource_color);
    draw_text(&citizens, 390.0, y, 20.0, resource_color);

    let time = format_run_time(run.elapsed_seconds);
    let status = match run.status {
        skylands_core::RunStatus::Running => time,
        skylands_core::RunStatus::Paused => format!("{time} Paused"),
        skylands_core::RunStatus::Bankrupt => format!("{time} Bankrupt"),
    };
    let time_size = measure_text(&status, None, 20, 1.0);
    draw_text(
        &status,
        screen_width() - time_size.width - 24.0,
        y,
        20.0,
        resource_color,
    );
}

fn draw_command_banner(banner: Option<&CommandBanner>) {
    let Some(banner) = banner else {
        return;
    };

    let font_size = 20;
    let padding = vec2(18.0, 9.0);
    let text_size = measure_text(&banner.text, None, font_size, 1.0);
    let width = text_size.width + padding.x * 2.0;
    let height = TOP_BAR_HEIGHT - 4.0;
    let x = (screen_width() - width) * 0.5;
    let y = TOP_BAR_HEIGHT + 10.0;

    draw_rectangle(x, y, width, height, Color::from_rgba(112, 33, 36, 232));
    draw_rectangle_lines(
        x,
        y,
        width,
        height,
        2.0,
        Color::from_rgba(246, 157, 148, 255),
    );
    draw_text(
        &banner.text,
        x + padding.x,
        y + 24.0,
        font_size as f32,
        WHITE,
    );
}

fn cursor_tooltip_lines(quote: Option<String>, inspection: Option<String>) -> Vec<String> {
    quote
        .into_iter()
        .chain(inspection)
        .take(CURSOR_TOOLTIP_MAX_LINES)
        .collect()
}

fn draw_cursor_tooltip(lines: Vec<String>) {
    if lines.is_empty() {
        return;
    }

    let font_size = 18;
    let line_height = 22.0;
    let padding = vec2(12.0, 9.0);
    let max_width = lines
        .iter()
        .map(|line| measure_text(line, None, font_size, 1.0).width)
        .fold(0.0, f32::max);
    let width = max_width + padding.x * 2.0;
    let height = padding.y * 2.0 + line_height * lines.len() as f32;
    let mouse = mouse_vec();
    let position = clamp_tooltip_position(mouse + vec2(18.0, 18.0), vec2(width, height));

    draw_rectangle(
        position.x,
        position.y,
        width,
        height,
        Color::from_rgba(23, 29, 33, 226),
    );
    draw_rectangle_lines(
        position.x,
        position.y,
        width,
        height,
        1.0,
        Color::from_rgba(218, 223, 228, 180),
    );

    for (index, line) in lines.iter().enumerate() {
        draw_text(
            line,
            position.x + padding.x,
            position.y + padding.y + 17.0 + index as f32 * line_height,
            font_size as f32,
            WHITE,
        );
    }
}

fn clamp_tooltip_position(position: Vec2, size: Vec2) -> Vec2 {
    let margin = 8.0;
    vec2(
        position.x.clamp(margin, screen_width() - size.x - margin),
        position
            .y
            .clamp(TOP_BAR_HEIGHT + margin, screen_height() - size.y - margin),
    )
}

fn format_run_time(seconds: u64) -> String {
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("Run {minutes:02}:{seconds:02}")
}

fn inspect_tile(
    run: &RunState,
    connectivity: &CityConnectivity,
    hovered_tile: Option<TileCoord>,
) -> Option<String> {
    let coord = hovered_tile?;

    if let Some(road) = run.roads.get(&coord) {
        let kind = run.road_kind_at(coord).unwrap_or(RoadKind::Sky);
        let connection = if connectivity.is_road_connected(coord) {
            "connected"
        } else {
            "disconnected"
        };
        return Some(format!(
            "Inspect: {} Road, {connection}, h{}, ({}, {})",
            kind.name(),
            road.height,
            coord.x,
            coord.z
        ));
    }

    if let Some(building) = run.building_at(coord) {
        let state = if connectivity.is_building_active(building) {
            "active"
        } else {
            "disconnected"
        };
        return Some(format!(
            "Inspect: {}, {state}, origin ({}, {})",
            building.kind.name(),
            building.origin.x,
            building.origin.z
        ));
    }

    Some(format!("Inspect: tile ({}, {})", coord.x, coord.z))
}

pub fn draw_toolbar(run: &RunState, selected_tool: Tool) {
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

pub fn toolbar_rect() -> Rect {
    Rect::new(
        0.0,
        screen_height() - TOOLBAR_HEIGHT,
        screen_width(),
        TOOLBAR_HEIGHT,
    )
}

pub fn tool_rect(index: usize) -> Rect {
    let width = 148.0_f32.min((screen_width() - 32.0) / TOOLS.len() as f32 - 8.0);
    let x = 16.0 + index as f32 * (width + 8.0);
    let y = screen_height() - TOOLBAR_HEIGHT + 12.0;
    Rect::new(x, y, width, 52.0)
}

pub fn mouse_vec() -> Vec2 {
    let (x, y) = mouse_position();
    vec2(x, y)
}

fn tool_name(tool: Tool) -> &'static str {
    match tool {
        Tool::Road => "Road",
        Tool::Demolish => "Demolish",
        Tool::Building(kind) => kind.name(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_tooltip_prioritizes_quote_before_inspection() {
        let lines = cursor_tooltip_lines(
            Some("Road: 2 SkyCoin, valid".to_owned()),
            Some("Inspect: tile (0, 0)".to_owned()),
        );

        assert_eq!(
            lines,
            vec![
                "Road: 2 SkyCoin, valid".to_owned(),
                "Inspect: tile (0, 0)".to_owned(),
            ]
        );
    }
}
