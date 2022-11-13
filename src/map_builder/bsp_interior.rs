use rltk::RandomNumberGenerator;
use specs::World;
use crate::map::Map;
use crate::{MAP_HEIGHT, MAP_WIDTH, Position, spawner, TileType};
use crate::map_builder::MapBuilder;
use crate::rect::Rect;

const MIN_ROOM_SIZE : i32 = 8;

pub struct BspInteriorBuilder {
	map: Map,
	starting_position: Position,
	depth: i32,
	rooms: Vec<Rect>,
	rects: Vec<Rect>,
	#[cfg(feature = "mapgen_visualiser")] history: Vec<Map>,
}

impl BspInteriorBuilder {
	#[allow(dead_code)]
	pub fn new (depth: i32) -> BspInteriorBuilder {
		BspInteriorBuilder {
			map: Map::new(
				MAP_WIDTH as i32,
				MAP_HEIGHT as i32,
				depth,
				Some(TileType::Wall),
			),
			starting_position: Position { x: 0, y: 0 },
			depth,
			rooms: Vec::new(),
			rects: Vec::new(),
			#[cfg(feature = "mapgen_visualiser")] history: Vec::new(),
		}
	}

	fn add_subrects (&mut self, rect: Rect, rng: &mut RandomNumberGenerator) {
		// Remove last rect from list
		if !self.rects.is_empty() {
			self.rects.remove(self.rects.len() - 1);
		}

		// Calc boundaries
		let width  = rect.x2 - rect.x1;
		let height = rect.y2 - rect.y1;
		let half_width  = width / 2;
		let half_height = height / 2;

		let split = rng.roll_dice(1, 4);

		if split <= 2 { // Horizontal Split
			let h1 = Rect::new(rect.x1, rect.y1, half_width - 1, height);
			self.rects.push(h1);
			if half_width > MIN_ROOM_SIZE { self.add_subrects(h1, rng) }

			let h2 = Rect::new(rect.x1 + half_width, rect.y1, half_width, height);
			self.rects.push(h2);
			if half_width > MIN_ROOM_SIZE { self.add_subrects(h2, rng) }
		} else { // Vertical Split
			let v1 = Rect::new(rect.x1, rect.y1, width, half_height - 1);
			self.rects.push(v1);
			if half_height > MIN_ROOM_SIZE { self.add_subrects(v1, rng) }

			let v2 = Rect::new(rect.x1, rect.y1 + half_height, width, half_height);
			self.rects.push(v2);
			if half_height > MIN_ROOM_SIZE { self.add_subrects(v2, rng) }
		}
	}

	fn draw_corridor (&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
		let mut x = x1;
		let mut y = y1;

		while x != x2 || y != y2 {
			if x < x2 { x += 1 }
			else if x > x2 { x -= 1 }
			else if y < y2 { y += 1 }
			else if y > y2 { y -= 1 }

			let idx = self.map.xy_idx(x, y);
			self.map.tiles[idx] = TileType::Floor;

			for y2 in y - 1 ..= y + 1 {
				for x2 in x - 1 ..= x + 1 {
					if x == x2 && y == y2 { continue }
					let idx = self.map.xy_idx(x2, y2);
					if self.map.tiles[idx] != TileType::Floor {
						self.map.tiles[idx] = TileType::Wall
					}
				}
			}
		}
	}

}

impl MapBuilder for BspInteriorBuilder {
	fn get_map(&mut self) -> Map {
		self.map.clone()
	}

	fn get_starting_position(&mut self) -> Position {
		self.starting_position.clone()
	}

	fn build(&mut self) {
		let mut rng = RandomNumberGenerator::new();

		self.rects.clear();
		self.rects.push(Rect::new(
			1, 1,
			self.map.width - 2,
			self.map.height - 2,
		));

		let first_room = self.rects[0];
		self.add_subrects(first_room, &mut rng);

		let rooms = self.rects.clone();
		for r in rooms.iter() {
			let room = *r;
			self.rooms.push(room);

			for y in room.y1 .. room.y2 {
				for x in room.x1 .. room.x2 {
					let idx = self.map.xy_idx(x, y);
					if idx > 0 && idx < ((self.map.width * self.map.height) - 1) as usize {
						self.map.tiles[idx] = TileType::Floor;
					}
				}
			}

			#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
		}

		for i in 0..self.rooms.len() - 1 {
			let room = self.rooms[i];
			let next_room = self.rooms[i + 1];

			let start_x = room.x1 + (rng.roll_dice(1, i32::abs(room.x1 - room.x2))-1);
			let start_y = room.y1 + (rng.roll_dice(1, i32::abs(room.y1 - room.y2))-1);
			let end_x = next_room.x1 + (rng.roll_dice(1, i32::abs(next_room.x1 - next_room.x2))-1);
			let end_y = next_room.y1 + (rng.roll_dice(1, i32::abs(next_room.y1 - next_room.y2))-1);

			self.draw_corridor(start_x, start_y, end_x, end_y);
			#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
		}

		let start = self.rooms[0].center();
		self.starting_position = Position { x: start.0, y: start.1 };

		let stairs = self.rooms[self.rooms.len() - 1].center();
		let stairs_idx = self.map.xy_idx(stairs.0, stairs.1);
		self.map.tiles[stairs_idx] = TileType::DownStairs;

		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
	}

	fn spawn(&mut self, ecs: &mut World) {
		for room in self.rooms.iter().skip(1) {
			spawner::spawn_room(ecs, &room, self.depth, &self.map);
		}

		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
	}

	#[cfg(feature = "mapgen_visualiser")]
	fn get_name(&self) -> String {
		"BSP Interior".to_string()
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