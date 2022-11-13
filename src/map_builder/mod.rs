mod common;
mod simple_map;
mod bsp_dungeon;
mod bsp_interior;
mod cellular_automata;

use specs::World;
use crate::Position;
use super::Map;
#[allow(unused_imports)]
use crate::map_builder::{
	simple_map::SimpleMapBuilder,
	bsp_dungeon::BspDungeonBuilder,
	bsp_interior::BspInteriorBuilder,
	cellular_automata::CellularAutomataBuilder,
};

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
		let builder = rng.roll_dice(1, ${count(x, 0)}) - 1;
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
	)
}
