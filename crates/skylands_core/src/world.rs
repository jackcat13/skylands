use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
    tiles: Vec<IslandTile>,
}

impl FlyingIsland {
    pub fn generated(seed: u64) -> Self {
        let mut tiles = Vec::new();

        for x in -5_i32..=5 {
            for z in -5_i32..=5 {
                let distance = x.abs() + z.abs();
                let noise = hash_height(seed, x, z);
                let within_shape = distance <= 6 || (distance <= 8 && noise % 4 != 0);

                if within_shape {
                    let height = (noise % 3) as i32;
                    let coord = TileCoord::new(x, z);
                    upsert_tile(&mut tiles, IslandTile { coord, height });
                }
            }
        }

        for coord in [
            TileCoord::new(0, 0),
            TileCoord::new(1, 0),
            TileCoord::new(0, 1),
            TileCoord::new(1, 1),
        ] {
            upsert_tile(&mut tiles, IslandTile { coord, height: 1 });
        }

        tiles.sort_by_key(|tile| tile.coord);

        Self {
            id: FlyingIslandId(0),
            tiles,
        }
    }

    pub fn tile(&self, coord: TileCoord) -> Option<&IslandTile> {
        self.tiles.iter().find(|tile| tile.coord == coord)
    }

    pub fn tiles(&self) -> impl Iterator<Item = &IslandTile> {
        self.tiles.iter()
    }
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

        for coord in [
            TileCoord::new(0, 0),
            TileCoord::new(1, 0),
            TileCoord::new(0, 1),
            TileCoord::new(1, 1),
        ] {
            assert_eq!(island.tile(coord).map(|tile| tile.height), Some(1));
        }
    }
}
