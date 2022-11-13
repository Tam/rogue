use specs::prelude::*;
use crate::{BlocksTile, Position};
use crate::map::Map;

pub struct MapIndexingSystem {}

impl<'a> System<'a> for MapIndexingSystem {
	type SystemData = (
		WriteExpect<'a, Map>,
		ReadStorage<'a, Position>,
		ReadStorage<'a, BlocksTile>,
		Entities<'a>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (mut map, position, blockers, entities) = data;

		map.populate_blocked();
		map.clear_content_index();

		for (entity, position) in (&entities, &position).join() {
			let idx = map.xy_idx(position.x, position.y);

			// If the block, update the blocking list
			let _p : Option<&BlocksTile> = blockers.get(entity);
			if let Some(_p) = _p {
				map.blocked[idx] = true;
			}

			// Push a copy ref of the entity to the appropriate index slot
			map.tile_content[idx].push(entity);
		}
	}
}