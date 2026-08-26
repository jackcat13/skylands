use crate::building::{Building, BuildingKind, building_footprint};
use crate::road::{ISLAND_ROAD_COST, Road, SKY_ROAD_COST, roads_can_connect};
use crate::simulation::{InvalidPlacement, RoadPlacementQuote};
use crate::world::TileCoord;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(crate) struct RoadNetwork<'a> {
    roads: &'a BTreeMap<TileCoord, Road>,
    tile_heights: &'a BTreeMap<TileCoord, i32>,
    buildings: &'a [Building],
    city_core_footprint: Option<[TileCoord; 4]>,
    sky_coin: i64,
}

impl<'a> RoadNetwork<'a> {
    pub(crate) fn new(
        roads: &'a BTreeMap<TileCoord, Road>,
        tile_heights: &'a BTreeMap<TileCoord, i32>,
        buildings: &'a [Building],
        city_core_footprint: Option<[TileCoord; 4]>,
        sky_coin: i64,
    ) -> Self {
        Self {
            roads,
            tile_heights,
            buildings,
            city_core_footprint,
            sky_coin,
        }
    }

    pub(crate) fn quote_roads(&self, coords: &[TileCoord]) -> RoadPlacementQuote {
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
        let mut previous_coord: Option<TileCoord> = None;

        for coord in tiles.iter().copied() {
            if let Some(previous) = previous_coord {
                if previous.manhattan_distance_to(coord) != 1 {
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

    pub(crate) fn connected_road_coords(&self) -> BTreeSet<TileCoord> {
        self.city_connectivity().connected_road_coords().clone()
    }

    pub(crate) fn city_connectivity(&self) -> CityConnectivity {
        CityConnectivity {
            connected_roads: connected_road_coords(self.roads, self.city_core_footprint),
        }
    }

    fn is_occupied(&self, coord: TileCoord) -> bool {
        self.buildings
            .iter()
            .any(|building| building_footprint(building.origin).contains(&coord))
    }

    fn proposed_road(
        &self,
        coord: TileCoord,
        previous_coord: Option<TileCoord>,
        roads: &BTreeMap<TileCoord, Road>,
    ) -> Option<Road> {
        if let Some(height) = self.tile_heights.get(&coord).copied() {
            return Some(Road { coord, height });
        }

        let inherited_height = previous_coord
            .and_then(|previous| roads.get(&previous).copied())
            .map(|previous| previous.height)
            .or_else(|| {
                coord
                    .orthogonal_neighbors()
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
        if self.tile_heights.contains_key(&coord) {
            ISLAND_ROAD_COST
        } else {
            SKY_ROAD_COST
        }
    }
}

#[derive(Debug, Clone)]
pub struct CityConnectivity {
    connected_roads: BTreeSet<TileCoord>,
}

impl CityConnectivity {
    pub fn connected_road_coords(&self) -> &BTreeSet<TileCoord> {
        &self.connected_roads
    }

    pub fn is_road_connected(&self, coord: TileCoord) -> bool {
        self.connected_roads.contains(&coord)
    }

    pub fn is_building_active(&self, building: &Building) -> bool {
        if building.kind == BuildingKind::CityCore {
            return true;
        }

        building_footprint(building.origin)
            .into_iter()
            .flat_map(TileCoord::orthogonal_neighbors)
            .any(|coord| self.connected_roads.contains(&coord))
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
        for neighbor_coord in road.coord.orthogonal_neighbors() {
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
    road.coord
        .orthogonal_neighbors()
        .into_iter()
        .any(|coord| city_core_footprint.contains(&coord))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::{BuildingId, BuildingKind};
    use crate::road::{RoadKind, SKY_ROAD_COST};

    #[test]
    fn road_kind_names_match_domain_language() {
        assert_eq!(RoadKind::Island.name(), "Island");
        assert_eq!(RoadKind::Sky.name(), "Sky");
    }

    #[test]
    fn quote_charges_new_island_and_sky_roads_only() {
        let mut roads = connected_road_chain_to(TileCoord::new(20, 20));
        let tile_heights = starting_island_heights();
        let new_sky_road = TileCoord::new(20, 21);
        let network = RoadNetwork::new(
            &roads,
            &tile_heights,
            &[],
            Some(city_core_footprint()),
            SKY_ROAD_COST,
        );

        let quote = network.quote_roads(&[TileCoord::new(20, 20), new_sky_road, new_sky_road]);

        assert!(quote.valid, "{quote:?}");
        assert_eq!(quote.tiles, vec![TileCoord::new(20, 20), new_sky_road]);
        assert_eq!(quote.total_cost, SKY_ROAD_COST);
        assert_eq!(
            quote.new_roads,
            vec![Road {
                coord: new_sky_road,
                height: 1,
            }]
        );

        roads.insert(new_sky_road, quote.new_roads[0]);
        assert!(
            RoadNetwork::new(
                &roads,
                &tile_heights,
                &[],
                Some(city_core_footprint()),
                SKY_ROAD_COST,
            )
            .city_connectivity()
            .is_road_connected(new_sky_road)
        );
    }

    #[test]
    fn quote_rejects_invalid_whole_drag_without_new_roads() {
        let roads = BTreeMap::new();
        let tile_heights = starting_island_heights();
        let buildings = vec![Building {
            id: BuildingId(1),
            kind: BuildingKind::House,
            origin: TileCoord::new(0, -1),
        }];
        let network = RoadNetwork::new(
            &roads,
            &tile_heights,
            &buildings,
            Some(city_core_footprint()),
            500,
        );

        let quote = network.quote_roads(&[TileCoord::new(0, -1)]);

        assert_eq!(
            quote.invalid_reason,
            Some(InvalidPlacement::RoadCannotOverlapBuilding)
        );
        assert!(quote.new_roads.is_empty());
    }

    #[test]
    fn quote_rejects_unaffordable_whole_drag() {
        let roads = BTreeMap::new();
        let tile_heights = starting_island_heights();
        let network = RoadNetwork::new(&roads, &tile_heights, &[], Some(city_core_footprint()), 1);

        let quote = network.quote_roads(&[TileCoord::new(0, -1)]);

        assert_eq!(
            quote.invalid_reason,
            Some(InvalidPlacement::NotEnoughSkyCoin)
        );
        assert!(quote.new_roads.is_empty());
    }

    #[test]
    fn quote_allows_disconnected_sky_roads() {
        let roads = BTreeMap::new();
        let tile_heights = starting_island_heights();
        let first_sky_road = TileCoord::new(20, 20);
        let second_sky_road = TileCoord::new(20, 21);
        let network = RoadNetwork::new(
            &roads,
            &tile_heights,
            &[],
            Some(city_core_footprint()),
            SKY_ROAD_COST * 2,
        );

        let quote = network.quote_roads(&[first_sky_road, second_sky_road]);

        assert!(quote.valid, "{quote:?}");
        assert_eq!(quote.total_cost, SKY_ROAD_COST * 2);
        assert_eq!(
            quote.new_roads,
            vec![
                Road {
                    coord: first_sky_road,
                    height: 0,
                },
                Road {
                    coord: second_sky_road,
                    height: 0,
                },
            ]
        );
        assert!(
            !network
                .city_connectivity()
                .is_road_connected(first_sky_road)
        );
    }

    #[test]
    fn connectivity_reaches_city_core_through_orthogonal_chains() {
        let roads = connected_road_chain_to(TileCoord::new(0, -3));
        let tile_heights = starting_island_heights();
        let network =
            RoadNetwork::new(&roads, &tile_heights, &[], Some(city_core_footprint()), 500);
        let connectivity = network.city_connectivity();

        assert!(connectivity.is_road_connected(TileCoord::new(0, -1)));
        assert!(connectivity.is_road_connected(TileCoord::new(0, -2)));
        assert!(connectivity.is_road_connected(TileCoord::new(0, -3)));
    }

    #[test]
    fn disconnected_buildings_are_inactive_and_connected_buildings_are_active() {
        let roads = connected_road_chain_to(TileCoord::new(0, -3));
        let tile_heights = starting_island_heights();
        let disconnected = Building {
            id: BuildingId(1),
            kind: BuildingKind::House,
            origin: TileCoord::new(5, 5),
        };
        let connected = Building {
            id: BuildingId(2),
            kind: BuildingKind::Farm,
            origin: TileCoord::new(0, -5),
        };
        let buildings = vec![disconnected.clone(), connected.clone()];
        let connectivity = RoadNetwork::new(
            &roads,
            &tile_heights,
            &buildings,
            Some(city_core_footprint()),
            500,
        )
        .city_connectivity();

        assert!(connectivity.is_building_active(&Building {
            id: BuildingId(0),
            kind: BuildingKind::CityCore,
            origin: TileCoord::new(0, 0),
        }));
        assert!(!connectivity.is_building_active(&disconnected));
        assert!(connectivity.is_building_active(&connected));
    }

    fn city_core_footprint() -> [TileCoord; 4] {
        [
            TileCoord::new(0, 0),
            TileCoord::new(1, 0),
            TileCoord::new(0, 1),
            TileCoord::new(1, 1),
        ]
    }

    fn starting_island_heights() -> BTreeMap<TileCoord, i32> {
        [
            TileCoord::new(0, 0),
            TileCoord::new(1, 0),
            TileCoord::new(0, 1),
            TileCoord::new(1, 1),
            TileCoord::new(0, -1),
            TileCoord::new(0, -2),
            TileCoord::new(0, -3),
        ]
        .into_iter()
        .map(|coord| (coord, 1))
        .collect()
    }

    fn connected_road_chain_to(target: TileCoord) -> BTreeMap<TileCoord, Road> {
        let mut roads = BTreeMap::new();
        let mut coord = if target.x == 0 {
            TileCoord::new(0, -1)
        } else {
            TileCoord::new(2, 0)
        };

        loop {
            roads.insert(coord, Road { coord, height: 1 });

            if coord == target {
                break;
            }

            if coord.x != target.x {
                coord = TileCoord::new(coord.x + (target.x - coord.x).signum(), coord.z);
            } else {
                coord = TileCoord::new(coord.x, coord.z + (target.z - coord.z).signum());
            }
        }

        roads
    }
}
