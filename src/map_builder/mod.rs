mod common;
mod simple_map;
mod bsp_dungeon;
mod bsp_interior;
mod cellular_automata;
mod drunkard;
mod maze;
mod dla;
mod voronoi;

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
};
use crate::map_builder::voronoi::VoronoiBuilder;

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

macro_rules! pick_random {
	($($x:expr),* $(,)?) => {{
		let mut rng = rltk::RandomNumberGenerator::new();
		let builder = (rng.roll_dice(1, ${count(x, 0)}) - 1) as u8;
		match builder {
			$(${index()} => $x,)*
			_ => panic!("Map out of range!")
		}
	}};
}

pub fn random_builder (depth: i32) -> Box<dyn MapBuilder> {
	pick_random!(
		Box::new(SimpleMapBuilder::new(depth)),
		Box::new(BspInteriorBuilder::new(depth)),
		Box::new(CellularAutomataBuilder::new(depth)),
		Box::new(BspDungeonBuilder::new(depth)),
		Box::new(DrunkardWalkBuilder::open_area(depth)),
		Box::new(DrunkardWalkBuilder::open_halls(depth)),
		Box::new(DrunkardWalkBuilder::winding_passages(depth)),
		Box::new(DrunkardWalkBuilder::fat_passages(depth)),
		Box::new(DrunkardWalkBuilder::fearful_symmetry(depth)),
		Box::new(MazeBuilder::new(depth)),
		Box::new(DLABuilder::walk_inwards(depth)),
		Box::new(DLABuilder::walk_outwards(depth)),
		Box::new(DLABuilder::central_attractor(depth)),
		Box::new(DLABuilder::insectoid(depth)),
		Box::new(VoronoiBuilder::pythagoras(depth)),
		Box::new(VoronoiBuilder::manhattan(depth)),
		Box::new(VoronoiBuilder::chebyshev(depth)),
	)
}
