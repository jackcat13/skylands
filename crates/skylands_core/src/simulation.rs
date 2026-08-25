use crate::world::{FlyingIsland, TileCoord};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const ISLAND_ROAD_COST: i64 = 2;
pub const SKY_ROAD_COST: i64 = 8;
const BUILDING_COST_MULTIPLIER: f64 = 1.15;
const CITY_CORE_CITIZEN_CAPACITY: u32 = 5;
const HOUSE_CITIZEN_CAPACITY: u32 = 6;
const FARM_FOOD_PRODUCTION_PER_SECOND: f64 = 2.0;
const FOOD_CONSUMPTION_PER_CITIZEN: f64 = 0.1;
const CITIZEN_ARRIVAL_PER_SECOND: f64 = 1.0;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildingQuote {
    pub valid: bool,
    pub cost: i64,
    pub footprint: [TileCoord; 4],
    pub invalid_reason: Option<InvalidPlacement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoadPlacementQuote {
    pub valid: bool,
    pub total_cost: i64,
    pub tiles: Vec<TileCoord>,
    pub new_roads: Vec<Road>,
    pub invalid_reason: Option<InvalidPlacement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidPlacement {
    CityCoreIsFixed,
    NotEnoughSkyCoin,
    BuildingFootprintMustBeOnFlyingIsland,
    BuildingFootprintMustBeBuildableAndOneHeight,
    BuildingFootprintIsOccupied,
    BuildingFootprintContainsRoad,
    RoadPlacementIsEmpty,
    RoadPlacementMustBeOrthogonallyContinuous,
    RoadHeightDifferenceTooSteep,
    RoadCannotOverlapBuilding,
    RunIsNotEditable,
    NothingToDemolish,
    CityCoreCannotBeDemolished,
}

impl InvalidPlacement {
    pub fn message(self) -> &'static str {
        match self {
            Self::CityCoreIsFixed => "City Core is fixed and unique",
            Self::NotEnoughSkyCoin => "Not enough SkyCoin",
            Self::BuildingFootprintMustBeOnFlyingIsland => {
                "Building footprint must be on a Flying Island"
            }
            Self::BuildingFootprintMustBeBuildableAndOneHeight => {
                "Building footprint must be buildable and at one height"
            }
            Self::BuildingFootprintIsOccupied => "Building footprint is occupied",
            Self::BuildingFootprintContainsRoad => "Building footprint contains a Road",
            Self::RoadPlacementIsEmpty => "Road placement is empty",
            Self::RoadPlacementMustBeOrthogonallyContinuous => {
                "Road placement must be orthogonally continuous"
            }
            Self::RoadHeightDifferenceTooSteep => "Road height difference is too steep",
            Self::RoadCannotOverlapBuilding => "Road cannot overlap a Building",
            Self::RunIsNotEditable => "Run is not editable",
            Self::NothingToDemolish => "Nothing to demolish",
            Self::CityCoreCannotBeDemolished => "City Core cannot be demolished",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemolitionOutcome {
    Road { coord: TileCoord },
    Building { id: BuildingId, kind: BuildingKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    Running,
    Paused,
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
    #[serde(skip)]
    tile_heights: BTreeMap<TileCoord, i32>,
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
        let islands = FlyingIsland::generated_many(seed);
        let tile_heights = tile_height_index(&islands);

        Self {
            seed,
            status: RunStatus::Running,
            elapsed_seconds: 0,
            sky_coin: 500,
            sky_coin_drain_per_second: 1.0,
            citizens: 0,
            citizen_capacity: CITY_CORE_CITIZEN_CAPACITY,
            food: FoodStock {
                current: 20,
                cap: 50,
                production_per_second: 0.0,
                consumption_per_second: 0.0,
            },
            islands,
            tile_heights,
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

    pub(crate) fn restored(
        seed: u64,
        status: RunStatus,
        elapsed_seconds: u64,
        sky_coin: i64,
        sky_coin_drain_per_second: f64,
        citizens: u32,
        citizen_capacity: u32,
        food: FoodStock,
        islands: Vec<FlyingIsland>,
        buildings: Vec<Building>,
        roads: BTreeMap<TileCoord, Road>,
        next_building_id: u32,
    ) -> Self {
        let tile_heights = tile_height_index(&islands);

        Self {
            seed,
            status,
            elapsed_seconds,
            sky_coin,
            sky_coin_drain_per_second,
            citizens,
            citizen_capacity,
            food,
            islands,
            tile_heights,
            buildings,
            roads,
            next_building_id,
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
        self.recalculate_city_outputs();
        self.apply_sky_coin_drain();
        self.apply_citizen_arrivals();
        self.apply_food_delta();

        if self.sky_coin <= 0 {
            self.status = RunStatus::Bankrupt;
        }
    }

    pub fn toggle_pause(&mut self) {
        self.status = match self.status {
            RunStatus::Running => RunStatus::Paused,
            RunStatus::Paused => RunStatus::Running,
            RunStatus::Bankrupt => RunStatus::Bankrupt,
        };
    }

    pub fn tile_height(&self, coord: TileCoord) -> Option<i32> {
        self.tile_heights.get(&coord).copied()
    }

    pub fn is_occupied(&self, coord: TileCoord) -> bool {
        self.building_at(coord).is_some()
    }

    pub fn building_at(&self, coord: TileCoord) -> Option<&Building> {
        self.buildings
            .iter()
            .find(|building| building_footprint(building.origin).contains(&coord))
    }

    pub fn building_cost(&self, kind: BuildingKind) -> i64 {
        let placed_count = self
            .buildings
            .iter()
            .filter(|building| building.kind == kind)
            .count() as i32;

        scaled_cost(building_base_cost(kind), placed_count)
    }

    pub fn quote_building(&self, kind: BuildingKind, origin: TileCoord) -> BuildingQuote {
        let footprint = building_footprint(origin);
        let cost = self.building_cost(kind);

        if kind == BuildingKind::CityCore {
            return invalid_building_quote(cost, footprint, InvalidPlacement::CityCoreIsFixed);
        }

        if self.sky_coin < cost {
            return invalid_building_quote(cost, footprint, InvalidPlacement::NotEnoughSkyCoin);
        }

        let Some(first_height) = self.tile_height(footprint[0]) else {
            return invalid_building_quote(
                cost,
                footprint,
                InvalidPlacement::BuildingFootprintMustBeOnFlyingIsland,
            );
        };

        for coord in footprint {
            if self.tile_height(coord) != Some(first_height) {
                return invalid_building_quote(
                    cost,
                    footprint,
                    InvalidPlacement::BuildingFootprintMustBeBuildableAndOneHeight,
                );
            }

            if self.is_occupied(coord) {
                return invalid_building_quote(
                    cost,
                    footprint,
                    InvalidPlacement::BuildingFootprintIsOccupied,
                );
            }

            if self.roads.contains_key(&coord) {
                return invalid_building_quote(
                    cost,
                    footprint,
                    InvalidPlacement::BuildingFootprintContainsRoad,
                );
            }
        }

        BuildingQuote {
            valid: true,
            cost,
            footprint,
            invalid_reason: None,
        }
    }

    pub fn place_building(
        &mut self,
        kind: BuildingKind,
        origin: TileCoord,
    ) -> Result<BuildingId, InvalidPlacement> {
        self.ensure_editable()?;

        let quote = self.quote_building(kind, origin);
        if !quote.valid {
            return Err(quote
                .invalid_reason
                .expect("invalid quote should carry reason"));
        }

        let id = BuildingId(self.next_building_id);
        self.next_building_id += 1;
        self.sky_coin -= quote.cost;
        self.buildings.push(Building { id, kind, origin });
        self.recalculate_city_outputs();
        Ok(id)
    }

    pub fn quote_roads(&self, coords: &[TileCoord]) -> RoadPlacementQuote {
        let tiles = normalize_road_coords(coords);
        if tiles.is_empty() {
            return invalid_road_quote(
                tiles,
                Vec::new(),
                0,
                InvalidPlacement::RoadPlacementIsEmpty,
            );
        }

        let mut roads = self.roads.clone();
        let mut new_roads = Vec::new();
        let mut total_cost = 0;
        let mut previous_coord = None;

        for coord in tiles.iter().copied() {
            if let Some(previous) = previous_coord {
                if !are_orthogonal_neighbors(previous, coord) {
                    return invalid_road_quote(
                        tiles,
                        new_roads,
                        total_cost,
                        InvalidPlacement::RoadPlacementMustBeOrthogonallyContinuous,
                    );
                }

                if let Some(previous_road) = roads.get(&previous)
                    && let Some(existing_road) = roads.get(&coord)
                    && !roads_can_connect(*previous_road, *existing_road)
                {
                    return invalid_road_quote(
                        tiles,
                        new_roads,
                        total_cost,
                        InvalidPlacement::RoadHeightDifferenceTooSteep,
                    );
                }
            }

            if roads.contains_key(&coord) {
                previous_coord = Some(coord);
                continue;
            }

            if self.is_occupied(coord) {
                return invalid_road_quote(
                    tiles,
                    new_roads,
                    total_cost,
                    InvalidPlacement::RoadCannotOverlapBuilding,
                );
            }

            let Some(road) = self.proposed_road(coord, previous_coord, &roads) else {
                return invalid_road_quote(
                    tiles,
                    new_roads,
                    total_cost,
                    InvalidPlacement::RoadHeightDifferenceTooSteep,
                );
            };

            if let Some(previous) = previous_coord
                && let Some(previous_road) = roads.get(&previous)
                && !roads_can_connect(*previous_road, road)
            {
                return invalid_road_quote(
                    tiles,
                    new_roads,
                    total_cost,
                    InvalidPlacement::RoadHeightDifferenceTooSteep,
                );
            }

            total_cost += self.road_cost_for(coord);
            if self.sky_coin < total_cost {
                return invalid_road_quote(
                    tiles,
                    new_roads,
                    total_cost,
                    InvalidPlacement::NotEnoughSkyCoin,
                );
            }

            roads.insert(coord, road);
            new_roads.push(road);
            previous_coord = Some(coord);
        }

        RoadPlacementQuote {
            valid: true,
            total_cost,
            tiles,
            new_roads,
            invalid_reason: None,
        }
    }

    pub fn place_roads(
        &mut self,
        coords: &[TileCoord],
    ) -> Result<Vec<TileCoord>, InvalidPlacement> {
        self.ensure_editable()?;

        let quote = self.quote_roads(coords);
        if !quote.valid {
            return Err(quote
                .invalid_reason
                .expect("invalid quote should carry reason"));
        }

        let placed_coords = quote
            .new_roads
            .iter()
            .map(|road| road.coord)
            .collect::<Vec<_>>();
        self.sky_coin -= quote.total_cost;
        for road in quote.new_roads {
            self.roads.insert(road.coord, road);
        }
        self.recalculate_city_outputs();

        Ok(placed_coords)
    }

    pub fn demolish_tile(
        &mut self,
        coord: TileCoord,
    ) -> Result<DemolitionOutcome, InvalidPlacement> {
        self.ensure_editable()?;

        if self.roads.remove(&coord).is_some() {
            self.recalculate_city_outputs();
            return Ok(DemolitionOutcome::Road { coord });
        }

        let Some(building_index) = self
            .buildings
            .iter()
            .position(|building| building_footprint(building.origin).contains(&coord))
        else {
            return Err(InvalidPlacement::NothingToDemolish);
        };

        let building = &self.buildings[building_index];
        if building.kind == BuildingKind::CityCore {
            return Err(InvalidPlacement::CityCoreCannotBeDemolished);
        }

        let building = self.buildings.remove(building_index);
        self.recalculate_city_outputs();
        Ok(DemolitionOutcome::Building {
            id: building.id,
            kind: building.kind,
        })
    }

    pub fn connected_road_coords(&self) -> BTreeSet<TileCoord> {
        connected_road_coords(&self.roads, self.city_core_footprint())
    }

    pub fn is_road_connected(&self, coord: TileCoord) -> bool {
        self.connected_road_coords().contains(&coord)
    }

    pub fn is_building_active(&self, id: BuildingId) -> bool {
        let Some(building) = self.buildings.iter().find(|building| building.id == id) else {
            return false;
        };

        if building.kind == BuildingKind::CityCore {
            return true;
        }

        let connected_roads = self.connected_road_coords();
        building_footprint(building.origin)
            .into_iter()
            .flat_map(orthogonal_neighbors)
            .any(|coord| connected_roads.contains(&coord))
    }

    fn proposed_road(
        &self,
        coord: TileCoord,
        previous_coord: Option<TileCoord>,
        roads: &BTreeMap<TileCoord, Road>,
    ) -> Option<Road> {
        if let Some(height) = self.tile_height(coord) {
            return Some(Road { coord, height });
        }

        let inherited_height = previous_coord
            .and_then(|previous| roads.get(&previous).copied())
            .map(|previous| previous.height)
            .or_else(|| {
                orthogonal_neighbors(coord)
                    .into_iter()
                    .filter_map(|neighbor| roads.get(&neighbor))
                    .map(|road| road.height)
                    .next()
            })
            .unwrap_or(0);

        Some(Road {
            coord,
            height: inherited_height,
        })
    }

    fn road_cost_for(&self, coord: TileCoord) -> i64 {
        if self.tile_height(coord).is_some() {
            ISLAND_ROAD_COST
        } else {
            SKY_ROAD_COST
        }
    }

    fn city_core_footprint(&self) -> Option<[TileCoord; 4]> {
        self.buildings
            .iter()
            .find(|building| building.kind == BuildingKind::CityCore)
            .map(|building| building_footprint(building.origin))
    }

    fn ensure_editable(&self) -> Result<(), InvalidPlacement> {
        if self.status != RunStatus::Running {
            return Err(InvalidPlacement::RunIsNotEditable);
        }

        Ok(())
    }

    fn recalculate_city_outputs(&mut self) {
        let active_kinds = self
            .active_non_core_buildings()
            .map(|building| building.kind)
            .collect::<Vec<_>>();

        self.citizen_capacity = CITY_CORE_CITIZEN_CAPACITY;
        self.food.production_per_second = 0.0;

        for kind in active_kinds {
            match kind {
                BuildingKind::House => {
                    self.citizen_capacity += HOUSE_CITIZEN_CAPACITY;
                }
                BuildingKind::Farm => {
                    self.food.production_per_second += FARM_FOOD_PRODUCTION_PER_SECOND;
                }
                BuildingKind::CityCore
                | BuildingKind::Workshop
                | BuildingKind::Market
                | BuildingKind::Monument => {}
            }
        }

        self.food.consumption_per_second = self.citizens as f64 * FOOD_CONSUMPTION_PER_CITIZEN;
        self.citizens = self.citizens.min(self.citizen_capacity);
    }

    fn active_non_core_buildings(&self) -> impl Iterator<Item = &Building> {
        let connected_roads = self.connected_road_coords();
        self.buildings
            .iter()
            .filter(|building| building.kind != BuildingKind::CityCore)
            .filter(move |building| {
                building_footprint(building.origin)
                    .into_iter()
                    .flat_map(orthogonal_neighbors)
                    .any(|coord| connected_roads.contains(&coord))
            })
    }

    fn apply_citizen_arrivals(&mut self) {
        if self.food.current <= 0 || self.citizens >= self.citizen_capacity {
            return;
        }

        let arriving = CITIZEN_ARRIVAL_PER_SECOND.floor() as u32;
        self.citizens = (self.citizens + arriving).min(self.citizen_capacity);
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

pub fn building_base_cost(kind: BuildingKind) -> i64 {
    match kind {
        BuildingKind::CityCore => 0,
        BuildingKind::House => 25,
        BuildingKind::Farm => 35,
        BuildingKind::Workshop => 75,
        BuildingKind::Market => 120,
        BuildingKind::Monument => 300,
    }
}

fn scaled_cost(base_cost: i64, placed_count: i32) -> i64 {
    ((base_cost as f64) * BUILDING_COST_MULTIPLIER.powi(placed_count)).ceil() as i64
}

fn invalid_building_quote(
    cost: i64,
    footprint: [TileCoord; 4],
    invalid_reason: InvalidPlacement,
) -> BuildingQuote {
    BuildingQuote {
        valid: false,
        cost,
        footprint,
        invalid_reason: Some(invalid_reason),
    }
}

fn invalid_road_quote(
    tiles: Vec<TileCoord>,
    new_roads: Vec<Road>,
    total_cost: i64,
    invalid_reason: InvalidPlacement,
) -> RoadPlacementQuote {
    RoadPlacementQuote {
        valid: false,
        total_cost,
        tiles,
        new_roads,
        invalid_reason: Some(invalid_reason),
    }
}

fn normalize_road_coords(coords: &[TileCoord]) -> Vec<TileCoord> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();

    for coord in coords {
        if seen.insert(*coord) {
            normalized.push(*coord);
        }
    }

    normalized
}

fn connected_road_coords(
    roads: &BTreeMap<TileCoord, Road>,
    city_core_footprint: Option<[TileCoord; 4]>,
) -> BTreeSet<TileCoord> {
    let Some(city_core_footprint) = city_core_footprint else {
        return BTreeSet::new();
    };

    let mut connected = BTreeSet::new();
    let mut frontier = VecDeque::new();

    for road in roads.values() {
        if road_touches_city_core(*road, city_core_footprint) {
            connected.insert(road.coord);
            frontier.push_back(*road);
        }
    }

    while let Some(road) = frontier.pop_front() {
        for neighbor_coord in orthogonal_neighbors(road.coord) {
            let Some(neighbor_road) = roads.get(&neighbor_coord).copied() else {
                continue;
            };

            if connected.contains(&neighbor_road.coord) || !roads_can_connect(road, neighbor_road) {
                continue;
            }

            connected.insert(neighbor_road.coord);
            frontier.push_back(neighbor_road);
        }
    }

    connected
}

fn road_touches_city_core(road: Road, city_core_footprint: [TileCoord; 4]) -> bool {
    orthogonal_neighbors(road.coord)
        .into_iter()
        .any(|coord| city_core_footprint.contains(&coord))
}

fn roads_can_connect(left: Road, right: Road) -> bool {
    are_orthogonal_neighbors(left.coord, right.coord) && (left.height - right.height).abs() <= 1
}

fn are_orthogonal_neighbors(left: TileCoord, right: TileCoord) -> bool {
    (left.x - right.x).abs() + (left.z - right.z).abs() == 1
}

fn orthogonal_neighbors(coord: TileCoord) -> [TileCoord; 4] {
    [
        TileCoord::new(coord.x + 1, coord.z),
        TileCoord::new(coord.x - 1, coord.z),
        TileCoord::new(coord.x, coord.z + 1),
        TileCoord::new(coord.x, coord.z - 1),
    ]
}

fn tile_height_index(islands: &[FlyingIsland]) -> BTreeMap<TileCoord, i32> {
    islands
        .iter()
        .flat_map(|island| island.tiles())
        .map(|tile| (tile.coord, tile.height))
        .collect()
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
    use std::time::{Duration, Instant};

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

    #[test]
    fn building_cost_is_paid_and_scales_by_placed_count() {
        let mut run = RunState::start(7);
        let first_origin = valid_unoccupied_building_origin(&run);

        let first_quote = run.quote_building(BuildingKind::House, first_origin);
        assert_eq!(first_quote.cost, 25);

        let first_id = run
            .place_building(BuildingKind::House, first_origin)
            .unwrap();

        assert_eq!(first_id, BuildingId(1));
        assert_eq!(run.sky_coin, 475);

        let second_origin = valid_unoccupied_building_origin(&run);
        let second_quote = run.quote_building(BuildingKind::House, second_origin);
        assert_eq!(second_quote.cost, 29);
    }

    #[test]
    fn building_placement_rejects_road_overlap() {
        let mut run = RunState::start(7);
        let origin = valid_unoccupied_building_origin(&run);
        let height = run.tile_height(origin).unwrap();
        run.roads.insert(
            origin,
            Road {
                coord: origin,
                height,
            },
        );

        let quote = run.quote_building(BuildingKind::House, origin);

        assert!(!quote.valid);
        assert_eq!(
            quote.invalid_reason,
            Some(InvalidPlacement::BuildingFootprintContainsRoad)
        );
    }

    #[test]
    fn road_quote_charges_new_island_and_sky_roads_only() {
        let mut run = RunState::start(7);
        let existing_sky_road = TileCoord::new(20, 20);
        let new_sky_road = TileCoord::new(20, 21);
        insert_existing_connected_road_chain_to(&mut run, existing_sky_road);

        let quote = run.quote_roads(&[existing_sky_road, new_sky_road, new_sky_road]);

        assert!(quote.valid, "{quote:?}");
        assert_eq!(quote.tiles, vec![existing_sky_road, new_sky_road]);
        assert_eq!(quote.total_cost, SKY_ROAD_COST);
        assert_eq!(
            quote.new_roads,
            vec![Road {
                coord: new_sky_road,
                height: 1,
            }]
        );
    }

    #[test]
    fn road_placement_rejects_invalid_whole_drag() {
        let mut run = RunState::start(7);
        let building_origin = valid_unoccupied_building_origin(&run);
        let building_id = run
            .place_building(BuildingKind::House, building_origin)
            .unwrap();
        let before_sky_coin = run.sky_coin;

        let result = run.place_roads(&[building_origin]);

        assert_eq!(building_id, BuildingId(1));
        assert_eq!(result, Err(InvalidPlacement::RoadCannotOverlapBuilding));
        assert_eq!(run.sky_coin, before_sky_coin);
        assert!(run.roads.is_empty());
    }

    #[test]
    fn road_placement_rejects_unaffordable_whole_drag() {
        let mut run = RunState::start(7);
        run.sky_coin = 1;

        let result = run.place_roads(&[TileCoord::new(0, -1)]);

        assert_eq!(result, Err(InvalidPlacement::NotEnoughSkyCoin));
        assert_eq!(run.sky_coin, 1);
        assert!(run.roads.is_empty());
    }

    #[test]
    fn road_connectivity_reaches_city_core_through_orthogonal_chains() {
        let mut run = RunState::start(7);
        run.place_roads(&[
            TileCoord::new(0, -1),
            TileCoord::new(0, -2),
            TileCoord::new(0, -3),
        ])
        .unwrap();

        assert!(run.is_road_connected(TileCoord::new(0, -1)));
        assert!(run.is_road_connected(TileCoord::new(0, -2)));
        assert!(run.is_road_connected(TileCoord::new(0, -3)));
    }

    #[test]
    fn disconnected_buildings_are_inactive_and_connected_buildings_are_active() {
        let mut run = RunState::start(7);
        let disconnected_origin = valid_unoccupied_building_origin(&run);
        let disconnected_id = run
            .place_building(BuildingKind::House, disconnected_origin)
            .unwrap();
        assert!(!run.is_building_active(disconnected_id));

        run.place_roads(&[
            TileCoord::new(0, -1),
            TileCoord::new(0, -2),
            TileCoord::new(0, -3),
        ])
        .unwrap();
        let connected_origin = valid_building_origin_adjacent_to_connected_road(&run);
        let connected_id = run
            .place_building(BuildingKind::Farm, connected_origin)
            .unwrap();

        assert!(run.is_building_active(BuildingId(0)));
        assert!(!run.is_building_active(disconnected_id));
        assert!(run.is_building_active(connected_id));
    }

    #[test]
    fn demolish_removes_roads_without_refund() {
        let mut run = RunState::start(7);
        run.place_roads(&[TileCoord::new(0, -1)]).unwrap();
        let sky_coin_after_placement = run.sky_coin;

        let outcome = run.demolish_tile(TileCoord::new(0, -1)).unwrap();

        assert_eq!(
            outcome,
            DemolitionOutcome::Road {
                coord: TileCoord::new(0, -1)
            }
        );
        assert!(!run.roads.contains_key(&TileCoord::new(0, -1)));
        assert_eq!(run.sky_coin, sky_coin_after_placement);
    }

    #[test]
    fn demolish_removes_non_city_core_buildings_without_refund() {
        let mut run = RunState::start(7);
        let origin = valid_unoccupied_building_origin(&run);
        let id = run.place_building(BuildingKind::House, origin).unwrap();
        let sky_coin_after_placement = run.sky_coin;

        let outcome = run.demolish_tile(origin).unwrap();

        assert_eq!(
            outcome,
            DemolitionOutcome::Building {
                id,
                kind: BuildingKind::House,
            }
        );
        assert!(run.buildings.iter().all(|building| building.id != id));
        assert_eq!(run.sky_coin, sky_coin_after_placement);
    }

    #[test]
    fn demolish_rejects_city_core() {
        let mut run = RunState::start(7);

        let result = run.demolish_tile(TileCoord::new(0, 0));

        assert_eq!(result, Err(InvalidPlacement::CityCoreCannotBeDemolished));
        assert!(
            run.buildings
                .iter()
                .any(|building| building.kind == BuildingKind::CityCore)
        );
    }

    #[test]
    fn demolition_can_disconnect_roads_and_buildings() {
        let mut run = RunState::start(7);
        run.place_roads(&[
            TileCoord::new(0, -1),
            TileCoord::new(0, -2),
            TileCoord::new(0, -3),
        ])
        .unwrap();
        let connected_origin = valid_building_origin_adjacent_to_road(&run, TileCoord::new(0, -3));
        let connected_id = run
            .place_building(BuildingKind::Farm, connected_origin)
            .unwrap();
        assert!(run.is_road_connected(TileCoord::new(0, -3)));
        assert!(run.is_building_active(connected_id));

        run.demolish_tile(TileCoord::new(0, -2)).unwrap();

        assert!(!run.is_road_connected(TileCoord::new(0, -3)));
        assert!(!run.is_building_active(connected_id));
    }

    #[test]
    fn bankrupt_run_rejects_placement_and_demolition() {
        let mut run = RunState::start(7);
        run.place_roads(&[TileCoord::new(0, -1)]).unwrap();
        let origin = valid_unoccupied_building_origin(&run);
        run.status = RunStatus::Bankrupt;

        let building_result = run.place_building(BuildingKind::House, origin);
        let road_result = run.place_roads(&[TileCoord::new(0, -2)]);
        let demolish_result = run.demolish_tile(TileCoord::new(0, -1));

        assert_eq!(building_result, Err(InvalidPlacement::RunIsNotEditable));
        assert_eq!(road_result, Err(InvalidPlacement::RunIsNotEditable));
        assert_eq!(demolish_result, Err(InvalidPlacement::RunIsNotEditable));
        assert!(run.roads.contains_key(&TileCoord::new(0, -1)));
    }

    #[test]
    fn disconnected_sky_roads_can_be_placed() {
        let mut run = RunState::start(7);

        let placed = run
            .place_roads(&[TileCoord::new(20, 20), TileCoord::new(20, 21)])
            .unwrap();

        assert_eq!(placed, vec![TileCoord::new(20, 20), TileCoord::new(20, 21)]);
        assert_eq!(
            run.road_kind_at(TileCoord::new(20, 20)),
            Some(RoadKind::Sky)
        );
        assert!(!run.is_road_connected(TileCoord::new(20, 20)));
    }

    #[test]
    fn only_active_buildings_affect_food_and_citizen_capacity() {
        let mut run = RunState::start(7);
        let disconnected_house = valid_unoccupied_building_origin(&run);
        run.place_building(BuildingKind::House, disconnected_house)
            .unwrap();

        run.tick();

        assert_eq!(run.citizen_capacity, 5);
        assert_eq!(run.food.production_per_second, 0.0);

        let mut run = RunState::start(7);
        run.place_roads(&[
            TileCoord::new(0, -1),
            TileCoord::new(0, -2),
            TileCoord::new(0, -3),
        ])
        .unwrap();
        let connected_house = valid_building_origin_adjacent_to_connected_road(&run);
        run.place_building(BuildingKind::House, connected_house)
            .unwrap();

        run.tick();

        assert_eq!(run.citizen_capacity, 11);

        let mut run = RunState::start(7);
        run.place_roads(&[
            TileCoord::new(0, -1),
            TileCoord::new(0, -2),
            TileCoord::new(0, -3),
        ])
        .unwrap();
        let connected_farm = valid_building_origin_adjacent_to_connected_road(&run);
        run.place_building(BuildingKind::Farm, connected_farm)
            .unwrap();

        run.tick();

        assert_eq!(
            run.food.production_per_second,
            FARM_FOOD_PRODUCTION_PER_SECOND
        );
    }

    #[test]
    fn empty_tile_height_lookup_stays_fast_with_generated_islands() {
        let run = RunState::start(7);
        let empty_coord = TileCoord::new(10_000, 10_000);

        let started_at = Instant::now();
        for _ in 0..20_000 {
            assert_eq!(run.tile_height(empty_coord), None);
        }
        let elapsed = started_at.elapsed();

        assert!(
            elapsed < Duration::from_millis(50),
            "20,000 empty tile height lookups took {elapsed:?}"
        );
    }

    fn valid_unoccupied_building_origin(run: &RunState) -> TileCoord {
        run.islands
            .iter()
            .flat_map(|island| island.tiles())
            .map(|tile| tile.coord)
            .find(|origin| {
                let footprint = building_footprint(*origin);
                let Some(first_height) = run.tile_height(footprint[0]) else {
                    return false;
                };

                footprint.iter().all(|coord| {
                    run.tile_height(*coord) == Some(first_height)
                        && !run.is_occupied(*coord)
                        && !run.roads.contains_key(coord)
                })
            })
            .expect("generated island should have at least one buildable 2x2 footprint")
    }

    fn valid_building_origin_adjacent_to_connected_road(run: &RunState) -> TileCoord {
        let connected_roads = run.connected_road_coords();
        run.islands
            .iter()
            .flat_map(|island| island.tiles())
            .map(|tile| tile.coord)
            .find(|origin| {
                let footprint = building_footprint(*origin);
                let Some(first_height) = run.tile_height(footprint[0]) else {
                    return false;
                };

                let footprint_is_valid = footprint.iter().all(|coord| {
                    run.tile_height(*coord) == Some(first_height)
                        && !run.is_occupied(*coord)
                        && !run.roads.contains_key(coord)
                });
                let touches_connected_road = footprint
                    .into_iter()
                    .flat_map(orthogonal_neighbors)
                    .any(|coord| connected_roads.contains(&coord));

                footprint_is_valid && touches_connected_road
            })
            .expect("generated island should have a valid footprint next to the connected Road")
    }

    fn valid_building_origin_adjacent_to_road(run: &RunState, road_coord: TileCoord) -> TileCoord {
        run.islands
            .iter()
            .flat_map(|island| island.tiles())
            .map(|tile| tile.coord)
            .find(|origin| {
                let footprint = building_footprint(*origin);
                let Some(first_height) = run.tile_height(footprint[0]) else {
                    return false;
                };

                let footprint_is_valid = footprint.iter().all(|coord| {
                    run.tile_height(*coord) == Some(first_height)
                        && !run.is_occupied(*coord)
                        && !run.roads.contains_key(coord)
                });
                let touches_road = footprint
                    .into_iter()
                    .flat_map(orthogonal_neighbors)
                    .any(|coord| coord == road_coord);

                footprint_is_valid && touches_road
            })
            .expect("generated island should have a valid footprint next to the target Road")
    }

    fn insert_existing_connected_road_chain_to(run: &mut RunState, target: TileCoord) {
        let mut coord = TileCoord::new(2, 0);

        loop {
            run.roads.insert(coord, Road { coord, height: 1 });

            if coord == target {
                break;
            }

            if coord.x != target.x {
                coord = TileCoord::new(coord.x + (target.x - coord.x).signum(), coord.z);
            } else {
                coord = TileCoord::new(coord.x, coord.z + (target.z - coord.z).signum());
            }
        }

        assert!(run.is_road_connected(target));
        assert_eq!(run.road_kind_at(target), Some(RoadKind::Sky));
    }
}
