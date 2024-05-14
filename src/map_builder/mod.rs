mod common;
mod simple_map;
mod bsp_dungeon;
mod bsp_interior;
mod cellular_automata;
mod drunkard;
mod maze;
mod dla;
mod voronoi;
mod waveform_collapse;
mod prefab_builder;

use specs::World;
use crate::Position;
use super::Map;
#[allow(unused_imports)]
use crate::map_builder::{
	simple_map::SimpleMapBuilder,
	bsp_dungeon::BspDungeonBuilder,
	bsp_interior::BspInteriorBuilder,
	cellular_automata::CellularAutomataBuilder,
	drunkard::*,
	maze::MazeBuilder,
	dla::DLABuilder,
	voronoi::VoronoiBuilder,
	waveform_collapse::WaveformCollapseBuilder,
};
use crate::map_builder::prefab_builder::PrefabBuilder;

pub trait MapBuilder {
	fn get_map (&mut self) -> Map;
	fn get_starting_position (&mut self) -> Position;

	fn build (&mut self);
	fn spawn (&mut self, ecs: &mut World);

	#[cfg(feature = "mapgen_visualiser")]
	fn get_name (&self) -> String;
	#[cfg(feature = "mapgen_visualiser")]
	fn get_snapshot_history (&self) -> Vec<Map>;
	#[cfg(feature = "mapgen_visualiser")]
	fn take_snapshot (&mut self);
}

#[allow(unused_macros)]
macro_rules! pick_random {
	($depth:expr, $($x:expr),* $(,)?) => {{
		let mut rng = rltk::RandomNumberGenerator::new();
		let builder = (rng.roll_dice(1, ${count(x, 0)}) - 1) as u8;
		let mut result : Box<dyn MapBuilder>;
		match builder {
			$(${index()} => result = Box::new($x($depth)),)*
			_ => panic!("Map out of range!")
		}

		if rng.roll_dice(1, 3) == 1 {
			result = Box::new(WaveformCollapseBuilder::derived_map($depth, result));
		}

		result
	}};
}

pub fn random_builder (depth: i32) -> Box<dyn MapBuilder> {
	pick_random!(depth,
		SimpleMapBuilder::new,
		BspInteriorBuilder::new,
		CellularAutomataBuilder::new,
		BspDungeonBuilder::new,
		DrunkardWalkBuilder::open_area,
		DrunkardWalkBuilder::open_halls,
		DrunkardWalkBuilder::winding_passages,
		DrunkardWalkBuilder::fat_passages,
		DrunkardWalkBuilder::fearful_symmetry,
		MazeBuilder::new,
		DLABuilder::walk_inwards,
		DLABuilder::walk_outwards,
		DLABuilder::central_attractor,
		DLABuilder::insectoid,
		VoronoiBuilder::pythagoras,
		VoronoiBuilder::manhattan,
		VoronoiBuilder::chebyshev,
	)
	// Box::new(PrefabBuilder::new(depth))
}
