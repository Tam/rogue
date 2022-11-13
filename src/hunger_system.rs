use specs::prelude::*;
use crate::{HungerClock, HungerState, RunState, SufferDamage};
use crate::gamelog::GameLog;

pub struct HungerSystem {}

impl<'a> System<'a> for HungerSystem {
	type SystemData = (
		Entities<'a>,
		WriteStorage<'a, HungerClock>,
		ReadExpect<'a, Entity>,
		ReadExpect<'a, RunState>,
		WriteStorage<'a, SufferDamage>,
		WriteExpect<'a, GameLog>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (
			entities, mut hunger_clock, player_entity, runstate,
			mut inflict_damage, mut log,
		) = data;

		for (entity, mut clock) in (&entities, &mut hunger_clock).join() {
			let is_player = entity == *player_entity;

			match *runstate {
				RunState::PlayerTurn => { if !is_player { continue } }
				RunState::MonsterTurn => { if is_player { continue } }
				_ => continue,
			}

			clock.duration -= 1;
			if clock.duration > 0 { continue; }

			match clock.state {
				HungerState::WellFed => {
					clock.state = HungerState::Normal;
					clock.duration = 200;
					if is_player {
						log.entries.push("It's been a while since you ate, it's now safe to swim".to_string());
					}
				}
				HungerState::Normal => {
					clock.state = HungerState::Hungry;
					clock.duration = 200;
					if is_player {
						log.entries.push("Your stomach starts to growl".to_string());
					}
				}
				HungerState::Hungry => {
					clock.state = HungerState::Starving;
					clock.duration = 200;
					if is_player {
						log.entries.push("Your stomach is about to go on strike".to_string());
					}
				}
				HungerState::Starving => {
					if is_player {
						log.entries.push("Your stomach is rioting".to_string());
					}

					SufferDamage::new_damage(&mut inflict_damage, entity, 1);
				}
			}
		}
	}
}