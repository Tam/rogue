use std::collections::HashMap;
use rltk::RandomNumberGenerator;
use specs::World;
use crate::map::Map;
use crate::{MAP_HEIGHT, MAP_WIDTH, Position, spawner, TileType};
use crate::map_builder::common::{generate_voronoi_spawn_regions, remove_unreachable_areas_returning_most_distant, snapshot};
use crate::map_builder::MapBuilder;

#[derive(PartialEq, Copy, Clone)]
pub enum DLAAlgorithm {
	WalkInwards,
	WalkOutwards,
	CentralAttractor,
}

#[allow(dead_code)]
#[derive(PartialEq, Copy, Clone)]
pub enum DLASymmetry {
	None,
	Horizontal,
	Vertical,
	Both,
}

pub struct DLABuilder {
	map: Map,
	starting_position: Position,
	depth: i32,
	noise_areas: HashMap<i32, Vec<usize>>,
	algorithm: DLAAlgorithm,
	brush_size: i32,
	symmetry: DLASymmetry,
	floor_percent: f32,
	name: String,
	#[cfg(feature = "mapgen_visualiser")] history: Vec<Map>,
}

impl DLABuilder {
	#[allow(dead_code)]
	pub fn new (
		name: String,
		depth: i32,
		algorithm: DLAAlgorithm,
		brush_size: i32,
		symmetry: DLASymmetry,
	) -> DLABuilder {
		DLABuilder {
			map: Map::new(
				MAP_WIDTH as i32,
				MAP_HEIGHT as i32,
				depth,
				Some(TileType::Wall),
			),
			starting_position: Position { x: 0, y: 0 },
			depth,
			noise_areas: HashMap::new(),
			algorithm,
			brush_size,
			symmetry,
			floor_percent: 0.25,
			name,
			#[cfg(feature = "mapgen_visualiser")] history: Vec::new(),
		}
	}

	pub fn walk_inwards (depth: i32) -> DLABuilder {
		DLABuilder::new(
			"Walk Inwards".to_string(),
			depth,
			DLAAlgorithm::WalkInwards,
			1,
			DLASymmetry::None,
		)
	}

	pub fn walk_outwards (depth: i32) -> DLABuilder {
		DLABuilder::new(
			"Walk Outwards".to_string(),
			depth,
			DLAAlgorithm::WalkOutwards,
			2,
			DLASymmetry::None,
		)
	}

	pub fn central_attractor (depth: i32) -> DLABuilder {
		DLABuilder::new(
			"Central Attractor".to_string(),
			depth,
			DLAAlgorithm::CentralAttractor,
			2,
			DLASymmetry::None,
		)
	}

	pub fn insectoid (depth: i32) -> DLABuilder {
		DLABuilder::new(
			"Insectoid".to_string(),
			depth,
			DLAAlgorithm::CentralAttractor,
			2,
			DLASymmetry::Horizontal,
		)
	}

	fn paint (&mut self, x: i32, y: i32) {
		match self.symmetry {
			DLASymmetry::None => self.apply_paint(x, y),
			DLASymmetry::Horizontal => {
				let center_x = self.map.width / 2;
				if x == center_x { self.apply_paint(x, y) }
				else {
					let dist_x = i32::abs(center_x - x);
					self.apply_paint(center_x + dist_x, y);
					self.apply_paint(center_x - dist_x, y);
				}
			}
			DLASymmetry::Vertical => {
				let center_y = self.map.height / 2;
				if y == center_y { self.apply_paint(x, y) }
				else {
					let dist_y = i32::abs(center_y - y);
					self.apply_paint(x, center_y + dist_y);
					self.apply_paint(x, center_y - dist_y);
				}
			}
			DLASymmetry::Both => {
				let center_x = self.map.width / 2;
				let center_y = self.map.height / 2;
				if x == center_x && y == center_y { self.apply_paint(x, y) }
				else {
					let dist_x = i32::abs(center_x - x);
					self.apply_paint(center_x + dist_x, y);
					self.apply_paint(center_x - dist_x, y);
					let dist_y = i32::abs(center_y - y);
					self.apply_paint(x, center_y + dist_y);
					self.apply_paint(x, center_y - dist_y);
				}
			}
		}
	}

	fn apply_paint (&mut self, x: i32, y: i32) {
		if self.brush_size == 1 {
			let idx = self.map.xy_idx(x, y);
			self.map.tiles[idx] = TileType::Floor;
		} else {
			let half_brush_size = self.brush_size / 2;
			for brush_y in y - half_brush_size .. y + half_brush_size {
				for brush_x in x - half_brush_size .. x + half_brush_size {
					if brush_x > 1 && brush_x < self.map.width - 1
					&& brush_y > 1 && brush_y < self.map.height - 1 {
						let idx = self.map.xy_idx(brush_x, brush_y);
						self.map.tiles[idx] = TileType::Floor;
					}
				}
			}
		}
	}
}

