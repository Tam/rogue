use specs::prelude::*;
use super::{Viewshed, Monster};
use rltk::{Point, a_star_search, DistanceAlg, RGB};
use crate::{Confusion, EntityMoved, Position, RunState, WantsToMelee};
use crate::map::Map;
use crate::particle_system::ParticleBuilder;

pub struct MonsterAI {}

impl<'a> System<'a> for MonsterAI {
	type SystemData = (
		WriteExpect<'a, Map>,
		ReadExpect<'a, Point>,
		ReadExpect<'a, Entity>,
		ReadExpect<'a, RunState>,
		Entities<'a>,
		WriteStorage<'a, Viewshed>,
		ReadStorage<'a, Monster>,
		WriteStorage<'a, Position>,
		WriteStorage<'a, WantsToMelee>,
		WriteStorage<'a, Confusion>,
		WriteExpect<'a, ParticleBuilder>,
		WriteStorage<'a, EntityMoved>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (
			mut map,
			player_pos,
			player_entity,
			runstate,
			entities,
			mut viewshed,
			monster,
			mut position,
			mut wants_to_melee,
			mut confused,
			mut particle_builder,
			mut entity_moved,
		) = data;

		if *runstate != RunState::MonsterTurn { return; }

		for (entity, mut viewshed, _monster, mut pos) in (&entities, &mut viewshed, &monster, &mut position).join()
		{
			let mut can_act = true;

			let is_confused = confused.get_mut(entity);
			if let Some(is_confused) = is_confused {
				is_confused.turns -= 1;
				if is_confused.turns < 1 {
					confused.remove(entity);
				}
				can_act = false;
				particle_builder.request(
					pos.x, pos.y,
					RGB::named(rltk::BLUEVIOLET),
					RGB::named(rltk::BLACK),
					rltk::to_cp437('?'),
					250.,
				);
			}

			if !can_act { continue; }

			let distance = DistanceAlg::Pythagoras.distance2d(
				Point::new(pos.x, pos.y),
				*player_pos,
			);

			if distance < 1.5 {
				wants_to_melee.insert(
					entity,
					WantsToMelee { target: *player_entity }
				).expect("Unable to attack player!");
				return;
			}

			if viewshed.visible_tiles.contains(&*player_pos) {
				let path = a_star_search(
					map.xy_idx(pos.x, pos.y) as i32,
					map.xy_idx(player_pos.x, player_pos.y) as i32,
					&mut *map,
				);

				if path.success && path.steps.len() > 1 {
					let mut idx = map.xy_idx(pos.x, pos.y);
					map.blocked[idx] = false;

					pos.x = path.steps[1] as i32 % map.width;
					pos.y = path.steps[1] as i32 / map.width;

					idx = map.xy_idx(pos.x, pos.y);
					map.blocked[idx] = true;
					viewshed.dirty = true;
					entity_moved.insert(entity, EntityMoved {})
						.expect("Failed to use numerous legs");
				}
			}
		}
	}
}