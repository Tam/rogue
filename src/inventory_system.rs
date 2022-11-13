use rltk::RGB;
use specs::prelude::*;
use crate::gamelog::GameLog;
use crate::{CombatStats, Consumable, InBackpack, Name, Position, ProvidesHealing, WantsToUseItem, WantsToDropItem, WantsToPickupItem, InflictsDamage, SufferDamage, AreaOfEffect, Confusion, Equippable, Equipped, WantsToRemoveItem, ProvidesFood, HungerClock, HungerState, MagicMapper, RunState};
use crate::map::Map;
use crate::particle_system::ParticleBuilder;

// Item Collection
// =========================================================================

pub struct ItemCollectionSystem {}

impl<'a> System<'a> for ItemCollectionSystem {
	type SystemData = (
		ReadExpect<'a, Entity>,
		WriteExpect<'a, GameLog>,
		WriteStorage<'a, WantsToPickupItem>,
		WriteStorage<'a, Position>,
		ReadStorage<'a, Name>,
		WriteStorage<'a, InBackpack>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (
			player_entity, mut gamelog, mut wants_pickup, mut positions, names,
			mut backpack,
		) = data;

		for pickup in wants_pickup.join() {
			positions.remove(pickup.item);
			backpack.insert(pickup.item, InBackpack {
				owner: pickup.collected_by,
			}).expect("Failed to add item to backpack");

			if pickup.collected_by == *player_entity {
				gamelog.entries.push(format!(
					"You pick up the {}.",
					names.get(pickup.item).unwrap().name
				));
			}
		}

		wants_pickup.clear();
	}
}

// Item Drop
// =========================================================================

pub struct ItemDropSystem {}

impl<'a> System<'a> for ItemDropSystem {
	type SystemData = (
		ReadExpect<'a, Entity>,
		WriteExpect<'a, GameLog>,
		Entities<'a>,
		WriteStorage<'a, WantsToDropItem>,
		ReadStorage<'a, Name>,
		WriteStorage<'a, Position>,
		WriteStorage<'a, InBackpack>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (
			player_entity, mut gamelog, entities, mut wants_drop, names,
			mut positions, mut backpack,
		) = data;

		for (entity, to_drop) in (&entities, &wants_drop).join() {
			let mut dropper_pos : Position = Position { x: 0, y: 0 };

			{
				let dropped_pos = positions.get(entity).unwrap();
				dropper_pos.x = dropped_pos.x;
				dropper_pos.y = dropped_pos.y;
			}

			positions.insert(
				to_drop.item,
				Position { x: dropper_pos.x, y: dropper_pos.y },
			).expect("Failed to insert drop position");
			backpack.remove(to_drop.item);

			if entity == *player_entity {
				gamelog.entries.push(format!(
					"You drop the {}",
					names.get(to_drop.item).unwrap().name
				));
			}
		}

		wants_drop.clear();
	}
}

// Item Use
// =========================================================================

pub struct ItemUseSystem {}

impl<'a> System<'a> for ItemUseSystem {
	type SystemData = (
		Entities<'a>,
		ReadExpect<'a, Map>,
		ReadExpect<'a, Entity>,
		WriteExpect<'a, GameLog>,
		WriteStorage<'a, WantsToUseItem>,
		ReadStorage<'a, Name>,
		ReadStorage<'a, ProvidesHealing>,
		WriteStorage<'a, CombatStats>,
		ReadStorage<'a, Consumable>,
		ReadStorage<'a, InflictsDamage>,
		WriteStorage<'a, SufferDamage>,
		ReadStorage<'a, AreaOfEffect>,
		WriteStorage<'a, Confusion>,
		ReadStorage<'a, Equippable>,
		WriteStorage<'a, Equipped>,
		WriteStorage<'a, InBackpack>,
		WriteExpect<'a, ParticleBuilder>,
		ReadStorage<'a, Position>,
		ReadStorage<'a, ProvidesFood>,
		WriteStorage<'a, HungerClock>,
		ReadStorage<'a, MagicMapper>,
		WriteExpect<'a, RunState>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (
			entities,
			map,
			player_entity,
			mut gamelog,
			mut wants_use,
			names,
			healing,
			mut combat_stats,
			consumables,
			inflict_damage,
			mut suffer_damage,
			aoe,
			mut confused,
			equippable,
			mut equipped,
			mut backpack,
			mut particle_builder,
			positions,
			provides_food,
			mut hunger_clock,
			magic_mapper,
			mut runstate,
		) = data;

