use std::collections::HashMap;
use rltk::RandomNumberGenerator;
use specs::World;
use crate::map::Map;
use crate::{MAP_HEIGHT, MAP_WIDTH, Position, spawner, TileType};
use crate::map_builder::common::{generate_voronoi_spawn_regions, remove_unreachable_areas_returning_most_distant, snapshot};
use crate::map_builder::MapBuilder;

pub struct CellularAutomataBuilder {
	map: Map,
	starting_position: Position,
	depth: i32,
	noise_areas: HashMap<i32, Vec<usize>>,
	#[cfg(feature = "mapgen_visualiser")] history: Vec<Map>,
}

impl CellularAutomataBuilder {
	#[allow(dead_code)]
	pub fn new (depth: i32) -> CellularAutomataBuilder {
		CellularAutomataBuilder {
			map: Map::new(
				MAP_WIDTH as i32,
				MAP_HEIGHT as i32,
				depth,
				Some(TileType::Wall),
			),
			starting_position: Position { x: 0, y: 0 },
			depth,
			noise_areas: HashMap::new(),
			#[cfg(feature = "mapgen_visualiser")] history: Vec::new(),
		}
	}
}

impl MapBuilder for CellularAutomataBuilder {
	fn get_map(&mut self) -> Map {
		self.map.clone()
	}

	fn get_starting_position(&mut self) -> Position {
		self.starting_position.clone()
	}

	fn build(&mut self) {
		let mut rng = RandomNumberGenerator::new();

		for y in 1 .. self.map.height - 1 {
			for x in 1 .. self.map.width - 1 {
				let roll = rng.roll_dice(1, 100);
				let idx = self.map.xy_idx(x, y);
				self.map.tiles[idx] =
					if roll > 55 { TileType::Floor }
					else { TileType::Wall }
			}
		}

		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();

		for _i in 0 .. 15 {
			let mut new_tiles = self.map.tiles.clone();

			for y in 1 .. self.map.height - 1 {
				for x in 1..self.map.width - 1 {
					let idx = self.map.xy_idx(x, y);
					let mut neighbours = 0;

					if self.map.tiles[idx - 1] == TileType::Wall { neighbours += 1 }
					if self.map.tiles[idx + 1] == TileType::Wall { neighbours += 1 }
					if self.map.tiles[idx - self.map.width as usize] == TileType::Wall { neighbours += 1; }
					if self.map.tiles[idx + self.map.width as usize] == TileType::Wall { neighbours += 1; }
					if self.map.tiles[idx - (self.map.width as usize - 1)] == TileType::Wall { neighbours += 1; }
					if self.map.tiles[idx - (self.map.width as usize + 1)] == TileType::Wall { neighbours += 1; }
					if self.map.tiles[idx + (self.map.width as usize - 1)] == TileType::Wall { neighbours += 1; }
					if self.map.tiles[idx + (self.map.width as usize + 1)] == TileType::Wall { neighbours += 1; }

					new_tiles[idx] =
						if neighbours > 4 || neighbours == 0 { TileType::Wall }
						else { TileType::Floor }
				}
			}

			self.map.tiles = new_tiles.clone();
			#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
		}

		// Find starting pos (start in center & walk left until floor tile)
		self.starting_position = Position {
			x: self.map.width / 2,
			y: self.map.height / 2,
		};
		let mut start_idx = self.map.xy_idx(
			self.starting_position.x,
			self.starting_position.y,
		);

		while self.map.tiles[start_idx] != TileType::Floor {
			self.starting_position.x -= 1;
			start_idx = self.map.xy_idx(
				self.starting_position.x,
				self.starting_position.y,
			);
		}

		// Get all walkable tiles (fill holes)
		let exit_idx = remove_unreachable_areas_returning_most_distant(
			&mut self.map,
			start_idx,
		);
		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();

		self.map.tiles[exit_idx] = TileType::DownStairs;
		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();

		// Build noise map for entity spawning
		self.noise_areas = generate_voronoi_spawn_regions(
			&self.map,
			&mut rng,
		);
	}

	fn spawn(&mut self, ecs: &mut World) {
		for area in self.noise_areas.iter() {
			spawner::spawn_region(ecs, area.1, self.depth, &self.map);
		}
	}

	#[cfg(feature = "mapgen_visualiser")]
	fn get_name(&self) -> String {
		"Cellular Automata".to_string()
	}

	#[cfg(feature = "mapgen_visualiser")]
	fn get_snapshot_history(&self) -> Vec<Map> {
		self.history.clone()
	}
	#[cfg(feature = "mapgen_visualiser")]
	fn take_snapshot(&mut self) {
		self.history.push(snapshot(&self.map));
	}
}