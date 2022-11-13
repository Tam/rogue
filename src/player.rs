use std::cmp::{max, min};
use rltk::{Point, Rltk, VirtualKeyCode};
use specs::prelude::*;
use crate::{CombatStats, EntityMoved, HungerClock, HungerState, Item, Monster, RunState, TileType, Viewshed, WantsToMelee, WantsToPickupItem};
use crate::gamelog::GameLog;
use crate::map::Map;
use super::{Player, Position, State};

pub fn try_move_player (delta_x: i32, delta_y: i32, ecs: &mut World) {
	let mut positions = ecs.write_storage::<Position>();
	let players = ecs.read_storage::<Player>();
	let mut viewsheds = ecs.write_storage::<Viewshed>();
	let combat_stats = ecs.read_storage::<CombatStats>();
	let map = ecs.fetch::<Map>();
	let entities = ecs.entities();
	let mut wants_to_melee = ecs.write_storage::<WantsToMelee>();
	let mut entity_moved = ecs.write_storage::<EntityMoved> ();

	for (entity, _player, pos, viewshed)
	 in (&entities, &players, &mut positions, &mut viewsheds).join()
	{
		if pos.x + delta_x < 1
			|| pos.x + delta_x > map.width - 1
			|| pos.y + delta_y < 1
			|| pos.y + delta_y > map.height - 1
		{ return; }

		let destination_idx = map.xy_idx(pos.x + delta_x, pos.y + delta_y);

		for potential_target in map.tile_content[destination_idx].iter() {
			let target = combat_stats.get(*potential_target);
			if let Some(_t) = target {
				wants_to_melee.insert(
					entity,
					WantsToMelee { target: *potential_target }
				).expect("Add melee target failed");
				return;
			}
		}

		if !map.blocked[destination_idx] {
			pos.x = min(79, max(0, pos.x + delta_x));
			pos.y = min(49, max(0, pos.y + delta_y));

			let mut ppos = ecs.write_resource::<Point>();
			ppos.x = pos.x;
			ppos.y = pos.y;

			viewshed.dirty = true;
			entity_moved.insert(entity, EntityMoved {})
				.expect("Failed to use legs");
		}
	}
}

pub fn player_input (gs: &mut State, ctx: &mut Rltk) -> RunState {
	// Movement
	match ctx.key {
		None => { return RunState::AwaitingInput }
		Some(key) => match key {
			// Cardinal
			VirtualKeyCode::W => try_move_player(0, -1, &mut gs.ecs),
			VirtualKeyCode::A => try_move_player(-1, 0, &mut gs.ecs),
			VirtualKeyCode::S => try_move_player(0, 1, &mut gs.ecs),
			VirtualKeyCode::D => try_move_player(1, 0, &mut gs.ecs),

			// Diagonal
			VirtualKeyCode::E => try_move_player(1, -1, &mut gs.ecs),
			VirtualKeyCode::Q => try_move_player(-1, -1, &mut gs.ecs),
			VirtualKeyCode::C => try_move_player(1, 1, &mut gs.ecs),
			VirtualKeyCode::Z => try_move_player(-1, 1, &mut gs.ecs),

			// Pickup / Interact
			VirtualKeyCode::F => {
				if try_next_level(&mut gs.ecs) {
					return RunState::NextLevel;
				} else {
					get_item(&mut gs.ecs)
				}
			},

			// Place (drop)
			VirtualKeyCode::P => return RunState::ShowDropItem,

			// Inventory
			VirtualKeyCode::I => return RunState::ShowInventory,

			// Equipped Items
			VirtualKeyCode::R => return RunState::ShowRemoveItem,

			// Save & Quit
			VirtualKeyCode::Escape => return RunState::SaveGame,

			// [DEBUG] Skip Level
			VirtualKeyCode::F12 => return RunState::NextLevel,

			// Skip Turn
			VirtualKeyCode::Space => return skip_turn(&mut gs.ecs),

			_ => { return RunState::AwaitingInput },
		}
	}

	return RunState::PlayerTurn;
}

fn try_next_level (ecs: &mut World) -> bool {
	let player_pos = ecs.fetch::<Point>();
	let map = ecs.fetch::<Map>();
	let player_idx = map.xy_idx(player_pos.x, player_pos.y);

	return map.tiles[player_idx] == TileType::DownStairs;
}

fn get_item (ecs: &mut World) {
	let player_pos = ecs.fetch::<Point>();
	let player_entity = ecs.fetch::<Entity>();
	let entities = ecs.entities();
	let items = ecs.read_storage::<Item>();
	let positions = ecs.read_storage::<Position>();
	let mut gamelog = ecs.fetch_mut::<GameLog>();

	let mut target_item : Option<Entity> = None;

	for (item_entity, _, position) in (&entities, &items, &positions).join() {
		if position.x == player_pos.x && position.y == player_pos.y {
			target_item = Some(item_entity);
		}
	}

	match target_item {
		None => gamelog.entries.push("There's nothing to pick up here!".to_string()),
		Some(item) => {
			let mut pickup = ecs.write_storage::<WantsToPickupItem>();
			pickup.insert(*player_entity, WantsToPickupItem {
				item,
				collected_by: *player_entity,
			}).expect("Failed to add want pickup to player");
		},
	}
}

fn skip_turn (ecs: &mut World) -> RunState {
	let player_entity = ecs.fetch::<Entity>();
	let hunger = ecs.read_storage::<HungerClock>();
	let mut gamelog = ecs.fetch_mut::<GameLog>();

	let hc = hunger.get(*player_entity);
	if let Some(hc) = hc {
		if hc.state == HungerState::Hungry || hc.state == HungerState::Starving {
			gamelog.entries.push(
				"Your want for food prevents you from resting".to_string()
			);
			return RunState::PlayerTurn;
		}
	}

	let viewsheds = ecs.read_storage::<Viewshed>();
	let monsters = ecs.read_storage::<Monster>();
	let worldmap_res = ecs.fetch::<Map>();

	let viewshed = viewsheds.get(*player_entity).unwrap();
	for tile in viewshed.visible_tiles.iter() {
		let idx = worldmap_res.xy_idx(tile.x, tile.y);
		for entity_id in worldmap_res.tile_content[idx].iter() {
			let mob = monsters.get(*entity_id);
			if mob.is_some() {
				gamelog.entries.push(
					"The sounds of nearby monsters keep you on edge!".to_string()
				);
				return RunState::PlayerTurn;
			}
		}
	}

	let mut stats = ecs.write_storage::<CombatStats>();
	let player_hp = stats.get_mut(*player_entity).unwrap();
	if player_hp.hp == player_hp.max_hp {
		gamelog.entries.push("You rest for a moment.".to_string());
	} else {
		player_hp.hp += 1;
		gamelog.entries.push("You rest for a moment, gaining 1hp.".to_string());
	}

	return RunState::PlayerTurn;
}