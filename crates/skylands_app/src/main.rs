mod city_render;
mod city_ui;
mod city_view;

use city_render::{draw_hud, draw_run, draw_toolbar, mouse_vec, tool_rect, toolbar_rect};
use city_ui::{AppUi, Preview, TOOLS, Tool};
use city_view::{CityCamera, pick_tile};
use macroquad::prelude::*;
use skylands_core::{
    Command, CommandError, CommandOutcome, GameState, RunStatus, SaveStateV1, TileCoord,
};

const SAVE_FILE: &str = "skylands-save.json";
const MAX_CATCH_UP_TICKS: u32 = 3;

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = load_or_start_game();

    let mut camera = CityCamera::default();
    let mut ui = AppUi::default();
    let mut tick_accumulator = 0.0_f32;

    loop {
        clear_background(Color::from_rgba(139, 198, 218, 255));

        let frame_time = get_frame_time();
        if game
            .current_run
            .as_ref()
            .is_some_and(|run| run.status == RunStatus::Running)
        {
            tick_accumulator += frame_time;
        }
        let mut catch_up_ticks = 0;

        while tick_accumulator >= 1.0 && catch_up_ticks < MAX_CATCH_UP_TICKS {
            game.apply(Command::Tick).expect("Run should keep ticking");
            tick_accumulator -= 1.0;
            catch_up_ticks += 1;
        }

        if catch_up_ticks == MAX_CATCH_UP_TICKS {
            tick_accumulator = 0.0;
        }
        if catch_up_ticks > 0 {
            persist_game(&game);
        }

        ui.update(frame_time);
        camera.update();
        handle_global_hotkeys(&mut game, &mut ui);

        let hovered_tile = {
            let run = game.current_run.as_ref().expect("Run should exist");
            pick_tile(run, &camera)
        };
        let pointer_over_toolbar = toolbar_rect().contains(mouse_vec());
        let hovered_world_tile = if pointer_over_toolbar {
            None
        } else {
            hovered_tile
        };

        handle_toolbar_mouse(&mut ui);
        let run_status = game.current_run.as_ref().expect("Run should exist").status;
        handle_world_input(
            &mut game,
            &mut ui,
            hovered_world_tile,
            pointer_over_toolbar,
            run_status,
        );

        let run = game.current_run.as_ref().expect("Run should exist");
        let preview = Preview::from_state(run, &ui, hovered_world_tile);

        set_camera(&camera.to_macroquad());
        draw_run(run, &camera, hovered_world_tile, &preview);
        set_default_camera();

        draw_hud(run, hovered_world_tile, &ui, &preview);
        draw_toolbar(run, ui.selected_tool());

        next_frame().await;
    }
}

fn load_or_start_game() -> GameState {
    if let Ok(json) = std::fs::read_to_string(SAVE_FILE)
        && let Ok(save) = SaveStateV1::from_json(&json)
    {
        let game = GameState::from(save);
        if game.current_run.is_some() {
            return game;
        }
    }

    let mut game = GameState::empty();
    game.apply(Command::StartRun { seed: 1 })
        .expect("initial Run should start");
    persist_game(&game);
    game
}

fn persist_game(game: &GameState) {
    if cfg!(test) {
        return;
    }

    let save = SaveStateV1::from(game);

    if let Ok(json) = save.to_json() {
        let _ = std::fs::write(SAVE_FILE, json);
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

fn handle_global_hotkeys(game: &mut GameState, ui: &mut AppUi) {
    if is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Space) {
        apply_command_to_ui(game, ui, Command::TogglePause, |_| {});
    }

    for (index, tool) in TOOLS.iter().enumerate() {
        if is_key_pressed(number_key(index)) {
            ui.select_tool(*tool);
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
            ui.select_tool(*tool);
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

    match ui.selected_tool() {
        Tool::Building(kind) => {
            if !pointer_over_toolbar
                && is_mouse_button_pressed(MouseButton::Left)
                && let Some(origin) = hovered_tile
            {
                apply_command_to_ui(game, ui, Command::PlaceBuilding { kind, origin }, |_| {});
            }
        }
        Tool::Road => handle_road_input(game, ui, hovered_tile, pointer_over_toolbar),
        Tool::Demolish => {
            if !pointer_over_toolbar
                && is_mouse_button_pressed(MouseButton::Left)
                && let Some(coord) = hovered_tile
            {
                apply_command_to_ui(game, ui, Command::DemolishTile { coord }, |_| {});
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
        ui.start_road_drag(coord);
    }

    if !pointer_over_toolbar
        && is_mouse_button_down(MouseButton::Left)
        && let Some(coord) = hovered_tile
    {
        ui.push_road_drag_to(coord);
    }

    if is_mouse_button_released(MouseButton::Left)
        && let Some(coords) = ui.finish_road_drag()
    {
        apply_command_to_ui(game, ui, Command::PlaceRoads { coords }, |_| {});
    }
}

fn apply_command_to_ui(
    game: &mut GameState,
    ui: &mut AppUi,
    command: Command,
    on_success: impl FnOnce(CommandOutcome),
) {
    match game.apply(command) {
        Ok(outcome) => {
            ui.clear_command_banner();
            on_success(outcome);
            persist_game(game);
        }
        Err(error) => ui.show_command_error(command_error_text(error)),
    }
}

fn command_error_text(error: CommandError) -> String {
    match error {
        CommandError::InvalidPlacement(reason) => reason.message().to_owned(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_command_clears_existing_command_banner() {
        let mut game = GameState::empty();
        game.apply(Command::StartRun { seed: 7 }).unwrap();
        let mut ui = AppUi::default();
        ui.show_command_error("Previous failure".to_owned());

        apply_command_to_ui(
            &mut game,
            &mut ui,
            Command::PlaceRoads {
                coords: vec![TileCoord::new(0, -1)],
            },
            |_| {},
        );

        assert!(ui.command_banner().is_none());
    }
}