		for (entity, item) in (&entities, &wants_use).join() {
			let mut used_item = true;

			// Targeting
			let mut targets : Vec<Entity> = Vec::new();
			match item.target {
				None => { targets.push(*player_entity) }
				Some(target) => {
					let area_effect = aoe.get(item.item);
					match area_effect {
						None => {
							// Single tile target
							let idx = map.xy_idx(target.x, target.y);
							for mob in map.tile_content[idx].iter() {
								targets.push(*mob);
							}
						}
						Some(area_effect) => {
							// AoE
							let mut blast_tiles = rltk::field_of_view(
								target,
								area_effect.radius,
								&*map
							);
							blast_tiles.retain(|p|
								p.x > 0 && p.x < map.width - 1
									&& p.y > 0 && p.y < map.height - 1
							);
							for tile_pos in blast_tiles.iter() {
								let idx = map.xy_idx(tile_pos.x, tile_pos.y);
								for mob in map.tile_content[idx].iter() {
									targets.push(*mob);
								}
								particle_builder.request(
									tile_pos.x, tile_pos.y,
									RGB::named(rltk::ORANGERED),
									RGB::named(rltk::BLACK),
									rltk::to_cp437('░'),
									150.,
								);
							}
						}
					}
				}
			}

			// Equipment
			let item_equippable = equippable.get(item.item);
			match item_equippable {
				None => {}
				Some(can_equip) => {
					let target_slot = can_equip.slot;
					let target = targets[0];

					// Remove any items the target has in the item's slot
					let mut to_unequip : Vec<Entity> = Vec::new();
					for (item_entity, already_equipped, name) in (&entities, &equipped, &names).join() {
						if already_equipped.owner == target && already_equipped.slot == target_slot {
							to_unequip.push(item_entity);

							if target == *player_entity {
								gamelog.entries.push(format!(
									"You unequip the {}",
									name.name,
								));
							}
						}
					}

					for item in to_unequip.iter() {
						equipped.remove(*item);
						backpack.insert(*item, InBackpack {
							owner: target,
						}).expect("Failed to move equipped to backpack");
					}

					// Wield item
					equipped.insert(item.item, Equipped {
						owner: target,
						slot: target_slot,
					}).expect("Failed to equip item");
					backpack.remove(item.item);

					if target == *player_entity {
						gamelog.entries.push(format!(
							"You equip the {}",
							names.get(item.item).unwrap().name,
						));
					}
				}
			}

			// Healing Item
			let heal_item = healing.get(item.item);
			match heal_item {
				None => {}
				Some(healer) => {
					for target in targets.iter() {
						let stats = combat_stats.get_mut(*target);
						if let Some(stats) = stats {
							stats.hp = i32::min(
								stats.max_hp,
								stats.hp + healer.heal_amount
							);

							if entity == *player_entity {
								gamelog.entries.push(format!(
									"You drink {}, healing {}hp",
									names.get(item.item).unwrap().name,
									healer.heal_amount,
								));
							}

							let pos = positions.get(*target);
							if let Some(pos) = pos {
								particle_builder.request(
									pos.x, pos.y,
									RGB::named(rltk::GREEN),
									RGB::named(rltk::BLACK),
									rltk::to_cp437('♥'),
									250.,
								);
							}
						}
					}
				}
			}

			// Damage item
			let damage_item = inflict_damage.get(item.item);
			match damage_item {
				None => {}
				Some(damage) => {
					used_item = false;

					for mob in targets.iter() {
						if combat_stats.get(*mob).is_none() { continue }

						SufferDamage::new_damage(
							&mut suffer_damage,
							*mob, damage.damage,
						);

						if entity == *player_entity {
							let mob_name = names.get(*mob).unwrap();
							let item_name = names.get(item.item).unwrap();
							gamelog.entries.push(format!(
								"You use {} on {}, dealing {}hp damage!",
								item_name.name,
								mob_name.name,
								damage.damage,
							));
						}

						used_item = true;

						let pos = positions.get(*mob);
						if let Some(pos) = pos {
							particle_builder.request(
								pos.x, pos.y,
								RGB::named(rltk::RED),
								RGB::named(rltk::BLACK),
								rltk::to_cp437('‼'),
								150.,
							);
						}
					}
				}
			}

			// Confusion
			let mut add_confusion = Vec::new();
			let causes_confusion = confused.get(item.item);
			match causes_confusion {
				None => {}
				Some(confusion) => {
					used_item = false;
					for mob in targets.iter() {
						add_confusion.push((*mob, confusion.turns));

						if entity == *player_entity {
							let mob_name = names.get(*mob).unwrap();
							let item_name = names.get(item.item).unwrap();
							gamelog.entries.push(format!(
								"You use {} on {}, confusing them!",
								item_name.name,
								mob_name.name,
							))
						}

						used_item = true;

						let pos = positions.get(*mob);
						if let Some(pos) = pos {
							particle_builder.request(
								pos.x, pos.y,
								RGB::named(rltk::BLUEVIOLET),
								RGB::named(rltk::BLACK),
								rltk::to_cp437('?'),
								250.,
							);
						}
					}
				}
			}
			for (target, turns) in add_confusion.iter() {
				confused.insert(
					*target,
					Confusion { turns: *turns },
				).expect("Failed to make confused");
			}

			// Map
			let is_map = magic_mapper.get(item.item);
			match is_map {
				None => {}
				Some(_) => {
					used_item = true;
					gamelog.entries.push("You see evErYTHING!".to_string());
					*runstate = RunState::MagicMapReveal { row: 0 };
				}
			}

			// Food
			let item_edible = provides_food.get(item.item);
			match item_edible {
				None => {}
				Some(_) => {
					used_item = true;
					let target = targets[0];
					let hc = hunger_clock.get_mut(target);
					if let Some(hc) = hc {
						hc.state = HungerState::WellFed;
						hc.duration = 20;
						gamelog.entries.push(
							format!(
								"You eat the {}",
								names.get(item.item).unwrap().name,
							)
						);
					}
				}
			}

			// Consumable
			if used_item {
				let consumable = consumables.get(item.item);
				match consumable {
					None => {}
					Some(_) => {
						entities.delete(item.item).expect("Failed to delete item");
					}
				}
			}
		}

		wants_use.clear();
	}
}

// Item Remove System
// =========================================================================

pub struct ItemRemoveSystem {}

impl<'a> System<'a> for ItemRemoveSystem {
	type SystemData = (
		Entities<'a>,
		WriteStorage<'a, WantsToRemoveItem>,
		WriteStorage<'a, Equipped>,
		WriteStorage<'a, InBackpack>,
		WriteExpect<'a, GameLog>,
		ReadStorage<'a, Name>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (entities, mut wants_remove, mut equipped, mut backpack, mut log, names) = data;

		for (entity, to_remove) in (&entities, &wants_remove).join() {
			equipped.remove(to_remove.item);
			backpack.insert(to_remove.item, InBackpack {
				owner: entity,
			}).expect("Failed to put unequipped item in backpack");
			log.entries.push(format!(
				"You remove the {}",
				names.get(to_remove.item).unwrap().name,
			));
		}

		wants_remove.clear();
	}
}
