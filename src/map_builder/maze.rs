use std::collections::HashMap;
use rltk::RandomNumberGenerator;
use specs::World;
use crate::map::Map;
use crate::{MAP_HEIGHT, MAP_WIDTH, Position, spawner, TileType};
use crate::map_builder::common::{generate_voronoi_spawn_regions, remove_unreachable_areas_returning_most_distant};
#[cfg(feature = "mapgen_visualiser")] use crate::map_builder::common::snapshot;
use crate::map_builder::MapBuilder;

const TOP    : usize = 0;
const RIGHT  : usize = 1;
const BOTTOM : usize = 2;
const LEFT   : usize = 3;

// Cell
// =========================================================================

#[derive(Copy, Clone)]
struct Cell {
	row     : i32,
	column  : i32,
	walls   : [bool; 4],
	visited : bool,
}

impl Cell {
	fn new (row: i32, column: i32) -> Cell {
		Cell {
			row,
			column,
			walls: [true, true, true, true],
			visited: false,
		}
	}

	fn remove_walls (&mut self, next: &mut Cell) {
		let x = self.column - next.column;
		let y = self.row - next.row;

		if x == 1 {
			self.walls[LEFT] = false;
			next.walls[RIGHT] = false;
		} else if x == -1 {
			self.walls[RIGHT] = false;
			next.walls[LEFT] = false;
		} else if y == 1 {
			self.walls[TOP] = false;
			next.walls[BOTTOM] = false;
		} else if y == -1 {
			self.walls[BOTTOM] = false;
			next.walls[TOP] = false;
		}
	}
}

// Grid
// =========================================================================

struct Grid<'a> {
	width     : i32,
	height    : i32,
	cells     : Vec<Cell>,
	backtrace : Vec<usize>,
	current   : usize,
	rng       : &'a mut RandomNumberGenerator,
}

impl<'a> Grid<'a> {
	fn new (width: i32, height: i32, rng: &mut RandomNumberGenerator) -> Grid {
		let mut grid = Grid {
			width,
			height,
			cells: Vec::new(),
			backtrace: Vec::new(),
			current: 0,
			rng,
		};

		for row in 0..height {
			for column in 0..width {
				grid.cells.push(Cell::new(row, column));
			}
		}

		grid
	}

	fn calculate_index (&self, row: i32, column: i32) -> i32 {
		if row < 0 || column < 0 || column > self.width - 1 || row > self.height - 1 {
			-1
		} else {
			column + (row * self.width)
		}
	}

	fn get_available_neighbours (&self) -> Vec<usize> {
		let mut neighbours : Vec<usize> = Vec::new();
		let current_row = self.cells[self.current].row;
		let current_column = self.cells[self.current].column;

		let neighbour_indices : [i32; 4] = [
			self.calculate_index(current_row - 1, current_column),
			self.calculate_index(current_row, current_column + 1),
			self.calculate_index(current_row + 1, current_column),
			self.calculate_index(current_row, current_column - 1),
		];

		for i in neighbour_indices.iter() {
			if *i != -1 && !self.cells[*i as usize].visited {
				neighbours.push(*i as usize);
			}
		}

		neighbours
	}

	fn find_next_cell (&mut self) -> Option<usize> {
		let neighbours = self.get_available_neighbours();

		if !neighbours.is_empty() {
			return if neighbours.len() == 1 {
				Some(neighbours[0])
			} else {
				Some(neighbours[(self.rng.roll_dice(1, neighbours.len() as i32) - 1) as usize])
			}
		}

		None
	}

