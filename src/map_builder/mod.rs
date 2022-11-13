mod simple_map;
mod common;

use specs::World;
use simple_map::SimpleMapBuilder;
use crate::Position;
use super::Map;

pub trait MapBuilder {
	fn get_map (&mut self) -> Map;
	fn get_starting_position (&mut self) -> Position;

	fn build (&mut self);
	fn spawn (&mut self, ecs: &mut World);

	#[cfg(feature = "mapgen_visualiser")]
	fn get_snapshot_history (&self) -> Vec<Map>;
	#[cfg(feature = "mapgen_visualiser")]
	fn take_snapshot (&mut self);
}

pub fn random_builder (depth: i32) -> Box<dyn MapBuilder> {
	Box::new(SimpleMapBuilder::new(depth))
}
