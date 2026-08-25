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
    pub fn apply(&mut self, command: Command) -> Result<Option<BuildingId>, CommandError> {
        match command {
            Command::StartRun { seed } => {
                if self.current_run.is_some() {
                    return Err(CommandError::RunAlreadyStarted);
                }

                self.current_run = Some(RunState::start(seed));
                Ok(None)
            }
            Command::Tick => {
                let run = self
                    .current_run
                    .as_mut()
                    .ok_or(CommandError::NoRunStarted)?;
                run.tick();
                Ok(None)
            }
            Command::PlaceBuilding { kind, origin } => {
                let run = self
                    .current_run
                    .as_mut()
                    .ok_or(CommandError::NoRunStarted)?;
                run.place_building(kind, origin)
                    .map(Some)
                    .map_err(CommandError::PlacementRejected)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_run_creates_a_city_core() {
        let mut game = GameState::empty();

        game.apply(Command::StartRun { seed: 7 }).unwrap();

        let run = game.current_run.unwrap();
        assert_eq!(run.buildings.len(), 1);
        assert_eq!(run.buildings[0].kind, BuildingKind::CityCore);
    }

    #[test]
    fn tick_advances_run_and_drains_sky_coin() {
        let mut game = GameState::empty();
        game.apply(Command::StartRun { seed: 7 }).unwrap();

        game.apply(Command::Tick).unwrap();

        let run = game.current_run.unwrap();
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
}
