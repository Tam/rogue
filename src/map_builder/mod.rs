mod common;
mod simple_map;
mod bsp_dungeon;
mod bsp_interior;

use specs::World;
use crate::Position;
use super::Map;
#[allow(unused_imports)]
use crate::map_builder::{
	simple_map::SimpleMapBuilder,
	bsp_dungeon::BspDungeonBuilder,
	bsp_interior::BspInteriorBuilder,
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

pub fn random_builder (depth: i32) -> Box<dyn MapBuilder> {
	let mut rng = rltk::RandomNumberGenerator::new();
	let builder = rng.roll_dice(1, 3);
	match builder {
		1 => Box::new(SimpleMapBuilder::new(depth)),
		2 => Box::new(BspInteriorBuilder::new(depth)),
		_ => Box::new(BspDungeonBuilder::new(depth)),
	}
}
