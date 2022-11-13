use rltk::RandomNumberGenerator;
use specs::World;
use crate::map::Map;
use crate::map_builder::MapBuilder;
use crate::{MAP_HEIGHT, MAP_WIDTH, Position, spawner, TileType};
use crate::map_builder::common::apply_room_to_map;
use crate::rect::Rect;

pub struct BspDungeonBuilder {
	map: Map,
	starting_position: Position,
	depth: i32,
	rooms: Vec<Rect>,
	rects: Vec<Rect>,
	#[cfg(feature = "mapgen_visualiser")] history: Vec<Map>,
}

impl BspDungeonBuilder {
	#[allow(dead_code)]
	pub fn new (depth: i32) -> BspDungeonBuilder {
		BspDungeonBuilder {
			map: Map::new(MAP_WIDTH as i32, MAP_HEIGHT as i32, depth, None),
			starting_position: Position { x: 0, y: 0 },
			depth,
			rooms: Vec::new(),
			rects: Vec::new(),
			#[cfg(feature = "mapgen_visualiser")] history: Vec::new(),
		}
	}

	fn add_subrects (&mut self, rect: Rect) {
		let width = i32::abs(rect.x1 - rect.x2);
		let height = i32::abs(rect.y1 - rect.y2);
		let half_width = i32::max(width / 2, 1);
		let half_height = i32::max(height / 2, 1);

		self.rects.push(Rect::new(rect.x1, rect.y1, half_width, half_height));
		self.rects.push(Rect::new(rect.x1, rect.y1 + half_height, half_width, half_height));
		self.rects.push(Rect::new(rect.x1 + half_width, rect.y1, half_width, half_height));
		self.rects.push(Rect::new(rect.x1 + half_width, rect.y1 + half_height, half_width, half_height));

		/* Subdivide the rect!
		┌───────────┐        ┌─────┬─────┐
		│           │        │  1  │  2  │
		│     0     │   ->   ├─────┼─────┤
		│           │        │  3  │  4  │
		└───────────┘        └─────┴─────┘
		*/
	}

	fn get_random_rect (&mut self, rng: &mut RandomNumberGenerator) -> Rect {
		if self.rects.len() == 1 { return self.rects[0]; }
		let idx = (rng.roll_dice(1, self.rects.len() as i32) - 1) as usize;

		return self.rects[idx];
	}

	fn get_random_sub_rect (&self, rect: Rect, rng: &mut RandomNumberGenerator) -> Rect {
		let mut result = rect;
		let width = i32::abs(rect.x1 - rect.x2);
		let height = i32::abs(rect.y1 - rect.y2);

		let w = i32::max(3, rng.roll_dice(1, i32::min(width, 10)) - 1) + 1;
		let h = i32::max(3, rng.roll_dice(1, i32::min(height, 10)) - 1) + 1;

		result.x1 += rng.roll_dice(1, 6) - 1;
		result.y1 += rng.roll_dice(1, 6) - 1;
		result.x2 = result.x1 + w;
		result.y2 = result.y1 + h;

		return result;
	}

	fn is_possible (&self, rect: Rect) -> bool {
		let mut expanded = rect;
		expanded.x1 -= 2;
		expanded.x2 += 2;
		expanded.y1 -= 2;
		expanded.y2 += 1;

		let mut can_build = true;

		for y in expanded.y1 ..= expanded.y2 {
			for x in expanded.x1 ..= expanded.x2 {
				if x > self.map.width - 2 { can_build = false }
				if y > self.map.height - 2 { can_build = false }
				if x < 1 { can_build = false }
				if y < 1 { can_build = false }
				if can_build { can_build = self.map.is_void_or_wall(x, y) }
			}
		}

		return can_build;
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

impl MapBuilder for BspDungeonBuilder {
	fn get_map(&mut self) -> Map {
		self.map.clone()
	}

	fn get_starting_position(&mut self) -> Position {
		self.starting_position.clone()
	}

	fn build(&mut self) {
		let mut rng = RandomNumberGenerator::new();

		self.rects.clear();
		// Place the first, big, room
		self.rects.push(Rect::new(
			2, 2,
			self.map.width - 5,
			self.map.height - 5
		));

		// Divide the first room
		let first_room = self.rects[0];
		self.add_subrects(first_room);

		// Up to 240 times, get a random rect & divide it. If there's space for
		// a room, add one
		let mut n_rooms = 0;
		while n_rooms < 240 {
			let rect = self.get_random_rect(&mut rng);
			let candidate = self.get_random_sub_rect(rect, &mut rng);

			if self.is_possible(candidate) {
				apply_room_to_map(&mut self.map, &candidate);
				self.rooms.push(candidate);
				self.add_subrects(rect);
				#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
			}

			n_rooms += 1;
		}

		self.rooms.sort_by(|a, b| a.x1.cmp(&b.x1));

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
		self.starting_position = Position {
			x: start.0,
			y: start.1,
		};

		let stairs = self.rooms[self.rooms.len() - 1].center();
		let stairs_idx = self.map.xy_idx(stairs.0, stairs.1);
		self.map.tiles[stairs_idx] = TileType::DownStairs;

		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
	}

	fn spawn(&mut self, ecs: &mut World) {
		for room in self.rooms.iter().skip(1) {
			spawner::spawn_room(ecs, room, self.depth, &self.map);
		}

		#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
	}

	#[cfg(feature = "mapgen_visualiser")]
	fn get_name(&self) -> String { "BSP".to_string() }

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