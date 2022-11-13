use rltk::{field_of_view, Point, RandomNumberGenerator};
use specs::prelude::*;
use crate::{Hidden, Name, Player};
use crate::gamelog::GameLog;
use crate::map::Map;
use super::{Viewshed, Position};

pub struct VisibilitySystem {}

impl<'a> System<'a> for VisibilitySystem {
	type SystemData = (
		WriteExpect<'a, Map>,
		Entities<'a>,
		WriteStorage<'a, Viewshed>,
		WriteStorage<'a, Position>,
		ReadStorage<'a, Player>,
		WriteStorage<'a, Hidden>,
		WriteExpect<'a, RandomNumberGenerator>,
		WriteExpect<'a, GameLog>,
		ReadStorage<'a, Name>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (
			mut map, entities, mut viewshed, pos, player,
			mut hidden, mut rng, mut log, names,
		) = data;

		for (ent, viewshed, pos) in (&entities, &mut viewshed, &pos).join() {
			if !viewshed.dirty { continue; }
			viewshed.dirty = false;

			viewshed.visible_tiles.clear();
			viewshed.visible_tiles = field_of_view(
				Point::new(pos.x, pos.y),
				viewshed.range,
				&*map
			);
			viewshed.visible_tiles.retain(
				|p|
					   p.x >= 0
					&& p.x < map.width
					&& p.y >= 0
					&& p.y <= map.height
			);

			// Reveal what the player can see
			let p : Option<&Player> = player.get(ent);
			if let Some(_p) = p {
				for t in map.visible_tiles.iter_mut() { *t = false };
				for vis in viewshed.visible_tiles.iter() {
					let idx = map.xy_idx(vis.x, vis.y);
					map.revealed_tiles[idx] = true;
					map.visible_tiles[idx] = true;

					// Chance to reveal hidden things
					for e in map.tile_content[idx].iter() {
						let maybe_hidden = hidden.get(*e);
						if let Some(_hidden) = maybe_hidden {
							if rng.roll_dice(1, 24) == 1 {
								let name = names.get(*e);
								if let Some(name) = name {
									log.entries.push(format!(
										"You've spotted a {}!",
										&name.name,
									));
								}
								hidden.remove(*e);
							}
						}
					}
				}
			}
		}
	}
}