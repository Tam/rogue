mod constraints;
mod common;
mod solver;

use std::collections::HashMap;
use rltk::RandomNumberGenerator;
use specs::World;
use crate::map::Map;
use crate::{MAP_HEIGHT, MAP_WIDTH, Position, spawner, TileType};
#[cfg(feature = "mapgen_visualiser")]
use crate::map_builder::common::snapshot;
use crate::map_builder::common::{generate_voronoi_spawn_regions, remove_unreachable_areas_returning_most_distant};
use crate::map_builder::MapBuilder;
use crate::map_builder::waveform_collapse::common::MapChunk;
use crate::map_builder::waveform_collapse::constraints::{build_patterns, patterns_to_constraints, render_pattern_to_map};
use crate::map_builder::waveform_collapse::solver::Solver;

pub struct WaveformCollapseBuilder {
	map: Map,
	starting_position: Position,
	depth: i32,
	noise_areas: HashMap<i32, Vec<usize>>,
	derive_from: Option<Box<dyn MapBuilder>>,
	#[cfg(feature = "mapgen_visualiser")] name: String,
	#[cfg(feature = "mapgen_visualiser")] history: Vec<Map>,
}

impl WaveformCollapseBuilder {
	pub fn new (
		depth: i32,
		derive_from: Option<Box<dyn MapBuilder>>,
		#[cfg(feature = "mapgen_visualiser")] name: String,
	) -> WaveformCollapseBuilder {
		WaveformCollapseBuilder {
			map: Map::new(
				MAP_WIDTH as i32,
				MAP_HEIGHT as i32,
				depth,
				Some(TileType::Wall),
			),
			starting_position: Position { x: 0, y: 0 },
			depth,
			noise_areas: HashMap::new(),
			derive_from,
			#[cfg(feature = "mapgen_visualiser")] name,
			#[cfg(feature = "mapgen_visualiser")] history: Vec::new(),
		}
	}

	#[allow(dead_code)]
	pub fn derived_map (depth: i32, builder: Box<dyn MapBuilder>) -> WaveformCollapseBuilder {
		let derive_from = Some(builder);
		#[cfg(feature = "mapgen_visualiser")]
		let name = derive_from.as_ref().unwrap().get_name();

		WaveformCollapseBuilder::new(
			depth,
			derive_from,
			#[cfg(feature = "mapgen_visualiser")] format!(
				"[Derived] {}",
				name,
			),
		)
	}

	#[allow(dead_code)]
	#[cfg(feature = "mapgen_visualiser")]
	fn render_tile_gallery (&mut self, constraints: &Vec<MapChunk>, chunk_size: i32) {
		self.map = Map::new_default(0);
		let mut counter = 0;
		let mut x = 0;
		let mut y = 0;

		while counter < constraints.len() {
			render_pattern_to_map(
				&mut self.map,
				&constraints[counter],
				chunk_size,
				x, y,
			);

			x += chunk_size + 1;
			if x + chunk_size > self.map.width {
				// Move to next row
				x = 1;
				y += chunk_size + 1;

				if y + chunk_size > self.map.height {
					// Move to next page
					self.take_snapshot();
					self.map = Map::new(MAP_WIDTH as i32, MAP_HEIGHT as i32, 0, None);

					x = 1;
					y = 1;
				}
			}

			counter += 1;
		}

		self.take_snapshot();
	}
}

impl MapBuilder for WaveformCollapseBuilder {
	fn get_map(&mut self) -> Map {
		self.map.clone()
	}

	fn get_starting_position(&mut self) -> Position {
		self.starting_position.clone()
	}

	fn build(&mut self) {
		let mut rng = RandomNumberGenerator::new();

		// Waveform Collapse
		const CHUNK_SIZE: i32 = 8;

		let mut source_map: Map;

		let prebuilder = &mut self.derive_from.as_mut().unwrap();
		prebuilder.build();
		source_map = prebuilder.get_map();
		for t in source_map.tiles.iter_mut() {
			if *t == TileType::DownStairs { *t = TileType::Floor }
		}

		let patterns = build_patterns(
			&source_map,
			CHUNK_SIZE,
			true,
			true,
		);

		let constraints = patterns_to_constraints(patterns, CHUNK_SIZE);

		// #[cfg(feature = "mapgen_visualiser")]
		// self.render_tile_gallery(&constraints, CHUNK_SIZE);

		loop {
			let mut solver = Solver::new(
				constraints.clone(),
				CHUNK_SIZE,
				&self.map,
			);

			while !solver.iteration(&mut self.map, &mut rng) {
				#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
			}

			#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();

			if solver.possible { break }
		}

		// Starting pos
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
		format!("Waveform Collapse ({})", self.name)
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
