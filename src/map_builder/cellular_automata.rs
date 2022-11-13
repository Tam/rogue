use std::collections::HashMap;
use rltk::RandomNumberGenerator;
use specs::World;
use crate::map::Map;
use crate::{MAP_HEIGHT, MAP_WIDTH, Position, spawner, TileType};
use crate::map_builder::MapBuilder;

pub struct CellularAutomataBuilder {
	map: Map,
	starting_position: Position,
	depth: i32,
	noise_areas: HashMap<i32, Vec<usize>>,
	#[cfg(feature = "mapgen_visualiser")] history: Vec<Map>,
}

impl CellularAutomataBuilder {
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

		// Find all tiles we can reach from the start pos
		self.map.populate_blocked();

		let map_starts : Vec<usize> = vec![start_idx];
		let dijkstra_map = rltk::DijkstraMap::new(
			self.map.width, self.map.height,
			&map_starts, &self.map,
			200.,
		);
		let mut exit_tile = (0, 0.0f32);

		for (i, tile) in self.map.tiles.iter_mut().enumerate() {
			if *tile != TileType::Floor { continue }

			let dist_to_start = dijkstra_map.map[i];

			if dist_to_start == f32::MAX {
				*tile = TileType::Wall;
			} else {
				if dist_to_start > exit_tile.1 {
					exit_tile.0 = i;
					exit_tile.1 = dist_to_start;
				}
			}
		}

		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();

		self.map.tiles[exit_tile.0] = TileType::DownStairs;
		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();

		// Build noise map for entity spawning
		let mut noise = rltk::FastNoise::seeded(rng.roll_dice(1, 65536) as u64);
		noise.set_noise_type(rltk::NoiseType::Cellular);
		noise.set_frequency(0.08);
		noise.set_cellular_distance_function(rltk::CellularDistanceFunction::Natural);

		for y in 1 .. self.map.height - 1 {
			for x in 1 .. self.map.width - 1 {
				let idx = self.map.xy_idx(x, y);
				if self.map.tiles[idx] != TileType::Floor { continue }

				let cell_value_f = noise.get_noise(x as f32, y as f32) * 10240.;
				let cell_value = cell_value_f as i32;

				if self.noise_areas.contains_key(&cell_value) {
					self.noise_areas.get_mut(&cell_value).unwrap().push(idx);
				} else {
					self.noise_areas.insert(cell_value, vec![idx]);
				}
			}
		}
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
		let mut snapshot = self.map.clone();
		for v in snapshot.revealed_tiles.iter_mut() { *v = true; }
		for v in snapshot.visible_tiles.iter_mut() { *v = true; }
		self.history.push(snapshot);
	}
}