use specs::prelude::*;
use crate::{CombatStats, Name, Player, Position, RunState, SufferDamage};
use crate::gamelog::GameLog;
use crate::map::Map;

pub struct DamageSystem {}

impl<'a> System<'a> for DamageSystem {
	type SystemData = (
		WriteStorage<'a, CombatStats>,
		WriteStorage<'a, SufferDamage>,
		Entities<'a>,
		ReadStorage<'a, Position>,
		WriteExpect<'a, Map>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (
			mut stats, mut damage, entities, positions, mut map,
		) = data;

		for (entity, mut stats, damage) in (&entities, &mut stats, &damage).join() {
			stats.hp -= damage.amount.iter().sum::<i32>();
			let pos = positions.get(entity);
			if let Some(pos) = pos {
				let idx = map.xy_idx(pos.x, pos.y);
				map.bloodstains.insert(idx);
			}
		}

		damage.clear();
	}
}

impl DamageSystem {
	pub fn delete_the_dead (ecs: &mut World) {
		let mut dead : Vec<Entity> = Vec::new();

		{
			let combat_stats = ecs.read_storage::<CombatStats>();
			let players = ecs.read_storage::<Player>();
			let names = ecs.read_storage::<Name>();
			let mut log = ecs.write_resource::<GameLog>();
			let entities = ecs.entities();

			for (entity, stats) in (&entities, &combat_stats).join() {
				if stats.hp < 1 {
					let player = players.get(entity);
					match player {
						None => {
							let victim_name = names.get(entity);
							if let Some(victim_name) = victim_name {
								log.entries.push(format!(
									"{} is dead!",
									&victim_name.name,
								));
							}
							dead.push(entity);
						}
						Some(_) => {
							let mut runstate = ecs.write_resource::<RunState>();
							*runstate = RunState::GameOver;
						}
					}
				}
			}
		}

		for victim in dead {
			ecs.delete_entity(victim).expect("Failed to delete dead");
		}
	}
}