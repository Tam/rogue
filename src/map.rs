use rltk::{RGB, Rltk, Algorithm2D, Point, BaseMap, SmallVec, DistanceAlg};
use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use specs::{Entity};

// region: Rendering

#[derive(PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize, Debug)]
pub enum TileType {
	Void,
	Placeholder,
	Wall,
	Floor,
	DownStairs,
}

pub fn draw_map (map: &Map, ctx: &mut Rltk) {
	let mut x = 0;
	let mut y = 0;
	for (idx, tile) in map.tiles.iter().enumerate() {
		if map.revealed_tiles[idx] {
			let glyph;
			let mut fg;
			let mut bg = RGB::from(rltk::BLACK);

			match tile {
				TileType::Floor => {
					glyph = rltk::to_cp437('.');
					fg = RGB::from_f32(0.1, 0.4, 0.1);
				}
				TileType::Wall => {
					glyph = wall_glyph(&*map, x, y);
					fg = RGB::from_f32(0.1, 0.4, 0.1);
				}
				TileType::DownStairs => {
					glyph = rltk::to_cp437('▼');
					fg = RGB::named(rltk::WHEAT4);
				}
				TileType::Placeholder => {
					glyph = rltk::to_cp437('#');
					fg = RGB::named(rltk::SLATEGRAY);
				}
				TileType::Void => {
					glyph = 0;
					fg = RGB::new();
				}
			}

			if map.bloodstains.contains(&idx) {
				bg = RGB::named(rltk::DARK_RED);
			}

			if *tile != TileType::Void {
				if !map.visible_tiles[idx] {
					fg = fg.to_greyscale();
					if map.bloodstains.contains(&idx) {
						bg = RGB::from(rltk::DARKSLATEGREY);
					}
				}

				ctx.set(x, y, fg, bg, glyph);
			}
		}

		x += 1;
		if x > 79 {
			x = 0;
			y += 1;
		}
	}
}

fn is_revealed_and_wall (map: &Map, x: i32, y: i32) -> bool {
	if x < 0 || x > map.width - 1 || y < 0 || y > map.height - 1 {
		return false;
	}

	let idx = map.xy_idx(x, y);
	map.tiles[idx] == TileType::Wall// && map.revealed_tiles[idx]
}

fn wall_glyph (map: &Map, x: i32, y: i32) -> rltk::FontCharType {
	let mut mask : u8 = 0;

	if is_revealed_and_wall(map, x, y - 1) { mask += 1 }
	if is_revealed_and_wall(map, x, y + 1) { mask += 2 }
	if is_revealed_and_wall(map, x - 1, y) { mask += 4 }
	if is_revealed_and_wall(map, x + 1, y) { mask += 8 }

	match mask {
		0  => 9,   // Pillar because we can't see neighbors
		1  => 186, // Wall only to the north
		2  => 186, // Wall only to the south
		3  => 186, // Wall to the north and south
		4  => 205, // Wall only to the west
		5  => 188, // Wall to the north and west
		6  => 187, // Wall to the south and west
		7  => 185, // Wall to the north, south and west
		8  => 205, // Wall only to the east
		9  => 200, // Wall to the north and east
		10 => 201, // Wall to the south and east
		11 => 204, // Wall to the north, south and east
		12 => 205, // Wall to the east and west
		13 => 202, // Wall to the east, west, and south
		14 => 203, // Wall to the east, west, and north
		15 => 206, // ╬ Wall on all sides
		_  => 35,  // We missed one?
	}
}

// endregion

pub const MAP_WIDTH  : usize = 80;
pub const MAP_HEIGHT : usize = 43;
pub const MAP_SIZE   : usize = MAP_HEIGHT * MAP_WIDTH;

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Map {
	pub tiles          : Vec<TileType>,
	pub width          : i32,
	pub height         : i32,
	pub revealed_tiles : Vec<bool>,
	pub visible_tiles  : Vec<bool>,
	pub blocked        : Vec<bool>,
	pub depth          : i32,
	pub bloodstains    : HashSet<usize>,

	#[serde(skip_serializing)]
	#[serde(skip_deserializing)]
	pub tile_content   : Vec<Vec<Entity>>,
}

impl Map {

	pub fn new (
		width: i32,
		height: i32,
		depth: i32,
		fill_tile: Option<TileType>,
	) -> Map {
		let l = height as usize * width as usize;

		Map {
			tiles : vec![fill_tile.unwrap_or(TileType::Void); l],
			width,
			height,
			revealed_tiles: vec![false; l],
			visible_tiles: vec![false; l],
			blocked: vec![false; l],
			depth,
			bloodstains: HashSet::new(),
			tile_content: vec![Vec::new(); l],
		}
	}

	pub fn new_default (depth: i32) -> Map {
		Map::new(MAP_WIDTH as i32, MAP_HEIGHT as i32, depth, None)
	}

	pub fn xy_idx (&self, x: i32, y: i32) -> usize {
		(y as usize * self.width as usize) + x as usize
	}

	// prev: is_exit_valid
	fn is_walkable (&self, x: i32, y: i32) -> bool {
		if x < 1 || x > self.width - 1 || y < 1 || y > self.height - 1 {
			return false;
		}

		let idx = self.xy_idx(x, y);
		return !self.blocked[idx];
	}

	pub fn populate_blocked (&mut self) {
		for (i, tile) in self.tiles.iter_mut().enumerate() {
			self.blocked[i] = *tile == TileType::Wall || *tile == TileType::Void;
		}
	}

	pub fn clear_content_index(&mut self) {
		for content in self.tile_content.iter_mut() {
			content.clear();
		}
	}

	pub fn is_void_or_wall (&self, x: i32, y: i32) -> bool {
		let idx = self.xy_idx(x, y);
		self.tiles[idx] == TileType::Wall || self.tiles[idx] == TileType::Void
	}

}

impl Algorithm2D for Map {
	fn dimensions (&self) -> Point { Point::new(self.width, self.height) }
}

impl BaseMap for Map {
	fn is_opaque(&self, idx: usize) -> bool {
		self.tiles[idx] == TileType::Wall
	}

	fn get_available_exits(&self, idx: usize) -> SmallVec<[(usize, f32); 10]> {
		let mut exists = SmallVec::new();
		let x = idx as i32 % self.width;
		let y = idx as i32 / self.width;
		let w = self.width as usize;

		// Cardinal Directions
		if self.is_walkable(x - 1, y) { exists.push((idx - 1, 1.)) };
		if self.is_walkable(x + 1, y) { exists.push((idx + 1, 1.)) };
		if self.is_walkable(x, y - 1) { exists.push((idx - w, 1.)) };
		if self.is_walkable(x, y + 1) { exists.push((idx + w, 1.)) };

		// Diagonals
		if self.is_walkable(x - 1, y - 1) { exists.push(((idx - w) - 1, 1.45)) }
		if self.is_walkable(x + 1, y - 1) { exists.push(((idx - w) + 1, 1.45)) }
		if self.is_walkable(x - 1, y + 1) { exists.push(((idx + w) - 1, 1.45)) }
		if self.is_walkable(x + 1, y + 1) { exists.push(((idx + w) + 1, 1.45)) }

		return exists;
	}

	fn get_pathing_distance(&self, idx1: usize, idx2: usize) -> f32 {
		let w = self.width as usize;
		let p1 = Point::new(idx1 % w, idx1 / w);
		let p2 = Point::new(idx2 % w, idx2 / w);

		return DistanceAlg::Pythagoras.distance2d(p1, p2);
	}
}