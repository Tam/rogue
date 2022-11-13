use std::collections::HashMap;
use rltk::{RandomNumberGenerator, RGB};
use specs::prelude::*;
use specs::saveload::{MarkedBuilder, SimpleMarker};
use crate::{AreaOfEffect, BlocksTile, CombatStats, Confusion, Consumable, DefenseBonus, EntityTrigger, EquipmentSlot, Equippable, Hidden, HungerClock, HungerState, InflictsDamage, Item, MagicMapper, MeleePowerBonus, Monster, Name, Player, Position, ProvidesFood, ProvidesHealing, Ranged, Renderable, SerializeMe, SingleActivation, TileType, Viewshed};
use crate::map::Map;
use crate::random_table::RandomTable;
use crate::rect::Rect;

const MAX_SPAWNS_PER_AREA : i32 = 4;

// Player
// =========================================================================

/// Spawn player entity
pub fn player (ecs: &mut World, player_x: i32, player_y: i32) -> Entity {
	ecs
		.create_entity()
		.with(Position { x: player_x, y: player_y })
		.with(Renderable {
			glyph: rltk::to_cp437('@'),
			fg: RGB::named(rltk::YELLOW),
			bg: RGB::named(rltk::BLACK),
			render_order: 0,
		})
		.with(Player {})
		.with(Viewshed {
			visible_tiles: Vec::new(),
			range: 8,
			dirty: true,
		})
		.with(Name { name: "you".to_string() })
		.with(CombatStats {
			max_hp: 30,
			hp: 30,
			defence: 2,
			power: 5,
		})
		.with(HungerClock {
			state: HungerState::WellFed,
			duration: 20,
		})
		.marked::<SimpleMarker<SerializeMe>>()
		.build()
}

// Mobs
// =========================================================================

// Monsters
fn orc (ecs: &mut World, x: i32, y: i32) { monster(ecs, x, y, 'o', "Ork") }
fn goblin (ecs: &mut World, x: i32, y: i32) { monster(ecs, x, y, 'g', "Goblin") }

/// Spawns monster entity
fn monster<S : ToString> (
	ecs: &mut World,
	x: i32, y: i32,
	glyph: char,
	name: S,
) {
	ecs
		.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437(glyph),
			fg: RGB::named(rltk::RED),
			bg: RGB::named(rltk::BLACK),
			render_order: 1,
		})
		.with(Viewshed {
			visible_tiles: Vec::new(),
			range: 8,
			dirty: true,
		})
		.with(Monster {})
		.with(Name { name: format!("{}", name.to_string()) })
		.with(BlocksTile {})
		.with(CombatStats {
			max_hp: 16,
			hp: 16,
			defence: 1,
			power: 4,
		})
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

// Items
// =========================================================================

fn health_potion (ecs: &mut World, x: i32, y: i32) {
	ecs.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437('¡'),
			fg: RGB::named(rltk::RED2),
			bg: RGB::named(rltk::BLACK),
			render_order: 2,
		})
		.with(Name { name: "Health Potion".to_string() })
		.with(Item {})
		.with(Consumable {})
		.with(ProvidesHealing { heal_amount: 8 })
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

fn magic_missile_scroll (ecs: &mut World, x: i32, y: i32) {
	ecs.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437('~'),
			fg: RGB::named(rltk::CYAN),
			bg: RGB::named(rltk::BLACK),
			render_order: 2,
		})
		.with(Name { name: "Magic Missile Scroll".to_string() })
		.with(Item {})
		.with(Consumable {})
		.with(Ranged { range: 6 })
		.with(InflictsDamage { damage: 8 })
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

fn fireball_scroll (ecs: &mut World, x: i32, y: i32) {
	ecs.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437('~'),
			fg: RGB::named(rltk::ORANGE),
			bg: RGB::named(rltk::BLACK),
			render_order: 2,
		})
		.with(Name { name: "Fireball Scroll".to_string() })
		.with(Item {})
		.with(Consumable {})
		.with(Ranged { range: 6 })
		.with(InflictsDamage { damage: 20 })
		.with(AreaOfEffect { radius: 3 })
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

fn confusion_scroll (ecs: &mut World, x: i32, y: i32) {
	ecs.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437('~'),
			fg: RGB::named(rltk::PINK),
			bg: RGB::named(rltk::BLACK),
			render_order: 2,
		})
		.with(Name { name: "Confusion Scroll".to_string() })
		.with(Item {})
		.with(Consumable {})
		.with(Ranged { range: 6 })
		.with(Confusion { turns: 4 })
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

fn magic_mapping_scroll (ecs: &mut World, x: i32, y: i32) {
	ecs.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437('~'),
			fg: RGB::named(rltk::CYAN3),
			bg: RGB::named(rltk::BLACK),
			render_order: 2,
		})
		.with(Name { name: "Scroll of Mapping".to_string() })
		.with(Item {})
		.with(Consumable {})
		.with(MagicMapper {})
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

fn rations (ecs: &mut World, x: i32, y: i32) {
	ecs.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437('%'),
			fg: RGB::named(rltk::LIME_GREEN),
			bg: RGB::named(rltk::BLACK),
			render_order: 2,
		})
		.with(Name { name: "Rations".to_string() })
		.with(Item {})
		.with(ProvidesFood {})
		.with(Consumable {})
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

// Equippables
// =========================================================================

fn dagger (ecs: &mut World, x: i32, y: i32) {
	ecs.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437('►'),
			fg: RGB::named(rltk::CYAN),
			bg: RGB::named(rltk::BLACK),
			render_order: 2,
		})
		.with(Name { name: "Dagger".to_string() })
		.with(Item {})
		.with(Equippable { slot: EquipmentSlot::Melee })
		.with(MeleePowerBonus { power: 2 })
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

