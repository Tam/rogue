use std::fs;
use std::fs::File;
use std::ops::Deref;
use std::path::Path;
use rltk::Point;
use specs::{Builder, Entity, Join, World, WorldExt};
use specs::saveload::{MarkedBuilder, SimpleMarker, SerializeComponents, DeserializeComponents, SimpleMarkerAllocator};
#[allow(deprecated)] use specs::error::NoError;
use crate::map::Map;
use crate::{MAP_SIZE, SerializationHelper, SerializeMe};
use crate::components::*;
use crate::gamelog::GameLog;

macro_rules! serialize_individually {
	($ecs:expr, $ser:expr, $data:expr, $($type:ty), * $(,)?) => { $(
		#[allow(deprecated)]
		SerializeComponents::<NoError, SimpleMarker<SerializeMe>>::serialize(
			&($ecs.read_storage::<$type>(), ),
			&$data.0,
			&$data.1,
			&mut $ser,
		).unwrap();
	)* };
}

macro_rules! deserialize_individually {
	($ecs:expr, $de:expr, $data:expr, $($type:ty), * $(,)?) => { $(
		#[allow(deprecated)]
		DeserializeComponents::<NoError, _>::deserialize(
			&mut ( &mut $ecs.write_storage::<$type>(), ),
			&mut $data.0, // Entities
			&mut $data.1, // Marker
			&mut $data.2, // Allocator
			&mut $de,
		).unwrap();
	)* };
}

pub fn save_game (ecs: &mut World) {
	// Create helper
	let mapcopy = ecs.get_mut::<Map>().unwrap().clone();
	let logcopy = ecs.fetch::<GameLog>().deref().clone();
	let savehelper = ecs.create_entity()
		.with(SerializationHelper {
			map: mapcopy,
			log: logcopy,
		})
		.marked::<SimpleMarker<SerializeMe>>()
		.build();

	// Actually Serialize
	{
		let data = (ecs.entities(), ecs.read_storage::<SimpleMarker<SerializeMe>>());
		let writer = File::create("./savegame.json").unwrap();
		let mut serializer = serde_json::Serializer::new(writer);
		serialize_individually!(
			ecs, serializer, data,
			Player,
			Monster,
			BlocksTile,
			Item,
			Consumable,
			Name,
			Position,
			Renderable,
			Viewshed,
			ParticleLifetime,
			Hidden,
			EntityTrigger,
			EntityMoved,
			SingleActivation,
			CombatStats,
			InflictsDamage,
			SufferDamage,
			Ranged,
			AreaOfEffect,
			HungerClock,
			ProvidesFood,
			WantsToMelee,
			WantsToPickupItem,
			WantsToDropItem,
			WantsToUseItem,
			WantsToRemoveItem,
			InBackpack,
			ProvidesHealing,
			Confusion,
			SerializationHelper,
			Equippable,
			Equipped,
			MeleePowerBonus,
			DefenseBonus,
			MagicMapper,
		);
	}

	// Cleanup
	ecs.delete_entity(savehelper).expect("Failed to cleanup after save");
}

pub fn does_save_exist () -> bool { Path::new("./savegame.json").exists() }

pub fn load_game (ecs: &mut World) {
	// Delete everything
	{
		let mut to_delete = Vec::new();
		for e in ecs.entities().join() { to_delete.push(e) }
		for del in to_delete.iter() {
			ecs.delete_entity(*del).expect("Deletion Failed")
		}
	}

	let data = fs::read_to_string("./savegame.json").unwrap();
	let mut de = serde_json::Deserializer::from_str(&data);

	{
		let mut d = (
			&mut ecs.entities(),
			&mut ecs.write_storage::<SimpleMarker<SerializeMe>>(),
			&mut ecs.write_resource::<SimpleMarkerAllocator<SerializeMe>>(),
		);

		deserialize_individually!(
			ecs, de, d,
			Player,
			Monster,
			BlocksTile,
			Item,
			Consumable,
			Name,
			Position,
			Renderable,
			Viewshed,
			ParticleLifetime,
			Hidden,
			EntityTrigger,
			EntityMoved,
			SingleActivation,
			CombatStats,
			InflictsDamage,
			SufferDamage,
			Ranged,
			AreaOfEffect,
			HungerClock,
			ProvidesFood,
			WantsToMelee,
			WantsToPickupItem,
			WantsToDropItem,
			WantsToUseItem,
			WantsToRemoveItem,
			InBackpack,
			ProvidesHealing,
			Confusion,
			SerializationHelper,
			Equippable,
			Equipped,
			MeleePowerBonus,
			DefenseBonus,
			MagicMapper,
		);
	}

	let mut deleteme : Option<Entity> = None;
	{
		let entities = ecs.entities();
		let helper = ecs.read_storage::<SerializationHelper>();
		let player = ecs.read_storage::<Player>();
		let position = ecs.read_storage::<Position>();

		for (e, h) in (&entities, &helper).join() {
			let mut worldmap = ecs.write_resource::<Map>();
			*worldmap = h.map.clone();
			worldmap.tile_content = vec![Vec::new(); MAP_SIZE];

			let mut log = ecs.write_resource::<GameLog>();
			*log = h.log.clone();

			deleteme = Some(e);
		}

		for (e, _p, pos) in (&entities, &player, &position).join() {
			let mut ppos = ecs.write_resource::<Point>();
			*ppos = Point::new(pos.x, pos.y);
			let mut player_resource = ecs.write_resource::<Entity>();
			*player_resource = e;
		}
	}

	ecs.delete_entity(deleteme.unwrap())
		.expect("Failed to delete load helper");
}

pub fn delete_save () {
	if does_save_exist() {
		fs::remove_file("./savegame.json")
			.expect("Failed to delete save");
	}
}