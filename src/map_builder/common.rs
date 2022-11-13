use std::cmp::{max, min};
use crate::map::Map;
use crate::rect::Rect;
use crate::TileType;

pub fn apply_room_to_map (map: &mut Map, room: &Rect) {
	let top = room.y1;
	let btm = room.y2 + 1;
	let lft = room.x1;
	let rgt = room.x2 + 1;

	for y in top ..= btm {
		for x in lft ..= rgt {
			let idx = map.xy_idx(x, y);
			if x == lft || x == rgt || y == top || y == btm {
				map.tiles[idx] = TileType::Wall;
			} else {
				map.tiles[idx] = TileType::Floor;
			}
		}
	}
}

pub fn apply_horizontal_tunnel (map: &mut Map, x1: i32, x2: i32, y: i32) {
	let top = y - 1;
	let btm = y + 1;

	let lft = min(x1, x2);
	let rgt = max(x1, x2);

	for y1 in top ..= btm {
		for x in lft ..= rgt {
			let idx = map.xy_idx(x, y1);
			if idx > 0 && idx < map.width as usize * map.height as usize {
				if map.tiles[idx as usize] == TileType::Floor {
					continue;
				}

				if y1 == top || y1 == btm {
					map.tiles[idx as usize] = TileType::Wall;
				} else {
					map.tiles[idx as usize] = TileType::Floor;
				}
			}
		}
	}
}

pub fn apply_vertical_tunnel (map: &mut Map, y1: i32, y2: i32, x: i32) {
	let top = min(y1, y2);
	let btm = max(y1, y2);
	let lft = x - 1;
	let rgt = x + 1;

	for x1 in lft ..= rgt {
		for y in top..=btm {
			let idx = map.xy_idx(x1, y);
			if idx > 0 && idx < map.width as usize * map.height as usize {
				if map.tiles[idx as usize] == TileType::Floor {
					continue;
				}

				if x1 == lft || x1 == rgt {
					map.tiles[idx as usize] = TileType::Wall;
				} else {
					map.tiles[idx as usize] = TileType::Floor;
				}
			}
		}
	}
}