fn shield (ecs: &mut World, x: i32, y: i32) {
	ecs.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437('('),
			fg: RGB::named(rltk::CYAN),
			bg: RGB::named(rltk::BLACK),
			render_order: 2,
		})
		.with(Name { name: "Shield".to_string() })
		.with(Item {})
		.with(Equippable { slot: EquipmentSlot::Shield })
		.with(DefenseBonus { defense: 1 })
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

fn longsword (ecs: &mut World, x: i32, y: i32) {
	ecs.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437('/'),
			fg: RGB::named(rltk::CYAN),
			bg: RGB::named(rltk::BLACK),
			render_order: 2,
		})
		.with(Name { name: "Longsword".to_string() })
		.with(Item {})
		.with(Equippable { slot: EquipmentSlot::Melee })
		.with(MeleePowerBonus { power: 4 })
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

fn tower_shield (ecs: &mut World, x: i32, y: i32) {
	ecs.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437('['),
			fg: RGB::named(rltk::CYAN),
			bg: RGB::named(rltk::BLACK),
			render_order: 2,
		})
		.with(Name { name: "Tower Shield".to_string() })
		.with(Item {})
		.with(Equippable { slot: EquipmentSlot::Shield })
		.with(DefenseBonus { defense: 3 })
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

// Traps
// =========================================================================

fn bear_trap (ecs: &mut World, x: i32, y: i32) {
	ecs.create_entity()
		.with(Position { x, y })
		.with(Renderable {
			glyph: rltk::to_cp437('^'),
			fg: RGB::named(rltk::RED),
			bg: RGB::named(rltk::BLACK),
			render_order: 2,
		})
		.with(Name { name: "Bear Trap".to_string() })
		.with(Hidden {})
		.with(EntityTrigger {})
		.with(SingleActivation {})
		.with(InflictsDamage { damage: 6 })
		.marked::<SimpleMarker<SerializeMe>>()
		.build();
}

// Rooms
// =========================================================================

fn room_table (map_depth: i32) -> RandomTable {
	RandomTable::new()
		.add("Goblin", 10)
		.add("Orc", 1 + map_depth)
		.add("Health Potion", 7)
		.add("Fireball Scroll", 2 + map_depth)
		.add("Confusion Scroll", 2 + map_depth)
		.add("Magic Missile Scroll", 4)
		.add("Dagger", 3)
		.add("Shield", 3)
		.add("Long Sword", map_depth - 1)
		.add("Tower Shield", map_depth - 1)
		.add("Rations", 10)
		.add("Magic Mapping Scroll", 2)
		.add("Bear Trap", 2)
}

/// Spawns a named entity at the given map IDx
/// spawn: (idx, name)
fn spawn_entity (ecs: &mut World, spawn: &(&usize, &String), map: &Map) {
	let x = (*spawn.0 % map.width as usize) as i32;
	let y = (*spawn.0 / map.width as usize) as i32;

	match spawn.1.as_ref() {
		"Goblin" => goblin(ecs, x, y),
		"Orc" => orc(ecs, x, y),
		"Health Potion" => health_potion(ecs, x, y),
		"Fireball Scroll" => fireball_scroll(ecs, x, y),
		"Confusion Scroll" => confusion_scroll(ecs, x, y),
		"Magic Missile Scroll" => magic_missile_scroll(ecs, x, y),
		"Dagger" => dagger(ecs, x, y),
		"Shield" => shield(ecs, x, y),
		"Long Sword" => longsword(ecs, x, y),
		"Tower Shield" => tower_shield(ecs, x, y),
		"Rations" => rations(ecs, x, y),
		"Magic Mapping Scroll" => magic_mapping_scroll(ecs, x, y),
		"Bear Trap" => bear_trap(ecs, x, y),
		_ => {}
	}
}

pub fn spawn_region (ecs: &mut World, area: &[usize], depth: i32, map: &Map) {
	let spawn_table = room_table(depth);
	let mut spawn_points : HashMap<usize, String> = HashMap::new();
	let mut areas : Vec<usize> = Vec::from(area);

	{
		let mut rng = ecs.write_resource::<RandomNumberGenerator>();
		let num_spawns = i32::min(
			areas.len() as i32,
			rng.roll_dice(1, MAX_SPAWNS_PER_AREA + 3) + (depth - 1) - 3,
		);

		if num_spawns == 0 { return; }

		for _i in 0 .. num_spawns {
			let index =
				if areas.len() == 1 { 0usize }
				else { (rng.roll_dice(1, areas.len() as i32) - 1) as usize };
			let map_idx = areas[index];

			spawn_points.insert(map_idx, spawn_table.roll(&mut rng));
			areas.remove(index);
		}
	}

	for spawn in spawn_points.iter() { spawn_entity(ecs, &spawn, map) }
}

/// Spawns a room with stuff in it
pub fn spawn_room (ecs: &mut World, room: &Rect, map_depth: i32, map: &Map) {
	let mut possible_targets : Vec<usize> = Vec::new();

	{
		let map = ecs.fetch::<Map>();
		for y in room.y1 + 1 .. room.y2 {
			for x in room.x1 + 1 .. room.x2 {
				let idx = map.xy_idx(x, y);
				if map.tiles[idx] == TileType::Floor {
					possible_targets.push(idx);
				}
			}
		}
	}

	spawn_region(ecs, &possible_targets, map_depth, map);
}