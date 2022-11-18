use std::collections::HashMap;
use rltk::RandomNumberGenerator;
use specs::World;
use crate::map::Map;
use crate::{MAP_HEIGHT, MAP_WIDTH, Position, spawner, TileType};
use crate::map_builder::common::{generate_voronoi_spawn_regions, remove_unreachable_areas_returning_most_distant};
#[cfg(feature = "mapgen_visualiser")] use crate::map_builder::common::snapshot;
use crate::map_builder::MapBuilder;

#[derive(PartialEq, Copy, Clone)]
pub enum DistanceAlgorithm {
	Pythagoras,
	Manhattan,
	Chebyshev,
}

pub struct VoronoiBuilder {
	map: Map,
	starting_position: Position,
	depth: i32,
	noise_areas: HashMap<i32, Vec<usize>>,
	n_seeds: usize,
	distance_algorithm: DistanceAlgorithm,
	#[cfg(feature = "mapgen_visualiser")] name: String,
	#[cfg(feature = "mapgen_visualiser")] history: Vec<Map>,
}

impl VoronoiBuilder {

	pub fn new (
		depth: i32,
		distance_algorithm: DistanceAlgorithm,
		#[allow(unused_variables)] name: String,
	) -> VoronoiBuilder {
		VoronoiBuilder {
			map: Map::new(
				MAP_WIDTH as i32,
				MAP_HEIGHT as i32,
				depth,
				Some(TileType::Wall)
			),
			starting_position: Position { x: 0, y: 0 },
			depth,
			noise_areas: HashMap::new(),
			n_seeds: 64,
			distance_algorithm,
			#[cfg(feature = "mapgen_visualiser")] name,
			#[cfg(feature = "mapgen_visualiser")] history: Vec::new(),
		}
	}

	#[allow(dead_code)]
	pub fn pythagoras (depth: i32) -> VoronoiBuilder {
		VoronoiBuilder::new(
			depth,
			DistanceAlgorithm::Pythagoras,
			"Pythagoras".to_string()
		)
	}

	#[allow(dead_code)]
	pub fn manhattan (depth: i32) -> VoronoiBuilder {
		VoronoiBuilder::new(
			depth,
			DistanceAlgorithm::Manhattan,
			"Manhattan".to_string()
		)
	}

	#[allow(dead_code)]
	pub fn chebyshev (depth: i32) -> VoronoiBuilder {
		VoronoiBuilder::new(
			depth,
			DistanceAlgorithm::Chebyshev,
			"Chebyshev".to_string()
		)
	}
}

impl MapBuilder for VoronoiBuilder {
	fn get_map(&mut self) -> Map {
		self.map.clone()
	}

	fn get_starting_position(&mut self) -> Position {
		self.starting_position.clone()
	}

	fn build(&mut self) {
		let mut rng = RandomNumberGenerator::new();

		// Start at center
		self.starting_position = Position {
			x: self.map.width / 2,
			y: self.map.height / 2,
		};
		let start_idx = self.map.xy_idx(
			self.starting_position.x,
			self.starting_position.y,
		);

		// Voronoi
		let mut voronoi_seeds : Vec<(usize, rltk::Point)> = Vec::new();

		while voronoi_seeds.len() < self.n_seeds {
			let vx = rng.roll_dice(1, self.map.width - 1);
			let vy = rng.roll_dice(1, self.map.height * 2 - 1);
			let vidx = self.map.xy_idx(vx, vy);
			let candidate = (vidx, rltk::Point::new(vx, vy));
			if !voronoi_seeds.contains(&candidate) {
				voronoi_seeds.push(candidate);
			}
		}

		let mut voronoi_distance = vec![(0, 0.0f32); self.n_seeds];
		let mut voronoi_membership : Vec<i32> = vec![0; self.map.width as usize * self.map.height as usize];

		for (i, vid) in voronoi_membership.iter_mut().enumerate() {
			let x = i as i32 % self.map.width;
			let y = i as i32 / self.map.height;

			for (seed, pos) in voronoi_seeds.iter().enumerate() {
				let dist = match self.distance_algorithm {
					DistanceAlgorithm::Pythagoras => rltk::DistanceAlg::PythagorasSquared.distance2d(
						rltk::Point::new(x, y),
						pos.1,
					),
					DistanceAlgorithm::Manhattan => rltk::DistanceAlg::Manhattan.distance2d(
						rltk::Point::new(x, y),
						pos.1,
					),
					DistanceAlgorithm::Chebyshev => rltk::DistanceAlg::Chebyshev.distance2d(
						rltk::Point::new(x, y),
						pos.1,
					),
				};
				voronoi_distance[seed] = (seed, dist);
			}

			voronoi_distance.sort_by(|a, b| {
				a.1.partial_cmp(&b.1).unwrap()
			});

			*vid = voronoi_distance[0].0 as i32;
		}

		for y in 1..self.map.height - 1 {
			for x in 1..self.map.width - 1 {
				let mut neighbours = 0;
				let idx = self.map.xy_idx(x, y);
				let seed = voronoi_membership[idx];

				if voronoi_membership[self.map.xy_idx(x - 1, y)] != seed { neighbours += 1; }
				if voronoi_membership[self.map.xy_idx(x + 1, y)] != seed { neighbours += 1; }
				if voronoi_membership[self.map.xy_idx(x, y - 1)] != seed { neighbours += 1; }
				if voronoi_membership[self.map.xy_idx(x, y + 1)] != seed { neighbours += 1; }

				if neighbours < 2 {
					self.map.tiles[idx] = TileType::Floor;
				}
			}

			#[cfg(feature = "mapgen_visualiser")] self.take_snapshot();
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
		format!("Voronoi ({})", self.name)
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