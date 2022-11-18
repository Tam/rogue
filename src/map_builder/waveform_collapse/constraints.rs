use std::collections::HashSet;
use crate::map::Map;
use crate::map_builder::waveform_collapse::common::{MapChunk, tile_idx_in_chunk};
use crate::TileType;

pub fn build_patterns (
	map: &Map,
	chunk_size: i32,
	include_flipping: bool,
	dedupe: bool,
) -> Vec<Vec<TileType>> {
	let chunks_x = map.width / chunk_size;
	let chunks_y = map.height / chunk_size;
	let mut patterns = Vec::new();

	for cy in 0..chunks_y {
		for cx in 0..chunks_x {
			// Normal orientation
			let mut pattern: Vec<TileType> = Vec::new();
			let start_x = cx * chunk_size;
			let end_x   = (cx + 1) * chunk_size;
			let start_y = cy * chunk_size;
			let end_y   = (cy + 1) * chunk_size;

			for y in start_y..end_y {
				for x in start_x..end_x {
					let idx = map.xy_idx(x, y);
					pattern.push(map.tiles[idx]);
				}
			}
			patterns.push(pattern);

			if !include_flipping { continue }

			// Flip
			let mut pattern_horizontal = Vec::new();
			let mut pattern_vertical = Vec::new();
			let mut pattern_both = Vec::new();
			let mut idx : usize;

			for y in start_y..end_y {
				for x in start_x..end_x {
					idx = map.xy_idx(end_x - (x + 1), y);
					pattern_horizontal.push(map.tiles[idx]);

					idx = map.xy_idx(x, end_y - (y + 1));
					pattern_vertical.push(map.tiles[idx]);

					idx = map.xy_idx(end_x - (x + 1), end_y - (y + 1));
					pattern_both.push(map.tiles[idx]);
				}
			}
			patterns.push(pattern_horizontal);
			patterns.push(pattern_vertical);
			patterns.push(pattern_both);
		}
	}

	// Dedupe
	if dedupe {
		let set : HashSet<Vec<TileType>> = patterns.drain(..).collect();
		patterns.extend(set.into_iter());
	}

	patterns
}

pub fn render_pattern_to_map (
	map: &mut Map,
	chunk: &MapChunk,
	chunk_size: i32,
	start_x: i32,
	start_y: i32,
) {
	let mut i = 0usize;
	for y in 0..chunk_size {
		for x in 0..chunk_size {
			let idx = map.xy_idx(start_x + x, start_y + y);
			map.tiles[idx] = chunk.pattern[i];
			map.visible_tiles[idx] = true;
			i += 1;
		}
	}

	for (x, northbound) in chunk.exits[0].iter().enumerate() {
		if *northbound {
			let idx = map.xy_idx(start_x + x as i32, start_y);
			map.tiles[idx] = TileType::Placeholder;
		}
	}

	for (x, southbound) in chunk.exits[1].iter().enumerate() {
		if *southbound {
			let idx = map.xy_idx(start_x + x as i32, start_y + chunk_size - 1);
			map.tiles[idx] = TileType::Placeholder;
		}
	}

	for (x, westbound) in chunk.exits[2].iter().enumerate() {
		if *westbound {
			let idx = map.xy_idx(start_x, start_y + x as i32);
			map.tiles[idx] = TileType::Placeholder;
		}
	}

	for (x, eastbound) in chunk.exits[3].iter().enumerate() {
		if *eastbound {
			let idx = map.xy_idx(start_x + chunk_size - 1, start_y + x as i32);
			map.tiles[idx] = TileType::Placeholder;
		}
	}
}

pub fn patterns_to_constraints (patterns: Vec<Vec<TileType>>, chunk_size: i32) -> Vec<MapChunk> {
	let mut constraints : Vec<MapChunk> = Vec::new();

	const VEC_BOOL: Vec<bool> = Vec::new();
	const VEC_USIZE: Vec<usize> = Vec::new();

	for p in patterns {
		let mut new_chunk = MapChunk {
			pattern: p,
			exits: [VEC_BOOL; 4],
			has_exits: true,
			compatible_with: [VEC_USIZE; 4],
		};

		for exit in new_chunk.exits.iter_mut() {
			for _i in 0..chunk_size {
				exit.push(false);
			}
		}

		let mut n_exits = 0;

		for x in 0..chunk_size {
			let north_idx = tile_idx_in_chunk(chunk_size, x, 0);
			if new_chunk.pattern[north_idx] == TileType::Floor {
				new_chunk.exits[0][x as usize] = true;
				n_exits += 1;
			}

			let south_idx = tile_idx_in_chunk(chunk_size, x, chunk_size - 1);
			if new_chunk.pattern[south_idx] == TileType::Floor {
				new_chunk.exits[1][x as usize] = true;
				n_exits += 1;
			}

			let west_idx = tile_idx_in_chunk(chunk_size, 0, x);
			if new_chunk.pattern[west_idx] == TileType::Floor {
				new_chunk.exits[2][x as usize] = true;
				n_exits += 1;
			}

			let east_idx = tile_idx_in_chunk(chunk_size, chunk_size - 1, x);
			if new_chunk.pattern[east_idx] == TileType::Floor {
				new_chunk.exits[3][x as usize] = true;
				n_exits += 1;
			}
		}

		if n_exits == 0 {
			new_chunk.has_exits = false;
		}

		constraints.push(new_chunk);
	}

	let ch = constraints.clone();
	for c in constraints.iter_mut() {
		for (j, potential) in ch.iter().enumerate() {
			if !c.has_exits || !potential.has_exits {
				for compat in c.compatible_with.iter_mut() {
					compat.push(j);
				}
			} else {
				for (direction, exit_list) in c.exits.iter_mut().enumerate() {
					let opposite = match direction {
						0 => 1, // N -> S
						1 => 0, // S -> N
						2 => 3, // W -> E
						_ => 2, // E -> W
					};

					let mut it_fits = false;
					let mut has_any = false;

					for (slot, can_enter) in exit_list.iter().enumerate() {
						if *can_enter {
							has_any = true;
							if potential.exits[opposite][slot] {
								it_fits = true;
							}
						}

						if it_fits {
							c.compatible_with[direction].push(j);
						}

						if !has_any {
							// for compat in c.compatible_with.iter_mut() {
							// 	compat.push(j);
							// }
							let matching_exit_count = potential.exits[opposite]
								.iter().filter(|a| !**a).count();
							if matching_exit_count == 0 {
								c.compatible_with[direction].push(j);
							}
						}
					}
				}
			}
		}
	}

	constraints
}