use crate::simulation::{BuildingId, BuildingKind, GameState, RunState};
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
    PlaceRoadPath {
        path: Vec<TileCoord>,
    },
    DemolishTile {
        coord: TileCoord,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    RunStarted,
    Ticked,
    BuildingPlaced { id: BuildingId },
    RoadPathPlaced { coords: Vec<TileCoord> },
    RoadDemolished { coord: TileCoord },
    BuildingDemolished { id: BuildingId, kind: BuildingKind },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    RunAlreadyStarted,
    NoRunStarted,
    PlacementRejected(String),
}

impl Display for CommandError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RunAlreadyStarted => write!(formatter, "a Run is already started"),
            Self::NoRunStarted => write!(formatter, "no Run is started"),
            Self::PlacementRejected(reason) => write!(formatter, "placement rejected: {reason}"),
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
                    .map_err(CommandError::PlacementRejected)
            }
            Command::PlaceRoadPath { path: _ } => Err(CommandError::PlacementRejected({
                self.current_run
                    .as_ref()
                    .ok_or(CommandError::NoRunStarted)?;
                "Road placement is not implemented yet".to_owned()
            })),
            Command::DemolishTile { coord: _ } => Err(CommandError::PlacementRejected({
                self.current_run
                    .as_ref()
                    .ok_or(CommandError::NoRunStarted)?;
                "Demolition is not implemented yet".to_owned()
            })),
        }
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

        assert!(matches!(result, Err(CommandError::PlacementRejected(_))));
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
    fn road_and_demolition_commands_exist_but_are_not_implemented_yet() {
        let mut game = GameState::empty();
        game.apply(Command::StartRun { seed: 7 }).unwrap();

        let road_result = game.apply(Command::PlaceRoadPath {
            path: vec![TileCoord::new(2, 0)],
        });
        let demolish_result = game.apply(Command::DemolishTile {
            coord: TileCoord::new(2, 0),
        });

        assert!(matches!(
            road_result,
            Err(CommandError::PlacementRejected(reason))
            if reason == "Road placement is not implemented yet"
        ));
        assert!(matches!(
            demolish_result,
            Err(CommandError::PlacementRejected(reason))
            if reason == "Demolition is not implemented yet"
        ));
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
