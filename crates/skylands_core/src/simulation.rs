use crate::world::{FlyingIsland, TileCoord};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BuildingId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingKind {
    CityCore,
    House,
    Farm,
    Workshop,
    Market,
    Monument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Building {
    pub id: BuildingId,
    pub kind: BuildingKind,
    pub origin: TileCoord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Road {
    pub coord: TileCoord,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    Running,
    Bankrupt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoodStock {
    pub current: i64,
    pub cap: i64,
    pub production_per_second: f64,
    pub consumption_per_second: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunState {
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
    #[serde(with = "road_map_serde")]
    pub roads: BTreeMap<TileCoord, Road>,
    pub next_building_id: u32,
    sky_coin_drain_remainder: f64,
    food_remainder: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub current_run: Option<RunState>,
}

impl GameState {
    pub fn empty() -> Self {
        Self { current_run: None }
    }
}

impl RunState {
    pub fn start(seed: u64) -> Self {
        Self {
            seed,
            status: RunStatus::Running,
            elapsed_seconds: 0,
            sky_coin: 500,
            sky_coin_drain_per_second: 1.0,
            citizens: 0,
            citizen_capacity: 5,
            food: FoodStock {
                current: 20,
                cap: 50,
                production_per_second: 0.0,
                consumption_per_second: 0.0,
            },
            islands: vec![FlyingIsland::generated(seed)],
            buildings: vec![Building {
                id: BuildingId(0),
                kind: BuildingKind::CityCore,
                origin: TileCoord::new(0, 0),
            }],
            roads: BTreeMap::new(),
            next_building_id: 1,
            sky_coin_drain_remainder: 0.0,
            food_remainder: 0.0,
        }
    }

    pub fn road_kind_at(&self, coord: TileCoord) -> Option<RoadKind> {
        self.roads.get(&coord).map(|_| {
            if self.tile_height(coord).is_some() {
                RoadKind::Island
            } else {
                RoadKind::Sky
            }
        })
    }

    pub fn tick(&mut self) {
        if self.status != RunStatus::Running {
            return;
        }

        self.elapsed_seconds += 1;
        self.apply_sky_coin_drain();
        self.apply_food_delta();

        if self.sky_coin <= 0 {
            self.status = RunStatus::Bankrupt;
        }
    }

    pub fn tile_height(&self, coord: TileCoord) -> Option<i32> {
        self.islands
            .iter()
            .find_map(|island| island.tile(coord).map(|tile| tile.height))
    }

    pub fn is_occupied(&self, coord: TileCoord) -> bool {
        self.buildings
            .iter()
            .flat_map(|building| building_footprint(building.origin))
            .any(|occupied| occupied == coord)
    }

    pub fn place_building(
        &mut self,
        kind: BuildingKind,
        origin: TileCoord,
    ) -> Result<BuildingId, String> {
        if kind == BuildingKind::CityCore {
            return Err("City Core is fixed and unique".to_owned());
        }

        let footprint = building_footprint(origin);
        let Some(first_height) = self.tile_height(footprint[0]) else {
            return Err("Building footprint must be on a Flying Island".to_owned());
        };

        for coord in footprint {
            if self.tile_height(coord) != Some(first_height) {
                return Err("Building footprint must be buildable and at one height".to_owned());
            }

            if self.is_occupied(coord) {
                return Err("Building footprint is occupied".to_owned());
            }
        }

        let id = BuildingId(self.next_building_id);
        self.next_building_id += 1;
        self.buildings.push(Building { id, kind, origin });
        Ok(id)
    }

    fn apply_sky_coin_drain(&mut self) {
        let total = self.sky_coin_drain_per_second + self.sky_coin_drain_remainder;
        let whole = total.floor() as i64;
        self.sky_coin_drain_remainder = total - whole as f64;
        self.sky_coin -= whole;
    }

    fn apply_food_delta(&mut self) {
        let net = self.food.production_per_second - self.food.consumption_per_second
            + self.food_remainder;
        let whole = net.trunc() as i64;
        self.food_remainder = net - whole as f64;
        self.food.current = (self.food.current + whole).clamp(0, self.food.cap);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadKind {
    Island,
    Sky,
}

pub fn building_footprint(origin: TileCoord) -> [TileCoord; 4] {
    [
        origin,
        TileCoord::new(origin.x + 1, origin.z),
        TileCoord::new(origin.x, origin.z + 1),
        TileCoord::new(origin.x + 1, origin.z + 1),
    ]
}

mod road_map_serde {
    use super::Road;
    use crate::world::TileCoord;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S>(roads: &BTreeMap<TileCoord, Road>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        roads.values().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<TileCoord, Road>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let roads = Vec::<Road>::deserialize(deserializer)?;
        Ok(roads.into_iter().map(|road| (road.coord, road)).collect())
    }
}

#[cfg(test)]
mod road_tests {
    use super::*;

    #[test]
    fn run_state_stores_roads_in_deterministic_tile_order() {
        let mut run = RunState::start(7);

        run.roads.insert(
            TileCoord::new(2, 0),
            Road {
                coord: TileCoord::new(2, 0),
                height: 1,
            },
        );
        run.roads.insert(
            TileCoord::new(-1, 4),
            Road {
                coord: TileCoord::new(-1, 4),
                height: 0,
            },
        );
        run.roads.insert(
            TileCoord::new(2, -1),
            Road {
                coord: TileCoord::new(2, -1),
                height: 1,
            },
        );

        let ordered_coords = run.roads.keys().copied().collect::<Vec<_>>();

        assert_eq!(
            ordered_coords,
            vec![
                TileCoord::new(-1, 4),
                TileCoord::new(2, -1),
                TileCoord::new(2, 0),
            ]
        );
    }

    #[test]
    fn road_kind_is_derived_from_the_world_tile() {
        let mut run = RunState::start(7);
        let island_coord = TileCoord::new(0, 2);
        let empty_coord = TileCoord::new(20, 20);

        let island_height = run
            .tile_height(island_coord)
            .expect("test coordinate should be on the generated island");
        run.roads.insert(
            island_coord,
            Road {
                coord: island_coord,
                height: island_height,
            },
        );
        run.roads.insert(
            empty_coord,
            Road {
                coord: empty_coord,
                height: island_height,
            },
        );

        assert_eq!(run.road_kind_at(island_coord), Some(RoadKind::Island));
        assert_eq!(run.road_kind_at(empty_coord), Some(RoadKind::Sky));
    }
}
