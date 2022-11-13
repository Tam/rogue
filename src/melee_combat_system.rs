use rltk::RGB;
use specs::prelude::*;
use crate::{CombatStats, DefenseBonus, Equipped, HungerClock, HungerState, MeleePowerBonus, Name, Position, SufferDamage, WantsToMelee};
use crate::gamelog::GameLog;
use crate::particle_system::ParticleBuilder;

pub struct MeleeCombatSystem {}

impl<'a> System<'a> for MeleeCombatSystem {
	type SystemData = (
		Entities<'a>,
		WriteStorage<'a, WantsToMelee>,
		ReadStorage<'a, Name>,
		ReadStorage<'a, CombatStats>,
		WriteStorage<'a, SufferDamage>,
		WriteExpect<'a, GameLog>,
		ReadStorage<'a, MeleePowerBonus>,
		ReadStorage<'a, DefenseBonus>,
		ReadStorage<'a, Equipped>,
		WriteExpect<'a, ParticleBuilder>,
		ReadStorage<'a, Position>,
		ReadStorage<'a, HungerClock>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (
			entities, mut wants_melee, names, combat_stats, mut inflict_damage,
			mut log, melee_power_bonuses, defense_bonuses, equipped,
			mut particle_builder, positions, hunger,
		) = data;

		let query = (&entities, &wants_melee, &names, &combat_stats).join();
		for (_entity, wants_melee, name, stats) in query {
			if stats.hp > 0 {
				let mut offensive_bonus = 0;
				for (_item_entity, power_bonus, equipped_by) in (&entities, &melee_power_bonuses, &equipped).join() {
					if equipped_by.owner == _entity {
						offensive_bonus += power_bonus.power;
					}
				}

				let hc = hunger.get(_entity);
				if let Some(hc) = hc {
					if hc.state == HungerState::WellFed {
						offensive_bonus += 1;
					}
				}

				if let Some(target_stats) = combat_stats.get(wants_melee.target) {
					if target_stats.hp > 0 {
						let target_name = names.get(wants_melee.target).unwrap();

						let mut defensive_bonus = 0;
						for (_item_entity, defense_bonus, equipped_by) in (&entities, &defense_bonuses, &equipped).join() {
							if equipped_by.owner == wants_melee.target {
								defensive_bonus += defense_bonus.defense;
							}
						}

						let pos = positions.get(wants_melee.target);
						if let Some(pos) = pos {
							particle_builder.request(
								pos.x, pos.y,
								RGB::named(rltk::ORANGERED),
								RGB::named(rltk::BLACK),
								rltk::to_cp437('‼'),
								150.,
							);
						}

						let damage = i32::max(0, (stats.power + offensive_bonus) - (target_stats.defence + defensive_bonus));

						if damage == 0 {
							log.entries.push(format!(
								"{} did no damage to {}!",
								&name.name,
								&target_name.name,
							));
						} else {
							log.entries.push(format!(
								"{} hits {} for {}hp!",
								&name.name,
								&target_name.name,
								damage,
							));

							SufferDamage::new_damage(
								&mut inflict_damage,
								wants_melee.target,
								damage,
							);
						}
					}
				}
			}
		}

		wants_melee.clear();
	}
}