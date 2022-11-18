use std::collections::HashSet;
use rltk::RandomNumberGenerator;
use crate::map::Map;
use crate::map_builder::waveform_collapse::common::MapChunk;

pub struct Solver {
	constraints: Vec<MapChunk>,
	chunk_size: i32,
	chunks: Vec<Option<usize>>,
	chunks_x: usize,
	chunks_y: usize,
	remaining: Vec<(usize, i32)>, // (index, # neighbours)
	pub possible: bool,
}

impl Solver {
	pub fn new (constraints: Vec<MapChunk>, chunk_size: i32, map: &Map) -> Solver {
		let chunks_x = (map.width / chunk_size) as usize;
		let chunks_y = (map.height / chunk_size) as usize;
		let mut remaining: Vec<(usize, i32)> = Vec::new();

		for i in 0..(chunks_x * chunks_y) {
			remaining.push((i, 0));
		}

		Solver {
			constraints,
			chunk_size,
			chunks: vec![None; chunks_x * chunks_y],
			chunks_x,
			chunks_y,
			remaining,
			possible: true,
		}
	}

	fn chunk_idx (&self, x: usize, y: usize) -> usize {
		((y * self.chunks_x) + x) as usize
	}

	fn count_neighbours (&self, chunk_x: usize, chunk_y: usize) -> i32 {
		let mut neighbours = 0;

		if chunk_x > 0 {
			let left_idx = self.chunk_idx(chunk_x - 1, chunk_y);
			if self.chunks[left_idx] != None { neighbours += 1 }
		}

		if chunk_x < self.chunks_x - 1 {
			let right_idx = self.chunk_idx(chunk_x + 1, chunk_y);
			if self.chunks[right_idx] != None { neighbours += 1 }
		}

		if chunk_y > 0 {
			let up_idx = self.chunk_idx(chunk_x, chunk_y - 1);
			if self.chunks[up_idx] != None { neighbours += 1 }
		}

		if chunk_y < self.chunks_y - 1 {
			let down_idx = self.chunk_idx(chunk_x, chunk_y + 1);
			if self.chunks[down_idx] != None { neighbours += 1 }
		}

		neighbours
	}

	pub fn iteration (&mut self, map: &mut Map, rng: &mut RandomNumberGenerator) -> bool {
		if self.remaining.is_empty() { return true }

		// Populate neighbour count
		let mut remain_copy = self.remaining.clone();
		let mut neighbours_exist = false;

		for r in remain_copy.iter_mut() {
			let idx = r.0;
			let chunk_x = idx % self.chunks_x;
			let chunk_y = idx / self.chunks_x;

			let neighbour_count = self.count_neighbours(chunk_x, chunk_y);
			if neighbour_count > 0 { neighbours_exist = true }
			r.1 = neighbour_count;
		}

		remain_copy.sort_by(|a, b| b.1.cmp(&a.1));
		self.remaining = remain_copy;

		// Pick random unhandled chunk
		let remaining_index = if !neighbours_exist {
			(rng.roll_dice(1, self.remaining.len() as i32) - 1) as usize
		} else { 0usize };

		let chunk_index = self.remaining[remaining_index].0;
		self.remaining.remove(remaining_index);

		let chunk_x = chunk_index % self.chunks_x;
		let chunk_y = chunk_index / self.chunks_x;

		let mut neighbours = 0;
		let mut options : Vec<Vec<usize>> = Vec::new();

		if chunk_x > 0 {
			let left_idx = self.chunk_idx(chunk_x - 1, chunk_y);
			if let Some(nt) = self.chunks[left_idx] {
				neighbours += 1;
				options.push(self.constraints[nt].compatible_with[3].clone())
			}
		}

		if chunk_x < self.chunks_x - 1 {
			let right_idx = self.chunk_idx(chunk_x + 1, chunk_y);
			if let Some(nt) = self.chunks[right_idx] {
				neighbours += 1;
				options.push(self.constraints[nt].compatible_with[2].clone())
			}
		}

		if chunk_y > 0 {
			let up_idx = self.chunk_idx(chunk_x, chunk_y - 1);
			if let Some(nt) = self.chunks[up_idx] {
				neighbours += 1;
				options.push(self.constraints[nt].compatible_with[1].clone())
			}
		}

		if chunk_y < self.chunks_y - 1 {
			let down_idx = self.chunk_idx(chunk_x, chunk_y + 1);
			if let Some(nt) = self.chunks[down_idx] {
				neighbours += 1;
				options.push(self.constraints[nt].compatible_with[0].clone())
			}
		}

		let new_chunk_idx;

		if neighbours == 0 {
			// Nothing nearby, pick at random
			new_chunk_idx = (rng.roll_dice(1, self.constraints.len() as i32) - 1) as usize;
		} else {
			// Has neighbours, find compatible
			let mut options_to_check : HashSet<usize> = HashSet::new();
			for o in options.iter() {
				for i in o.iter() {
					options_to_check.insert(*i);
				}
			}

			let mut possible_options : Vec<usize> = Vec::new();
			for new_chunk_idx in options_to_check.iter() {
				let mut possible = true;

				for o in options.iter() {
					if !o.contains(new_chunk_idx) { possible = false }
				}

				if possible {
					possible_options.push(*new_chunk_idx);
				}
			}

			if possible_options.is_empty() {
				self.possible = false;
				return true;
			} else {
				new_chunk_idx =
					if possible_options.len() == 1 { 0 }
					else { rng.roll_dice(1, possible_options.len() as i32) - 1 } as usize;
			}
		}

		// Blit chunk to map
		self.chunks[chunk_index] = Some(new_chunk_idx);

		let left = chunk_x as i32 * self.chunk_size as i32;
		let right = (chunk_x as i32 + 1) * self.chunk_size as i32;
		let top = chunk_y as i32 * self.chunk_size as i32;
		let bottom = (chunk_y as i32 + 1) * self.chunk_size as i32;

		let mut i : usize = 0;
		for y in top..bottom {
			for x in left..right {
				let idx = map.xy_idx(x, y);
				let tile = self.constraints[new_chunk_idx].pattern[i];
				map.tiles[idx] = tile;
				i += 1;
			}
		}

		false
	}
}