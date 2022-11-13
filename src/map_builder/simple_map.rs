use rltk::RandomNumberGenerator;
use specs::World;
use crate::map::Map;
use crate::map_builder::MapBuilder;
use crate::{MAP_HEIGHT, MAP_WIDTH, Position, spawner, TileType};
use crate::map_builder::common::{apply_horizontal_tunnel, apply_room_to_map, apply_vertical_tunnel};
use crate::rect::Rect;

pub struct SimpleMapBuilder {
	map: Map,
	starting_position: Position,
	depth: i32,
	rooms: Vec<Rect>,
	#[cfg(feature = "mapgen_visualiser")]
	history: Vec<Map>,
}

impl MapBuilder for SimpleMapBuilder {
	fn get_map(&mut self) -> Map {
		self.map.clone()
	}

	fn get_starting_position(&mut self) -> Position {
		self.starting_position.clone()
	}

	fn build(&mut self) {
		self.rooms_and_corridors();
	}

	fn spawn(&mut self, ecs: &mut World) {
		for room in self.rooms.iter().skip(1) {
			spawner::spawn_room(ecs, room, self.depth, &self.map);
		}
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

impl SimpleMapBuilder {
	pub fn new (depth: i32) -> SimpleMapBuilder {
		SimpleMapBuilder {
			map: Map::new(
				MAP_WIDTH as i32,
				MAP_HEIGHT as i32,
				depth,
				None,
			),
			starting_position: Position { x: 0, y: 0 },
			depth,
			rooms: Vec::new(),
			#[cfg(feature = "mapgen_visualiser")]
			history: Vec::new(),
		}
	}

	fn rooms_and_corridors (&mut self) {
		const MAX_ROOMS : i32 = 30;
		const MIN_SIZE  : i32 = 6;
		const MAX_SIZE  : i32 = 10;

		let mut rng = RandomNumberGenerator::new();

		'generateRooms: for _ in 0..MAX_ROOMS {
			let w = rng.range(MIN_SIZE, MAX_SIZE);
			let h = rng.range(MIN_SIZE, MAX_SIZE);

			let x = rng.roll_dice(1, MAP_WIDTH as i32 - w - 1) - 1;
			let y = rng.roll_dice(1, MAP_HEIGHT as i32 - h - 1) - 1;

			let new_room = Rect::new(x, y, w, h);

			for other_room in self.rooms.iter() {
				if new_room.intersect(other_room) {
					continue 'generateRooms;
				}
			}

			apply_room_to_map(&mut self.map, &new_room);
			self.rooms.push(new_room);
			#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
		}

		// Add tunnels
		for (i, room) in self.rooms.clone().iter().skip(1).enumerate() {
			let (new_x, new_y) = room.center();
			let (prev_x, prev_y) = self.rooms[i].center();

			// FIXME: Tunnel corners aren't being walled up
			if rng.range(0, 2) == 1 {
				apply_horizontal_tunnel(&mut self.map, prev_x, new_x, prev_y);
				apply_vertical_tunnel(&mut self.map, prev_y, new_y, new_x);
			} else {
				apply_vertical_tunnel(&mut self.map, prev_y, new_y, prev_x);
				apply_horizontal_tunnel(&mut self.map, prev_x, new_x, new_y);
			}

			#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
		}

		let stairs_pos = self.rooms[self.rooms.len() - 1].center();
		let stairs_idx = self.map.xy_idx(stairs_pos.0, stairs_pos.1);
		self.map.tiles[stairs_idx] = TileType::DownStairs;

		let start_pos = self.rooms[0].center();
		self.starting_position = Position {
			x: start_pos.0,
			y: start_pos.1,
		};
	}
}