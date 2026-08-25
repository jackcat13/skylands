use skylands_core::{BuildingKind, BuildingQuote, RoadPlacementQuote, RunState, TileCoord};

const COMMAND_BANNER_SECONDS: f32 = 1.5;

#[derive(Debug, Clone)]
pub struct AppUi {
    selected_tool: Tool,
    road_drag: Option<RoadDrag>,
    command_banner: Option<CommandBanner>,
}

impl Default for AppUi {
    fn default() -> Self {
        Self {
            selected_tool: Tool::Road,
            road_drag: None,
            command_banner: None,
        }
    }
}

impl AppUi {
    pub fn update(&mut self, frame_time: f32) {
        if let Some(banner) = &mut self.command_banner {
            banner.remaining_seconds -= frame_time;
            if banner.remaining_seconds <= 0.0 {
                self.command_banner = None;
            }
        }
    }

    pub fn selected_tool(&self) -> Tool {
        self.selected_tool
    }

    pub fn select_tool(&mut self, tool: Tool) {
        self.selected_tool = tool;
        self.road_drag = None;
    }

    pub fn start_road_drag(&mut self, coord: TileCoord) {
        self.road_drag = Some(RoadDrag::start(coord));
    }

    pub fn push_road_drag_to(&mut self, coord: TileCoord) {
        if let Some(drag) = &mut self.road_drag {
            drag.push_to(coord);
        }
    }

    pub fn finish_road_drag(&mut self) -> Option<Vec<TileCoord>> {
        self.road_drag.take().map(|drag| drag.tiles)
    }

    pub fn show_command_error(&mut self, text: String) {
        self.command_banner = Some(CommandBanner {
            text,
            remaining_seconds: COMMAND_BANNER_SECONDS,
        });
    }

    pub fn clear_command_banner(&mut self) {
        self.command_banner = None;
    }

    pub fn command_banner(&self) -> Option<&CommandBanner> {
        self.command_banner.as_ref()
    }

    fn road_drag_tiles(&self) -> Option<Vec<TileCoord>> {
        self.road_drag.as_ref().map(|drag| drag.tiles.clone())
    }
}

#[derive(Debug, Clone)]
pub struct CommandBanner {
    pub text: String,
    remaining_seconds: f32,
}

#[derive(Debug, Clone)]
struct RoadDrag {
    tiles: Vec<TileCoord>,
    current_hover: TileCoord,
}

impl RoadDrag {
    fn start(coord: TileCoord) -> Self {
        Self {
            tiles: vec![coord],
            current_hover: coord,
        }
    }

    fn push_to(&mut self, coord: TileCoord) {
        if self.current_hover == coord {
            return;
        }

        let mut current = self.current_hover;
        while current.x != coord.x {
            current = TileCoord::new(current.x + (coord.x - current.x).signum(), current.z);
            push_road_drag_step(&mut self.tiles, current);
        }

        while current.z != coord.z {
            current = TileCoord::new(current.x, current.z + (coord.z - current.z).signum());
            push_road_drag_step(&mut self.tiles, current);
        }

        self.current_hover = coord;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Road,
    Demolish,
    Building(BuildingKind),
}

pub const TOOLS: [Tool; 7] = [
    Tool::Road,
    Tool::Demolish,
    Tool::Building(BuildingKind::House),
    Tool::Building(BuildingKind::Farm),
    Tool::Building(BuildingKind::Workshop),
    Tool::Building(BuildingKind::Market),
    Tool::Building(BuildingKind::Monument),
];

#[derive(Debug, Clone)]
pub enum Preview {
    None,
    Building(BuildingQuote),
    Road(RoadPlacementQuote),
}

impl Preview {
    pub fn from_state(run: &RunState, ui: &AppUi, hovered_tile: Option<TileCoord>) -> Self {
        match (ui.selected_tool, hovered_tile) {
            (Tool::Building(kind), Some(origin)) => {
                let quote = run.quote_building(kind, origin);
                Self::Building(quote)
            }
            (Tool::Road, _) => {
                let tiles = ui
                    .road_drag_tiles()
                    .or_else(|| hovered_tile.map(|coord| vec![coord]));

                let Some(tiles) = tiles else {
                    return Self::None;
                };

                let quote = run.quote_roads(&tiles);
                Self::Road(quote)
            }
            _ => Self::None,
        }
    }

    pub fn quote_line(&self) -> Option<String> {
        match self {
            Self::None => None,
            Self::Building(quote) => Some(format!(
                "Building: {cost} SkyCoin, {validity}",
                cost = quote.cost,
                validity = quote.validity_text(),
            )),
            Self::Road(quote) => Some(format!(
                "Road: {cost} SkyCoin, {validity}",
                cost = quote.total_cost,
                validity = quote.validity_text(),
            )),
        }
    }
}

fn push_road_drag_step(tiles: &mut Vec<TileCoord>, coord: TileCoord) {
    if let Some(existing_index) = tiles.iter().position(|tile| *tile == coord) {
        tiles.truncate(existing_index + 1);
    } else {
        tiles.push(coord);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn road_drag_fills_manhattan_gaps_and_truncates_backtracking() {
        let mut drag = RoadDrag::start(TileCoord::new(0, 0));

        drag.push_to(TileCoord::new(3, 2));
        assert_eq!(
            drag.tiles,
            vec![
                TileCoord::new(0, 0),
                TileCoord::new(1, 0),
                TileCoord::new(2, 0),
                TileCoord::new(3, 0),
                TileCoord::new(3, 1),
                TileCoord::new(3, 2),
            ]
        );

        drag.push_to(TileCoord::new(1, 2));
        assert_eq!(
            drag.tiles,
            vec![
                TileCoord::new(0, 0),
                TileCoord::new(1, 0),
                TileCoord::new(2, 0),
                TileCoord::new(3, 0),
                TileCoord::new(3, 1),
                TileCoord::new(3, 2),
                TileCoord::new(2, 2),
                TileCoord::new(1, 2),
            ]
        );

        drag.push_to(TileCoord::new(2, 0));

        assert_eq!(
            drag.tiles,
            vec![
                TileCoord::new(0, 0),
                TileCoord::new(1, 0),
                TileCoord::new(2, 0),
            ]
        );
    }

    #[test]
    fn command_banner_expires_after_timeout() {
        let mut ui = AppUi::default();

        ui.show_command_error("Not enough SkyCoin".to_owned());
        ui.update(COMMAND_BANNER_SECONDS - 0.1);
        assert!(ui.command_banner().is_some());

        ui.update(0.11);
        assert!(ui.command_banner().is_none());
    }
}
