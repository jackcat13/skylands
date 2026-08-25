use crate::simulation::{Building, FoodStock, Road, RunState, RunStatus};
use crate::world::FlyingIsland;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BonusLevelsSaveV1 {
    pub stonks: u32,
    pub long_suffering: u32,
    pub drug_traffic: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MilestoneReachCountsSaveV1 {
    pub first_house: u32,
    pub ten_citizens: u32,
    pub first_market: u32,
    pub first_monument: u32,
    pub one_hundred_performance: u32,
    pub two_minute_survival: u32,
    pub ten_minute_survival: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveStateV1 {
    pub version: u32,
    pub divine_coin: i64,
    pub bonus_levels: BonusLevelsSaveV1,
    pub milestone_reach_counts: MilestoneReachCountsSaveV1,
    pub current_run: Option<RunSaveV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunSaveV1 {
    pub seed: u64,
    pub status: RunStatus,
    pub elapsed_seconds: u64,
    pub sky_coin: i64,
    pub sky_coin_drain_per_second: f64,
    pub citizens: u32,
    pub citizen_capacity: u32,
    pub food: FoodStock,
    pub islands: Vec<FlyingIsland>,
    pub buildings: Vec<Building>,
    pub roads: Vec<Road>,
    pub next_building_id: u32,
}

impl SaveStateV1 {
    pub fn empty() -> Self {
        Self {
            version: 1,
            divine_coin: 0,
            bonus_levels: BonusLevelsSaveV1::default(),
            milestone_reach_counts: MilestoneReachCountsSaveV1::default(),
            current_run: None,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl From<&RunState> for RunSaveV1 {
    fn from(run: &RunState) -> Self {
        Self {
            seed: run.seed,
            status: run.status,
            elapsed_seconds: run.elapsed_seconds,
            sky_coin: run.sky_coin,
            sky_coin_drain_per_second: run.sky_coin_drain_per_second,
            citizens: run.citizens,
            citizen_capacity: run.citizen_capacity,
            food: run.food.clone(),
            islands: run.islands.clone(),
            buildings: run.buildings.clone(),
            roads: run.roads.values().copied().collect(),
            next_building_id: run.next_building_id,
        }
    }
}

pub type SaveState = SaveStateV1;
pub type RunSave = RunSaveV1;
pub type SaveEnvelopeV1 = BTreeMap<String, serde_json::Value>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::{Road, RunState};
    use crate::world::TileCoord;

    #[test]
    fn save_state_round_trips_as_json() {
        let save = SaveStateV1 {
            current_run: Some(RunSaveV1::from(&RunState::start(10))),
            ..SaveStateV1::empty()
        };

        let json = save.to_json().unwrap();
        let loaded = SaveStateV1::from_json(&json).unwrap();

        assert_eq!(loaded, save);
    }

    #[test]
    fn run_save_round_trips_roads_as_json() {
        let mut run = RunState::start(10);
        run.roads.insert(
            TileCoord::new(-2, 1),
            Road {
                coord: TileCoord::new(-2, 1),
                height: 0,
            },
        );
        run.roads.insert(
            TileCoord::new(3, -1),
            Road {
                coord: TileCoord::new(3, -1),
                height: 2,
            },
        );

        let save = SaveStateV1 {
            current_run: Some(RunSaveV1::from(&run)),
            ..SaveStateV1::empty()
        };

        let json = save.to_json().unwrap();
        let loaded = SaveStateV1::from_json(&json).unwrap();
        let roads = loaded.current_run.unwrap().roads;

        assert_eq!(
            roads,
            vec![
                Road {
                    coord: TileCoord::new(-2, 1),
                    height: 0,
                },
                Road {
                    coord: TileCoord::new(3, -1),
                    height: 2,
                },
            ]
        );
    }
}
