use crate::world::TileCoord;
use serde::{Deserialize, Serialize};

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

impl BuildingKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::CityCore => "City Core",
            Self::House => "House",
            Self::Farm => "Farm",
            Self::Workshop => "Workshop",
            Self::Market => "Market",
            Self::Monument => "Monument",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Building {
    pub id: BuildingId,
    pub kind: BuildingKind,
    pub origin: TileCoord,
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
