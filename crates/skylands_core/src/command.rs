use crate::simulation::{
    BuildingId, BuildingKind, DemolitionOutcome, GameState, InvalidPlacement, RunState, RunStatus,
};
use crate::world::TileCoord;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    StartRun {
        seed: u64,
    },
    Tick,
    PlaceBuilding {
        kind: BuildingKind,
        origin: TileCoord,
    },
    PlaceRoads {
        coords: Vec<TileCoord>,
    },
    DemolishTile {
        coord: TileCoord,
    },
    TogglePause,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    RunStarted,
    Ticked,
    BuildingPlaced { id: BuildingId },
    RoadsPlaced { coords: Vec<TileCoord> },
    RoadDemolished { coord: TileCoord },
    BuildingDemolished { id: BuildingId, kind: BuildingKind },
    RunPaused,
    RunResumed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    RunAlreadyStarted,
    NoRunStarted,
    InvalidPlacement(InvalidPlacement),
}

impl Display for CommandError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RunAlreadyStarted => write!(formatter, "a Run is already started"),
            Self::NoRunStarted => write!(formatter, "no Run is started"),
            Self::InvalidPlacement(reason) => {
                write!(formatter, "invalid placement: {}", reason.message())
            }
        }
    }
}

impl Error for CommandError {}

impl GameState {
    pub fn apply(&mut self, command: Command) -> Result<CommandOutcome, CommandError> {
        match command {
            Command::StartRun { seed } => {
                if self.current_run.is_some() {
                    return Err(CommandError::RunAlreadyStarted);
                }

                self.current_run = Some(RunState::start(seed));
                Ok(CommandOutcome::RunStarted)
            }
            Command::Tick => {
                let run = self
                    .current_run
                    .as_mut()
                    .ok_or(CommandError::NoRunStarted)?;
                run.tick();
                Ok(CommandOutcome::Ticked)
            }
            Command::PlaceBuilding { kind, origin } => {
                let run = self
                    .current_run
                    .as_mut()
                    .ok_or(CommandError::NoRunStarted)?;
                run.place_building(kind, origin)
                    .map(|id| CommandOutcome::BuildingPlaced { id })
                    .map_err(CommandError::InvalidPlacement)
            }
            Command::PlaceRoads { coords } => {
                let run = self
                    .current_run
                    .as_mut()
                    .ok_or(CommandError::NoRunStarted)?;
                run.place_roads(&coords)
                    .map(|coords| CommandOutcome::RoadsPlaced { coords })
                    .map_err(CommandError::InvalidPlacement)
            }
            Command::DemolishTile { coord } => {
                let run = self
                    .current_run
                    .as_mut()
                    .ok_or(CommandError::NoRunStarted)?;
                run.demolish_tile(coord)
                    .map(command_outcome_from_demolition)
                    .map_err(CommandError::InvalidPlacement)
            }
            Command::TogglePause => {
                let run = self
                    .current_run
                    .as_mut()
                    .ok_or(CommandError::NoRunStarted)?;
                run.toggle_pause();
                Ok(match run.status {
                    RunStatus::Paused => CommandOutcome::RunPaused,
                    _ => CommandOutcome::RunResumed,
                })
            }
        }
    }
}

