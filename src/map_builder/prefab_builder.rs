use specs::World;
use crate::map::Map;
#[cfg(feature = "mapgen_visualiser")]
use crate::map_builder::common::snapshot;
use crate::map_builder::MapBuilder;
use crate::{Position, TileType};

#[allow(dead_code)]
#[derive(PartialEq, Clone)]
pub enum PrefabMode {
	RexLevel { template: &'static str }
}

pub struct PrefabBuilder {
	map: Map,
	starting_position: Position,
	depth: i32,
	mode: PrefabMode,
	#[cfg(feature = "mapgen_visualiser")] history: Vec<Map>,
}

impl PrefabBuilder {
	pub fn new (depth: i32) -> PrefabBuilder {
		PrefabBuilder {
			map: Map::new_default(depth),
			starting_position: Position { x: 0, y: 0 },
			depth,
			mode: PrefabMode::RexLevel { template: "../resources/wfc-demo1.xp" },
			#[cfg(feature = "mapgen_visualiser")] history: Vec::new(),
		}
	}

	#[allow(dead_code)]
	fn load_rex_map (&mut self, path: &str) {
		let xp_file = rltk::rex::XpFile::from_resource(path).unwrap();

		for layer in &xp_file.layers {
			for y in 0..layer.height {
				for x in 0..layer.width {
					if x > self.map.width as usize
					|| y > self.map.height as usize { continue }

					let cell = layer.get(x, y).unwrap();
					let idx = self.map.xy_idx(x as i32, y as i32);

					self.map.tiles[idx] = match (cell.ch as u8) as char {
						' ' => TileType::Floor, // Space
						'#' => TileType::Wall,  // Hash
						 c  => panic!("Unknown REX map character: {}", c),
					}
				}
			}
		}
	}
}

impl MapBuilder for PrefabBuilder {
	fn get_map(&mut self) -> Map {
		self.map.clone()
	}

	fn get_starting_position(&mut self) -> Position {
		self.starting_position.clone()
	}

	fn build(&mut self) {
		match self.mode {
			PrefabMode::RexLevel {template} => self.load_rex_map(&template),
		}

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
		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
	}

	fn spawn(&mut self, ecs: &mut World) {
		// todo!()
	}

	#[cfg(feature = "mapgen_visualiser")]
	fn get_name(&self) -> String {
		"Prefab".to_string()
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
