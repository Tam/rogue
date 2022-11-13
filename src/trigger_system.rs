use rltk::RGB;
use specs::prelude::*;
use crate::{EntityMoved, EntityTrigger, Hidden, InflictsDamage, Name, Position, SingleActivation, SufferDamage};
use crate::gamelog::GameLog;
use crate::map::Map;
use crate::particle_system::ParticleBuilder;

pub struct TriggerSystem {}

impl<'a> System<'a> for TriggerSystem {
	type SystemData = (
		Entities<'a>,
		ReadExpect<'a, Map>,
		WriteStorage<'a, EntityMoved>,
		ReadStorage<'a, Position>,
		ReadStorage<'a, EntityTrigger>,
		WriteStorage<'a, Hidden>,
		ReadStorage<'a, Name>,
		WriteExpect<'a, GameLog>,
		ReadStorage<'a, InflictsDamage>,
		WriteExpect<'a, ParticleBuilder>,
		WriteStorage<'a, SufferDamage>,
		ReadStorage<'a, SingleActivation>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (
			entities, map, mut entity_moved, position, entity_trigger,
			mut hidden, names, mut log, inflicts_damage, mut particles,
			mut suffer_damage, single_activation,
		) = data;

		let mut remove_entities : Vec<Entity> = Vec::new();

		for (entity, mut _moved, pos) in (&entities, &mut entity_moved, &position).join() {
			let idx = map.xy_idx(pos.x, pos.y);
			for entity_id in map.tile_content[idx].iter() {
				if entity == *entity_id { continue } // don't check self

				let is_trigger = entity_trigger.get(*entity_id);
				if let Some(_trigger) = is_trigger {

					let damage = inflicts_damage.get(*entity_id);
					if let Some(damage) = damage {
						particles.request(
							pos.x, pos.y,
							RGB::named(rltk::RED),
							RGB::named(rltk::BLACK),
							rltk::to_cp437('‼'),
							150.,
						);

						SufferDamage::new_damage(
							&mut suffer_damage,
							entity,
							damage.damage,
						);
					}

					let name = names.get(*entity_id);
					if let Some(name) = name {
						log.entries.push(format!(
							"{} triggers!",
							&name.name,
						));
					}

					let sa = single_activation.get(*entity_id);
					if let Some(_sa) = sa {
						remove_entities.push(*entity_id);
					}

					// No longer hidden
					hidden.remove(*entity_id);
				}
			}
		}

		for trap in remove_entities.iter() {
			entities.delete(*trap).expect("Failed to de-trap");
		}

		entity_moved.clear();
	}
}