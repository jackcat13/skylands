use serde::{Deserialize, Serialize};

pub const GENERATED_ISLAND_COUNT: usize = 1_000;
const MIN_ISLAND_CENTER_DISTANCE: i32 = 64;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct TileCoord {
    pub x: i32,
    pub z: i32,
}

impl TileCoord {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IslandTile {
    pub coord: TileCoord,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FlyingIslandId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlyingIsland {
    pub id: FlyingIslandId,
    #[serde(default)]
    bounds_min: TileCoord,
    #[serde(default)]
    bounds_max: TileCoord,
    tiles: Vec<IslandTile>,
}

impl FlyingIsland {
    pub fn generated(seed: u64) -> Self {
        generated_island(seed, FlyingIslandId(0), TileCoord::new(0, 0), true)
    }

    pub fn generated_many(seed: u64) -> Vec<Self> {
        generated_island_centers(seed)
            .into_iter()
            .enumerate()
            .map(|(index, center)| {
                generated_island(seed, FlyingIslandId(index as u32), center, index == 0)
            })
            .collect()
    }

    pub fn tile(&self, coord: TileCoord) -> Option<&IslandTile> {
        self.tiles.iter().find(|tile| tile.coord == coord)
    }

    pub fn tiles(&self) -> impl Iterator<Item = &IslandTile> {
        self.tiles.iter()
    }

    pub fn bounds(&self) -> Option<(TileCoord, TileCoord)> {
        (!self.tiles.is_empty()).then_some((self.bounds_min, self.bounds_max))
    }
}

fn generated_island(
    seed: u64,
    id: FlyingIslandId,
    center: TileCoord,
    reserve_city_core_footprint: bool,
) -> FlyingIsland {
    let mut tiles = Vec::new();
    let island_seed = seed ^ (id.0 as u64).wrapping_mul(0x9e3779b97f4a7c15);
    let radius_x = 12 + (hash_height(island_seed, center.x, center.z) % 9) as i32;
    let radius_z = 9 + (hash_height(island_seed, center.x - 41, center.z + 73) % 10) as i32;
    let edge_radius_x = radius_x + 3;
    let edge_radius_z = radius_z + 3;
    let base_height = (hash_height(island_seed, center.x + 19, center.z - 23) % 4) as i32;
    let plateau_radius = radius_x.min(radius_z);
    let raised_plateau = plateau(center, island_seed, plateau_radius, 0);
    let lowered_plateau = plateau(center, island_seed, plateau_radius, 1);

    for x in (center.x - edge_radius_x)..=(center.x + edge_radius_x) {
        for z in (center.z - edge_radius_z)..=(center.z + edge_radius_z) {
            let local_x = x - center.x;
            let local_z = z - center.z;
            let noise = hash_height(island_seed, x, z);
            let distance = ellipse_distance(local_x, local_z, radius_x, radius_z);
            let edge_noise = 0.18 + (noise % 16) as f32 / 100.0;
            let within_shape = distance <= 1.0 || (distance <= 1.0 + edge_noise && noise % 4 != 0);

            if within_shape {
                let height = plateau_height(
                    base_height,
                    TileCoord::new(x, z),
                    raised_plateau,
                    lowered_plateau,
                );
                let coord = TileCoord::new(x, z);
                upsert_tile(&mut tiles, IslandTile { coord, height });
            }
        }
    }

    if reserve_city_core_footprint {
        for coord in [
            TileCoord::new(0, 0),
            TileCoord::new(1, 0),
            TileCoord::new(0, 1),
            TileCoord::new(1, 1),
        ] {
            upsert_tile(
                &mut tiles,
                IslandTile {
                    coord,
                    height: base_height,
                },
            );
        }
    }

    tiles.sort_by_key(|tile| tile.coord);

    flying_island(id, tiles)
}

#[derive(Debug, Clone, Copy)]
struct Plateau {
    center: TileCoord,
    radius: i32,
}

fn plateau(island_center: TileCoord, seed: u64, island_radius: i32, index: i32) -> Plateau {
    let angle_steps = hash_height(seed, index * 17 + 3, index * 29 - 5) % 360;
    let angle = angle_steps as f32 * std::f32::consts::TAU / 360.0;
    let distance = (island_radius / 2 + (hash_height(seed, index * 31, index * 37) % 3) as i32)
        * if index == 0 { 1 } else { -1 };

    Plateau {
        center: TileCoord::new(
            island_center.x + (angle.cos() * distance as f32).round() as i32,
            island_center.z + (angle.sin() * distance as f32).round() as i32,
        ),
        radius: (island_radius / 3).max(3),
    }
}

fn plateau_height(
    base_height: i32,
    coord: TileCoord,
    raised_plateau: Plateau,
    lowered_plateau: Plateau,
) -> i32 {
    if manhattan_distance(coord, raised_plateau.center) <= raised_plateau.radius {
        base_height + 1
    } else if manhattan_distance(coord, lowered_plateau.center) <= lowered_plateau.radius {
        (base_height - 1).max(0)
    } else {
        base_height
    }
}

fn generated_island_centers(seed: u64) -> Vec<TileCoord> {
    let mut centers = Vec::with_capacity(GENERATED_ISLAND_COUNT);
    centers.push(TileCoord::new(0, 0));

    for index in 1..GENERATED_ISLAND_COUNT {
        let index = index as i32;
        let mut center;
        let mut attempt = 0_i32;

        loop {
            let noise = hash_height(seed, index, attempt);
            let angle = (noise & 0xffff) as f32 / 65_535.0 * std::f32::consts::TAU;
            let radius_noise = ((noise >> 16) & 0xffff) as f32 / 65_535.0;
            let max_radius = 80.0 + (index as f32).sqrt() * 38.0;
            let radius = 34.0 + radius_noise.sqrt() * max_radius + attempt as f32 * 12.0;

            center = TileCoord::new(
                (angle.cos() * radius).round() as i32,
                (angle.sin() * radius).round() as i32,
            );

            if centers
                .iter()
                .all(|existing| manhattan_distance(*existing, center) >= MIN_ISLAND_CENTER_DISTANCE)
            {
                break;
            }

            attempt += 1;
        }

        centers.push(center);
    }

    centers
}

fn ellipse_distance(local_x: i32, local_z: i32, radius_x: i32, radius_z: i32) -> f32 {
    let x = local_x as f32 / radius_x as f32;
    let z = local_z as f32 / radius_z as f32;
    x * x + z * z
}

fn flying_island(id: FlyingIslandId, tiles: Vec<IslandTile>) -> FlyingIsland {
    let (bounds_min, bounds_max) = tile_bounds(&tiles).unwrap_or_default();

    FlyingIsland {
        id,
        bounds_min,
        bounds_max,
        tiles,
    }
}

fn tile_bounds(tiles: &[IslandTile]) -> Option<(TileCoord, TileCoord)> {
    let first = tiles.first()?;
    let mut min = first.coord;
    let mut max = first.coord;

    for tile in tiles {
        min.x = min.x.min(tile.coord.x);
        min.z = min.z.min(tile.coord.z);
        max.x = max.x.max(tile.coord.x);
        max.z = max.z.max(tile.coord.z);
    }

    Some((min, max))
}

fn manhattan_distance(left: TileCoord, right: TileCoord) -> i32 {
    (left.x - right.x).abs() + (left.z - right.z).abs()
}

fn upsert_tile(tiles: &mut Vec<IslandTile>, new_tile: IslandTile) {
    if let Some(tile) = tiles.iter_mut().find(|tile| tile.coord == new_tile.coord) {
        *tile = new_tile;
    } else {
        tiles.push(new_tile);
    }
}

fn hash_height(seed: u64, x: i32, z: i32) -> u64 {
    let mut value = seed ^ ((x as i64 as u64) << 32) ^ z as i64 as u64;
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51afd7ed558ccd);
    value ^= value >> 33;
    value = value.wrapping_mul(0xc4ceb9fe1a85ec53);
    value ^ (value >> 33)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_island_is_deterministic_for_a_seed() {
        assert_eq!(FlyingIsland::generated(42), FlyingIsland::generated(42));
    }

