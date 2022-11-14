use std::cmp::{max, min};
use std::collections::HashMap;
use rltk::RandomNumberGenerator;
use crate::map::Map;
use crate::rect::Rect;
use crate::{TileType};

pub fn snapshot (map: &Map) -> Map {
	let mut snapshot = map.clone();
	for v in snapshot.revealed_tiles.iter_mut() { *v = true; }
	for v in snapshot.visible_tiles.iter_mut() { *v = true; }
	snapshot
}

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

pub fn draw_corridor (map: &mut Map, x1: i32, y1: i32, x2: i32, y2: i32) {
	let mut x = x1;
	let mut y = y1;

	while x != x2 || y != y2 {
		if x < x2 { x += 1 }
		else if x > x2 { x -= 1 }
		else if y < y2 { y += 1 }
		else if y > y2 { y -= 1 }

		let idx = map.xy_idx(x, y);
		map.tiles[idx] = TileType::Floor;

		for y2 in y - 1 ..= y + 1 {
			for x2 in x - 1 ..= x + 1 {
				if x == x2 && y == y2 { continue }
				let idx = map.xy_idx(x2, y2);
				if map.tiles[idx] != TileType::Floor {
					map.tiles[idx] = TileType::Wall
				}
			}
		}
	}
}

pub fn apply_horizontal_tunnel (map: &mut Map, x1: i32, x2: i32, y: i32) {
	let lft = min(x1, x2);
	let rgt = max(x1, x2);

	draw_corridor(map, lft, y, rgt, y);
}

pub fn apply_vertical_tunnel (map: &mut Map, y1: i32, y2: i32, x: i32) {
	let top = min(y1, y2);
	let btm = max(y1, y2);

	draw_corridor(map, x, top, x, btm);
}

pub fn remove_unreachable_areas_returning_most_distant (map: &mut Map, start_idx: usize) -> usize {
	map.populate_blocked();

	let map_starts : Vec<usize> = vec![start_idx];
	let dijkstra_map = rltk::DijkstraMap::new(
		map.width, map.height,
		&map_starts, map,
		200.,
	);
	let mut exit_tile = (0, 0.0f32);

	for (i, tile) in map.tiles.iter_mut().enumerate() {
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

	exit_tile.0
}

pub fn generate_voronoi_spawn_regions (map: &Map, rng: &mut RandomNumberGenerator) -> HashMap<i32, Vec<usize>> {
	let mut noise = rltk::FastNoise::seeded(rng.roll_dice(1, 65536) as u64);
	noise.set_noise_type(rltk::NoiseType::Cellular);
	noise.set_frequency(0.08);
	noise.set_cellular_distance_function(rltk::CellularDistanceFunction::Natural);

	let mut noise_areas : HashMap<i32, Vec<usize>> = HashMap::new();

	for y in 1 .. map.height - 1 {
		for x in 1 .. map.width - 1 {
			let idx = map.xy_idx(x, y);
			if map.tiles[idx] != TileType::Floor { continue }

			let cell_value_f = noise.get_noise(x as f32, y as f32) * 10240.;
			let cell_value = cell_value_f as i32;

			if noise_areas.contains_key(&cell_value) {
				noise_areas.get_mut(&cell_value).unwrap().push(idx);
			} else {
				noise_areas.insert(cell_value, vec![idx]);
			}
		}
	}

	noise_areas
}