	fn generate_maze (&mut self, generator: &mut MazeBuilder) {
		#[cfg(feature = "mapgen_visualiser")]
		let mut i = 0;

		loop {
			self.cells[self.current].visited = true;
			let next = self.find_next_cell();

			match next {
				Some(next) => {
					self.cells[next].visited = true;
					self.backtrace.push(self.current);

					//      lower part        higher part
					//  /                \ /               \
					// | ------cell1----- | -----cell2----- |

					let (lower_part, higher_part) = self.cells.split_at_mut(
						std::cmp::max(self.current, next)
					);

					let cell1 = &mut lower_part[std::cmp::min(self.current, next)];
					let cell2 = &mut higher_part[0];

					cell1.remove_walls(cell2);
					self.current = next;
				}
				None => {
					if self.backtrace.is_empty() { break }

					self.current = self.backtrace[0];
					self.backtrace.remove(0);
				}
			}

			#[cfg(feature = "mapgen_visualiser")]
			{
				if i % 50 == 0 {
					self.copy_to_map(&mut generator.map);
					generator.take_snapshot();
				}
				i += 1;
			}
		}

		self.copy_to_map(&mut generator.map);
	}

	fn copy_to_map (&self, map: &mut Map) {
		for i in map.tiles.iter_mut() { *i = TileType::Wall }

		for cell in self.cells.iter() {
			let x = cell.column + 1;
			let y = cell.row + 1;
			let idx = map.xy_idx(x * 2, y * 2);

			map.tiles[idx] = TileType::Floor;
			if !cell.walls[TOP]    { map.tiles[idx - map.width as usize] = TileType::Floor }
			if !cell.walls[RIGHT]  { map.tiles[idx + 1] = TileType::Floor }
			if !cell.walls[BOTTOM] { map.tiles[idx + map.width as usize] = TileType::Floor }
			if !cell.walls[LEFT]   { map.tiles[idx - 1] = TileType::Floor }
		}
	}
}

// Builder
// =========================================================================

pub struct MazeBuilder {
	map: Map,
	starting_position: Position,
	depth: i32,
	noise_areas: HashMap<i32, Vec<usize>>,
	#[cfg(feature = "mapgen_visualiser")] history: Vec<Map>,
}

impl MazeBuilder {
	#[allow(dead_code)]
	pub fn new (depth: i32) -> MazeBuilder {
		MazeBuilder {
			map: Map::new(
				MAP_WIDTH as i32,
				MAP_HEIGHT as i32,
				depth,
				None,
			),
			starting_position: Position { x: 0, y: 0 },
			depth,
			noise_areas: HashMap::new(),
			#[cfg(feature = "mapgen_visualiser")] history: Vec::new(),
		}
	}
}

impl MapBuilder for MazeBuilder {
	fn get_map(&mut self) -> Map {
		self.map.clone()
	}

	fn get_starting_position(&mut self) -> Position {
		self.starting_position.clone()
	}

	fn build(&mut self) {
		let mut rng = RandomNumberGenerator::new();

		let mut grid = Grid::new(
			(self.map.width / 2) - 2,
			(self.map.height / 2) - 2,
			&mut rng,
		);
		grid.generate_maze(self);

		self.starting_position = Position { x: 2, y: 2 };
		let start_idx = self.map.xy_idx(
			self.starting_position.x,
			self.starting_position.y,
		);

		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();

		let exit_tile = remove_unreachable_areas_returning_most_distant(&mut self.map, start_idx);
		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();

		self.map.tiles[exit_tile] = TileType::DownStairs;
		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();

		self.noise_areas = generate_voronoi_spawn_regions(&self.map, &mut rng);
	}

	fn spawn(&mut self, ecs: &mut World) {
		for area in self.noise_areas.iter() {
			spawner::spawn_region(ecs, area.1, self.depth, &self.map);
		}
	}

	#[cfg(feature = "mapgen_visualiser")]
	fn get_name(&self) -> String {
		"Maze".to_string()
	}

	#[cfg(feature = "mapgen_visualiser")]
	fn get_snapshot_history(&self) -> Vec<Map> {
		self.history.clone()
	}

	#[cfg(feature = "mapgen_visualiser")]
	fn take_snapshot(&mut self) {
		self.history.push(snapshot(&self.map))
	}
}