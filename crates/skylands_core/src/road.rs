use crate::world::TileCoord;
use serde::{Deserialize, Serialize};

pub const ISLAND_ROAD_COST: i64 = 2;
pub const SKY_ROAD_COST: i64 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Road {
    pub coord: TileCoord,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadKind {
    Island,
    Sky,
}

impl RoadKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Island => "Island",
            Self::Sky => "Sky",
        }
    }
}

pub(crate) fn roads_can_connect(left: Road, right: Road) -> bool {
    left.coord.manhattan_distance_to(right.coord) == 1 && (left.height - right.height).abs() <= 1
}