fn command_outcome_from_demolition(outcome: DemolitionOutcome) -> CommandOutcome {
    match outcome {
        DemolitionOutcome::Road { coord } => CommandOutcome::RoadDemolished { coord },
        DemolitionOutcome::Building { id, kind } => CommandOutcome::BuildingDemolished { id, kind },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_run_creates_a_city_core() {
        let mut game = GameState::empty();

        let outcome = game.apply(Command::StartRun { seed: 7 }).unwrap();

        let run = game.current_run.unwrap();
        assert_eq!(outcome, CommandOutcome::RunStarted);
        assert_eq!(run.buildings.len(), 1);
        assert_eq!(run.buildings[0].kind, BuildingKind::CityCore);
    }

    #[test]
    fn tick_advances_run_and_drains_sky_coin() {
        let mut game = GameState::empty();
        game.apply(Command::StartRun { seed: 7 }).unwrap();

        let outcome = game.apply(Command::Tick).unwrap();

        let run = game.current_run.unwrap();
        assert_eq!(outcome, CommandOutcome::Ticked);
        assert_eq!(run.elapsed_seconds, 1);
        assert_eq!(run.sky_coin, 499);
    }

    #[test]
    fn place_building_rejects_occupied_footprints() {
        let mut game = GameState::empty();
        game.apply(Command::StartRun { seed: 7 }).unwrap();

        let result = game.apply(Command::PlaceBuilding {
            kind: BuildingKind::House,
            origin: TileCoord::new(0, 0),
        });

        assert_eq!(
            result,
            Err(CommandError::InvalidPlacement(
                InvalidPlacement::BuildingFootprintIsOccupied,
            ))
        );
    }

    #[test]
    fn place_building_returns_a_command_outcome() {
        let mut game = GameState::empty();
        game.apply(Command::StartRun { seed: 7 }).unwrap();
        let origin = valid_unoccupied_building_origin(game.current_run.as_ref().unwrap());

        let outcome = game
            .apply(Command::PlaceBuilding {
                kind: BuildingKind::House,
                origin,
            })
            .unwrap();

        assert_eq!(
            outcome,
            CommandOutcome::BuildingPlaced { id: BuildingId(1) }
        );
    }

    #[test]
    fn demolish_tile_returns_a_command_outcome() {
        let mut game = GameState::empty();
        game.apply(Command::StartRun { seed: 7 }).unwrap();
        let origin = valid_unoccupied_building_origin(game.current_run.as_ref().unwrap());
        let placed = game
            .apply(Command::PlaceBuilding {
                kind: BuildingKind::House,
                origin,
            })
            .unwrap();
        assert_eq!(placed, CommandOutcome::BuildingPlaced { id: BuildingId(1) });

        let demolish_result = game.apply(Command::DemolishTile { coord: origin }).unwrap();

        assert_eq!(
            demolish_result,
            CommandOutcome::BuildingDemolished {
                id: BuildingId(1),
                kind: BuildingKind::House,
            }
        );
    }

    #[test]
    fn place_roads_returns_a_command_outcome() {
        let mut game = GameState::empty();
        game.apply(Command::StartRun { seed: 7 }).unwrap();

        let outcome = game
            .apply(Command::PlaceRoads {
                coords: vec![TileCoord::new(0, -1)],
            })
            .unwrap();

        assert_eq!(
            outcome,
            CommandOutcome::RoadsPlaced {
                coords: vec![TileCoord::new(0, -1)]
            }
        );
    }

    #[test]
    fn toggle_pause_stops_and_resumes_tick_effects() {
        let mut game = GameState::empty();
        game.apply(Command::StartRun { seed: 7 }).unwrap();

        assert_eq!(
            game.apply(Command::TogglePause),
            Ok(CommandOutcome::RunPaused)
        );
        assert_eq!(game.apply(Command::Tick), Ok(CommandOutcome::Ticked));
        let paused_run = game.current_run.as_ref().unwrap();
        assert_eq!(paused_run.elapsed_seconds, 0);
        assert_eq!(
            game.apply(Command::PlaceRoads {
                coords: vec![TileCoord::new(0, -1)],
            }),
            Err(CommandError::InvalidPlacement(
                InvalidPlacement::RunIsNotEditable,
            ))
        );

        assert_eq!(
            game.apply(Command::TogglePause),
            Ok(CommandOutcome::RunResumed)
        );
        assert_eq!(game.apply(Command::Tick), Ok(CommandOutcome::Ticked));
        assert_eq!(game.current_run.as_ref().unwrap().elapsed_seconds, 1);
    }

    fn valid_unoccupied_building_origin(run: &RunState) -> TileCoord {
        run.islands
            .iter()
            .flat_map(|island| island.tiles())
            .map(|tile| tile.coord)
            .find(|origin| {
                let footprint = crate::simulation::building_footprint(*origin);
                let Some(first_height) = run.tile_height(footprint[0]) else {
                    return false;
                };

                footprint.iter().all(|coord| {
                    run.tile_height(*coord) == Some(first_height) && !run.is_occupied(*coord)
                })
            })
            .expect("generated island should have at least one buildable 2x2 footprint")
    }
}
