use std::collections::HashMap;
use rltk::RandomNumberGenerator;
use specs::World;
use crate::map::Map;
use crate::{MAP_HEIGHT, MAP_WIDTH, Position, spawner, TileType};
use crate::map_builder::common::{generate_voronoi_spawn_regions, remove_unreachable_areas_returning_most_distant, snapshot};
use crate::map_builder::MapBuilder;

#[allow(dead_code)]
pub enum DrunkSpawnMode {
	StartingPoint,
	Random,
}

pub struct DrunkardSettings {
	pub spawn_mode: DrunkSpawnMode,
	pub lifetime: i32,
	pub floor_percent: f32,
	#[cfg(feature = "mapgen_visualiser")] pub name: String,
}

pub struct DrunkardWalkBuilder {
	map: Map,
	starting_position: Position,
	depth: i32,
	noise_areas: HashMap<i32, Vec<usize>>,
	settings: DrunkardSettings,
	#[cfg(feature = "mapgen_visualiser")] history: Vec<Map>,
}

impl DrunkardWalkBuilder {
	#[allow(dead_code)]
	pub fn new (depth: i32, settings: DrunkardSettings) -> DrunkardWalkBuilder {
		DrunkardWalkBuilder {
			map: Map::new(
				MAP_WIDTH as i32,
				MAP_HEIGHT as i32,
				depth,
				Some(TileType::Wall),
			),
			starting_position: Position { x: 0, y: 0 },
			depth,
			noise_areas: HashMap::new(),
			settings,
			#[cfg(feature = "mapgen_visualiser")] history: Vec::new(),
		}
	}

	#[allow(dead_code)]
	pub fn open_area (depth: i32) -> DrunkardWalkBuilder {
		DrunkardWalkBuilder::new(depth, DrunkardSettings {
			spawn_mode: DrunkSpawnMode::StartingPoint,
			lifetime: 400,
			floor_percent: 0.5,
			#[cfg(feature = "mapgen_visualiser")] name: "Open Area".to_string(),
		})
	}

	#[allow(dead_code)]
	pub fn open_halls (depth: i32) -> DrunkardWalkBuilder {
		DrunkardWalkBuilder::new(depth, DrunkardSettings {
			spawn_mode: DrunkSpawnMode::Random,
			lifetime: 400,
			floor_percent: 0.5,
			#[cfg(feature = "mapgen_visualiser")] name: "Open Halls".to_string(),
		})
	}

	#[allow(dead_code)]
	pub fn winding_passages (depth: i32) -> DrunkardWalkBuilder {
		DrunkardWalkBuilder::new(depth, DrunkardSettings {
			spawn_mode: DrunkSpawnMode::StartingPoint,
			lifetime: 100,
			floor_percent: 0.4,
			#[cfg(feature = "mapgen_visualiser")] name: "Winding Passages".to_string(),
		})
	}
}

impl MapBuilder for DrunkardWalkBuilder {
	fn get_map(&mut self) -> Map {
		self.map.clone()
	}

	fn get_starting_position(&mut self) -> Position {
		self.starting_position.clone()
	}

	fn build(&mut self) {
		let mut rng = RandomNumberGenerator::new();

		// Start at center
		self.starting_position = Position {
			x: self.map.width / 2,
			y: self.map.height / 2,
		};
		let start_idx = self.map.xy_idx(
			self.starting_position.x,
			self.starting_position.y,
		);

		// Drunkard
		self.map.tiles[start_idx] = TileType::Floor;

		let total_tiles = self.map.width * self.map.height;
		let desired_floor_tiles = (self.settings.floor_percent * total_tiles as f32) as usize;
		let mut floor_tile_count = self.map.tiles.iter().filter(|a| **a == TileType::Floor).count();
		let mut digger_count = 0;

		while floor_tile_count < desired_floor_tiles {
			#[cfg(feature = "mapgen_visualiser")] let mut did_something = false;
			let mut drunk_x;
			let mut drunk_y;
			let mut drunk_life = self.settings.lifetime;

			match self.settings.spawn_mode {
				DrunkSpawnMode::StartingPoint => {
					drunk_x = self.starting_position.x;
					drunk_y = self.starting_position.y;
				}
				DrunkSpawnMode::Random => {
					if digger_count == 0 {
						drunk_x = self.starting_position.x;
						drunk_y = self.starting_position.y;
					} else {
						drunk_x = rng.roll_dice(1, self.map.width - 3) + 1;
						drunk_y = rng.roll_dice(1, self.map.height - 3) + 1;
					}
				}
			}

			while drunk_life > 0 {
				let drunk_idx = self.map.xy_idx(drunk_x, drunk_y);

				if self.map.tiles[drunk_idx] == TileType::Wall {
					#[cfg(feature = "mapgen_visualiser")]
					{
						did_something = true;
						self.map.tiles[drunk_idx] = TileType::Placeholder;
					}
					#[cfg(not(feature = "mapgen_visualiser"))]
					{ self.map.tiles[drunk_idx] = TileType::Floor; }
				}

				let stagger_direction = rng.roll_dice(1, 4);
				match stagger_direction {
					1 => { if drunk_x > 2 { drunk_x -= 1 } },
					2 => { if drunk_x < self.map.width - 2 { drunk_x += 1 } },
					3 => { if drunk_y > 2 { drunk_y -= 1 } },
					_ => { if drunk_y < self.map.height - 2 { drunk_y += 1 } },
				}

				drunk_life -= 1;
			}

			#[cfg(feature = "mapgen_visualiser")]
			if did_something {
				self.take_snapshot();
				for t in self.map.tiles.iter_mut() {
					if *t == TileType::Placeholder {
						*t = TileType::Floor;
					}
				}
			}

			digger_count += 1;
			floor_tile_count = self.map.tiles.iter().filter(|a| **a == TileType::Floor).count();
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
		format!("Drunkard Walk ({})", self.settings.name)
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