impl MapBuilder for DLABuilder {
	fn get_map(&mut self) -> Map {
		self.map.clone()
	}

	fn get_starting_position(&mut self) -> Position {
		self.starting_position.clone()
	}

	fn build(&mut self) {
		let mut rng = RandomNumberGenerator::new();

		// Carve starting seed
		self.starting_position = Position {
			x: self.map.width / 2,
			y: self.map.height / 2,
		};
		let start_idx = self.map.xy_idx(self.starting_position.x, self.starting_position.y);
		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();

		self.map.tiles[start_idx] = TileType::Floor;
		self.map.tiles[start_idx - 1] = TileType::Floor;
		self.map.tiles[start_idx + 1] = TileType::Floor;
		self.map.tiles[start_idx - self.map.width as usize] = TileType::Floor;
		self.map.tiles[start_idx + self.map.width as usize] = TileType::Floor;

		// Random walker
		let total_tiles = self.map.width * self.map.height;
		let desired_floor_tiles = (self.floor_percent * total_tiles as f32) as usize;
		let mut floor_tile_count = self.map.tiles.iter().filter(|a| **a == TileType::Floor).count();

		#[cfg(feature = "mapgen_visualiser")] let mut i = 0;

		while floor_tile_count < desired_floor_tiles {
			match self.algorithm {
				DLAAlgorithm::WalkInwards => {
					let mut digger_x = rng.roll_dice(1, self.map.width - 3) + 1;
					let mut digger_y = rng.roll_dice(1, self.map.height - 3) + 1;
					let mut prev_x = digger_x;
					let mut prev_y = digger_y;
					let mut digger_idx = self.map.xy_idx(digger_x, digger_y);

					while self.map.tiles[digger_idx] == TileType::Wall {
						prev_x = digger_x;
						prev_y = digger_y;

						let stagger_direction = rng.roll_dice(1, 4);
						match stagger_direction {
							1 => { if digger_x > 2 { digger_x -= 1 } }
							2 => { if digger_x < self.map.width - 2 { digger_x += 1 } }
							3 => { if digger_y > 2 { digger_y -= 1 } }
							_ => { if digger_y < self.map.height - 2 { digger_y += 1 } }
						}

						digger_idx = self.map.xy_idx(digger_x, digger_y);
					}

					self.paint(prev_x, prev_y);
				}
				DLAAlgorithm::WalkOutwards => {
					let mut digger_x = self.starting_position.x;
					let mut digger_y = self.starting_position.y;
					let mut digger_idx = self.map.xy_idx(digger_x, digger_y);

					while self.map.tiles[digger_idx] == TileType::Floor {
						let stagger_direction = rng.roll_dice(1, 4);

						match stagger_direction {
							1 => { if digger_x > 2 { digger_x -= 1 } }
							2 => { if digger_x < self.map.width - 2 { digger_x += 1 } }
							3 => { if digger_y > 2 { digger_y -= 1 } }
							_ => { if digger_y < self.map.height - 2 { digger_y += 1 } }
						}

						digger_idx = self.map.xy_idx(digger_x, digger_y);
					}

					self.paint(digger_x, digger_y);
				}
				DLAAlgorithm::CentralAttractor => {
					let mut digger_x = rng.roll_dice(1, self.map.width - 3) + 1;
					let mut digger_y = rng.roll_dice(1, self.map.height - 3) + 1;
					let mut prev_x = digger_x;
					let mut prev_y = digger_y;
					let mut digger_idx = self.map.xy_idx(digger_x, digger_y);

					let mut path = rltk::line2d(
						rltk::LineAlg::Bresenham,
						rltk::Point::new(digger_x, digger_y),
						rltk::Point::new(self.starting_position.x, self.starting_position.y),
					);

					while self.map.tiles[digger_idx] == TileType::Wall && !path.is_empty() {
						prev_x = digger_x;
						prev_y = digger_y;
						digger_x = path[0].x;
						digger_y = path[0].y;
						path.remove(0);
						digger_idx = self.map.xy_idx(digger_x, digger_y);
					}

					self.paint(prev_x, prev_y);
				}
			}

			#[cfg(feature = "mapgen_visualiser")]
			{
				i += 1;
				if i % 50 == 0 { self.take_snapshot() }
			}

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
		format!("Drunkard Walk ({})", self.name)
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