    #[test]
    fn generated_island_reserves_a_valid_city_core_footprint() {
        let island = FlyingIsland::generated(42);

        let expected_height = island.tile(TileCoord::new(0, 0)).map(|tile| tile.height);
        for coord in [
            TileCoord::new(0, 0),
            TileCoord::new(1, 0),
            TileCoord::new(0, 1),
            TileCoord::new(1, 1),
        ] {
            assert_eq!(island.tile(coord).map(|tile| tile.height), expected_height);
        }
    }

    #[test]
    fn generated_many_creates_many_deterministic_islands() {
        let islands = FlyingIsland::generated_many(42);

        assert_eq!(islands.len(), GENERATED_ISLAND_COUNT);
        assert_eq!(islands, FlyingIsland::generated_many(42));
        assert_eq!(islands[0].id, FlyingIslandId(0));
        assert_eq!(islands[999].id, FlyingIslandId(999));
    }

    #[test]
    fn generated_island_is_larger_and_mostly_flat() {
        let island = FlyingIsland::generated(42);
        let tiles: Vec<_> = island.tiles().collect();
        let base_height = island.tile(TileCoord::new(0, 0)).unwrap().height;
        let base_height_tiles = tiles
            .iter()
            .filter(|tile| tile.height == base_height)
            .count();

        assert!(tiles.len() >= 220);
        assert!(base_height_tiles * 100 / tiles.len() >= 70);
    }

    #[test]
    fn generated_island_neighbor_heights_are_roadable() {
        let island = FlyingIsland::generated(42);

        for tile in island.tiles() {
            for neighbor in [
                TileCoord::new(tile.coord.x + 1, tile.coord.z),
                TileCoord::new(tile.coord.x, tile.coord.z + 1),
            ] {
                if let Some(neighbor) = island.tile(neighbor) {
                    assert!((tile.height - neighbor.height).abs() <= 1);
                }
            }
        }
    }

    #[test]
    fn generated_islands_have_varied_shapes() {
        let islands = FlyingIsland::generated_many(42);
        let mut aspect_deltas: Vec<_> = islands
            .iter()
            .take(80)
            .filter_map(|island| island.bounds())
            .map(|(min, max)| (max.x - min.x).abs_diff(max.z - min.z))
            .collect();
        aspect_deltas.sort_unstable();
        aspect_deltas.dedup();

        assert!(aspect_deltas.len() >= 6);
    }

    #[test]
    fn generated_islands_have_varied_distances_from_start() {
        let islands = FlyingIsland::generated_many(42);
        let mut distance_buckets: Vec<_> = islands
            .iter()
            .skip(1)
            .take(120)
            .filter_map(|island| island.bounds())
            .map(|(min, max)| {
                let center = TileCoord::new((min.x + max.x) / 2, (min.z + max.z) / 2);
                manhattan_distance(TileCoord::new(0, 0), center) / 20
            })
            .collect();
        distance_buckets.sort_unstable();
        distance_buckets.dedup();

        assert!(distance_buckets.len() >= 10);
    }
}
