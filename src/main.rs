extern crate serde;

pub mod components;
pub mod map;
pub mod player;
pub mod rect;
pub mod visibility_system;
pub mod monster_ai_system;
pub mod map_indexing_system;
pub mod melee_combat_system;
pub mod damage_system;
pub mod gui;
pub mod gamelog;
pub mod spawner;
pub mod inventory_system;
pub mod saveload_system;
pub mod random_table;
pub mod particle_system;
pub mod hunger_system;
pub mod trigger_system;
pub mod map_builder;

pub use components::*;
pub use map::*;
pub use player::*;

use rltk::{Rltk, GameState, RGB, Point, RandomNumberGenerator, VirtualKeyCode};
use crate::map::Map;
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};
use crate::damage_system::DamageSystem;
use crate::gamelog::GameLog;
use crate::gui::{draw_main_menu, drop_item_menu, ItemMenuResult, MainMenuResult, MainMenuSelection, ranged_target, show_inventory};
use crate::hunger_system::HungerSystem;
use crate::inventory_system::{ItemCollectionSystem, ItemDropSystem, ItemRemoveSystem, ItemUseSystem};
use crate::map_indexing_system::MapIndexingSystem;
use crate::melee_combat_system::MeleeCombatSystem;
use crate::monster_ai_system::MonsterAI;
use crate::trigger_system::TriggerSystem;
use crate::visibility_system::VisibilitySystem;

// Game
// =========================================================================

#[derive(PartialEq, Copy, Clone)]
pub enum RunState {
    PreRun,
    AwaitingInput,
    PlayerTurn,
    MonsterTurn,
    ShowInventory,
    ShowDropItem,
    ShowRemoveItem,
    ShowTargeting {
        range : i32,
        item  : Entity,
    },
    MainMenu {
        menu_selection: MainMenuSelection,
    },
    SaveGame,
    NextLevel,
    GameOver,
    MagicMapReveal { row: i32 },
    #[cfg(feature = "mapgen_visualiser")] MapGeneration,
}

pub struct State {
    pub ecs: World,

    #[cfg(feature = "mapgen_visualiser")] mapgen_running : bool,
    #[cfg(feature = "mapgen_visualiser")] mapgen_history : Vec<Map>,
    #[cfg(feature = "mapgen_visualiser")] mapgen_index   : usize,
    #[cfg(feature = "mapgen_visualiser")] mapgen_timer   : f32,
}

impl State {
    fn run_systems (&mut self) {
        let mut vis = VisibilitySystem {};
        vis.run_now(&self.ecs);

        let mut mob = MonsterAI {};
        mob.run_now(&self.ecs);

        let mut triggers = TriggerSystem {};
        triggers.run_now(&self.ecs);

        let mut mapindex = MapIndexingSystem {};
        mapindex.run_now(&self.ecs);

        let mut melee = MeleeCombatSystem {};
        melee.run_now(&self.ecs);

        let mut damage = DamageSystem {};
        damage.run_now(&self.ecs);

        let mut pickup = ItemCollectionSystem {};
        pickup.run_now(&self.ecs);

        let mut drop = ItemDropSystem {};
        drop.run_now(&self.ecs);

        let mut item_use = ItemUseSystem {};
        item_use.run_now(&self.ecs);

        let mut item_remove = ItemRemoveSystem {};
        item_remove.run_now(&self.ecs);

        let mut hunger = HungerSystem {};
        hunger.run_now(&self.ecs);

        // Last
        let mut particles = particle_system::ParticleSpawnSystem {};
        particles.run_now(&self.ecs);

        self.ecs.maintain();
    }

    fn entities_to_remove_on_level_change (&mut self) -> Vec<Entity> {
        let entities = self.ecs.entities();
        let player = self.ecs.read_storage::<Player>();
        let backpack = self.ecs.read_storage::<InBackpack>();
        let player_entity = self.ecs.fetch::<Entity>();
        let equipped = self.ecs.read_storage::<Equipped>();

        let mut to_delete : Vec<Entity> = Vec::new();
        for entity in entities.join() {
            // Don't delete the player
            let p = player.get(entity);
            if let Some(_p) = p { continue }

            // Don't delete inventory items
            let i = backpack.get(entity);
            if let Some(i) = i {
                if i.owner == *player_entity { continue }
            }

            // Don't delete equipped
            let e = equipped.get(entity);
            if let Some(e) = e {
                if e.owner == *player_entity { continue }
            }

            to_delete.push(entity);
        }

        return to_delete;
    }

    fn goto_next_level(&mut self) {
        // Delete all entities not related to the player
        let to_delete = self.entities_to_remove_on_level_change();
        for target in to_delete {
            self.ecs.delete_entity(target)
                .expect("Failed to delete old entity on level change");
        }

        // Generate map
        let current_depth;
        {
            let worldmap_res = self.ecs.fetch::<Map>();
            current_depth = worldmap_res.depth;
        }
        self.generate_world_map(current_depth + 1);

        // Notify the player
        let mut gamelog = self.ecs.fetch_mut::<GameLog>();
        gamelog.entries.push("You descend, taking a moment to catch your breath...".to_string());

        // Heal the player
        let player_entity = self.ecs.fetch::<Entity>();
        let mut hp_store = self.ecs.write_storage::<CombatStats>();
        let player_hp = hp_store.get_mut(*player_entity);
        if let Some(player_hp) = player_hp {
            player_hp.hp = i32::max(player_hp.hp, player_hp.max_hp / 2);
        }
    }

    fn game_over_cleanup(&mut self) {
        // Delete all the things
        let mut to_delete = Vec::new();
        for e in self.ecs.entities().join() {
            to_delete.push(e);
        }
        for del in to_delete.iter() {
            self.ecs.delete_entity(*del).expect("Delete failed");
        }

        // Spawn new player
        {
            let player_entity = spawner::player(&mut self.ecs, 0, 0);
            let mut player_writer = self.ecs.write_resource::<Entity>();
            *player_writer = player_entity;
        }

        // Generate map
        self.generate_world_map(1);
    }

    fn generate_world_map (&mut self, depth: i32) {
        #[cfg(feature = "mapgen_visualiser")]
        {
            self.mapgen_running = true;
            self.mapgen_index = 0;
            self.mapgen_timer = 0.;
            self.mapgen_history.clear();
        }

        let mut builder = map_builder::random_builder(depth);
        builder.build();

        #[cfg(feature = "mapgen_visualiser")]
        { self.mapgen_history = builder.get_snapshot_history(); }

        let player_start;
        {
            let mut worldmap = self.ecs.write_resource::<Map>();
            *worldmap = builder.get_map();
            player_start = builder.get_starting_position();
        }

        // Spawn entities
        builder.spawn(&mut self.ecs);

        // Place player
        let mut player_pos = self.ecs.write_resource::<Point>();
        *player_pos = Point::new(player_start.x, player_start.y);

        let mut pos_comps = self.ecs.write_storage::<Position>();
        let player_entity = self.ecs.fetch::<Entity>();
        let player_pos_comp = pos_comps.get_mut(*player_entity);
        if let Some(player_pos_comp) = player_pos_comp {
            player_pos_comp.x = player_start.x;
            player_pos_comp.y = player_start.y;
        }

        let mut viewsheds = self.ecs.write_storage::<Viewshed>();
        let vs = viewsheds.get_mut(*player_entity);
        if let Some(vs) = vs {
            vs.dirty = true;
        }
    }
}

impl GameState for State {
    fn tick (&mut self, ctx : &mut Rltk) {
        // Get current state
        let mut new_runstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            new_runstate = *runstate;
        }

        // Clear console
        ctx.cls();
        particle_system::cull_dead_particles(&mut self.ecs, ctx);

        // Render game (or not)
        match new_runstate {
            RunState::MainMenu { .. } => {}
            RunState::GameOver { .. } => {}
            #[cfg(feature = "mapgen_visualiser")]
            RunState::MapGeneration => {
                draw_map(&self.mapgen_history[self.mapgen_index], ctx);

                if self.mapgen_running {
                    self.mapgen_timer += ctx.frame_time_ms;
                    if self.mapgen_timer > 200. {
                        self.mapgen_timer = 0.;
                        self.mapgen_index += 1;
                        if self.mapgen_index >= self.mapgen_history.len() {
                            self.mapgen_index = self.mapgen_history.len() - 1;
                            self.mapgen_running = false;
                        }
                    }
                } else {
                    ctx.print_color_centered(
                        MAP_HEIGHT + 2,
                        RGB::named(rltk::SPRINGGREEN),
                        RGB::named(rltk::BLACK),
                        " Map Generated ",
                    );
                    ctx.print_color_centered(
                        MAP_HEIGHT + 4,
                        RGB::named(rltk::GREY),
                        RGB::named(rltk::BLACK),
                        " Press SPACE to regenerate ",
                    );
                    if ctx.key.unwrap_or(VirtualKeyCode::Key0) == VirtualKeyCode::Space {
                        self.generate_world_map(1);
                    }
                }
            }
            _ => {
                draw_map(&self.ecs.fetch::<Map>(), ctx);

                {
                    let positions = self.ecs.read_storage::<Position>();
                    let renderables = self.ecs.read_storage::<Renderable>();
                    let hidden = self.ecs.read_storage::<Hidden>();
                    let map = self.ecs.fetch::<Map>();

                    let mut data = (&positions, &renderables, !&hidden).join().collect::<Vec<_>>();
                    data.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order));
                    for (pos, render, _hidden) in data.iter() {
                        let idx = map.xy_idx(pos.x, pos.y);
                        if map.visible_tiles[idx] {
                            let mut bg = render.bg;

                            // Show bloodstain on entity that doesn't have a background
                            if bg == RGB::named(rltk::BLACK) && map.bloodstains.contains(&idx) {
                                bg = RGB::named(rltk::DARK_RED);
                            }

                            ctx.set(pos.x, pos.y, render.fg, bg, render.glyph);
                        }
                    }
                }
            }
        }

        // Handle states
        match new_runstate {
            RunState::PreRun => {
                self.run_systems();
                new_runstate = RunState::AwaitingInput;
            }
            RunState::AwaitingInput => {
                new_runstate = player_input(self, ctx);
            }
            RunState::PlayerTurn => {
                self.run_systems();
                match *self.ecs.fetch::<RunState>() {
                    RunState::MagicMapReveal {..} => new_runstate = RunState::MagicMapReveal { row: 0 },
                    _ => new_runstate = RunState::MonsterTurn
                }
            }
            RunState::MonsterTurn => {
                self.run_systems();
                new_runstate = RunState::AwaitingInput;
            }
            RunState::ShowInventory => {
                let result = show_inventory(self, ctx);
                match result.0 {
                    ItemMenuResult::Cancel => new_runstate = RunState::AwaitingInput,
                    ItemMenuResult::NoResponse => {},
                    ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let is_ranged = self.ecs.read_storage::<Ranged>();
                        let is_item_ranged = is_ranged.get(item_entity);
                        if let Some(is_item_ranged) = is_item_ranged {
                            new_runstate = RunState::ShowTargeting {
                                range: is_item_ranged.range,
                                item: item_entity,
                            };
                        } else {
                            let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                            intent.insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToUseItem {
                                    item: item_entity,
                                    target: None,
                                },
                            ).expect("Failed to insert drink intent");
                            new_runstate = RunState::PlayerTurn;
                        }
                    },
                }
            }
            RunState::ShowRemoveItem => {
                let result = gui::remove_item_menu(self, ctx);
                match result.0 {
                    ItemMenuResult::Cancel => new_runstate = RunState::AwaitingInput,
                    ItemMenuResult::NoResponse => {}
                    ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToRemoveItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToRemoveItem {
                            item: item_entity,
                        }).expect("Failed to unequip item");
                        new_runstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::ShowDropItem => {
                let result = drop_item_menu(self, ctx);
                match result.0 {
                    ItemMenuResult::Cancel => new_runstate = RunState::AwaitingInput,
                    ItemMenuResult::NoResponse => {},
                    ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                        intent.insert(
                            *self.ecs.fetch::<Entity>(),
                            WantsToDropItem { item: item_entity },
                        ).expect("Failed to insert drop intent");
                        new_runstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::ShowTargeting { range, item } => {
                let target = ranged_target(self, ctx, range);
                match target.0 {
                    ItemMenuResult::Cancel => new_runstate = RunState::AwaitingInput,
                    ItemMenuResult::NoResponse => {}
                    ItemMenuResult::Selected => {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent.insert(
                            *self.ecs.fetch::<Entity>(),
                            WantsToUseItem {
                                item,
                                target: target.1,
                            },
                        ).expect("Failed to insert use intent");
                        new_runstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::MagicMapReveal { row } => {
                let mut map = self.ecs.fetch_mut::<Map>();
                for x in 0..MAP_WIDTH {
                    let idx = map.xy_idx(x as i32, row);
                    map.revealed_tiles[idx] = true;
                    if row as usize == MAP_HEIGHT - 1 {
                        new_runstate = RunState::MonsterTurn;
                    } else {
                        new_runstate = RunState::MagicMapReveal { row: row + 1 };
                    }
                }
            }
            RunState::MainMenu { .. } => {
                let result = draw_main_menu(&self, ctx);
                match result {
                    MainMenuResult::NoSelection { selected } => {
                        new_runstate = RunState::MainMenu {
                            menu_selection: selected,
                        };
                    }
                    MainMenuResult::Selected { selected } => {
                        match selected {
                            MainMenuSelection::NewGame => new_runstate = RunState::PreRun,
                            MainMenuSelection::LoadGame => {
                                saveload_system::load_game(&mut self.ecs);
                                new_runstate = RunState::AwaitingInput;
                                saveload_system::delete_save();
                            },
                            MainMenuSelection::Quit => std::process::exit(0),
                        };
                    }
                }
            }
            RunState::SaveGame => {
                saveload_system::save_game(&mut self.ecs);

                new_runstate = RunState::MainMenu {
                    menu_selection: MainMenuSelection::LoadGame,
                };
            }
            RunState::NextLevel => {
                self.goto_next_level();
                new_runstate = RunState::PreRun;
            }
            RunState::GameOver => {
                let result = gui::game_over(ctx);
                match result {
                    gui::GameOverResult::NoSelection => {}
                    gui::GameOverResult::QuitToMenu => {
                        self.game_over_cleanup();
                        new_runstate = RunState::MainMenu {
                            menu_selection: MainMenuSelection::NewGame,
                        };
                    }
                }
            }
            #[allow(unreachable_patterns)] _ => {}
        }

        // Render GUI
        match new_runstate {
            RunState::MainMenu { .. } => {}
            RunState::GameOver { .. } => {}
            #[cfg(feature = "mapgen_visualiser")] RunState::MapGeneration => {}
            _ => gui::draw_ui(&self.ecs, ctx)
        }

        // Update state
        {
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = new_runstate;
        }

        // Delete dead entities
        DamageSystem::delete_the_dead(&mut self.ecs);
    }
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;

    let mut context = RltkBuilder::simple80x50()
        .with_tile_dimensions(8 * 2, 8 * 2)
        .with_title("Rogue")
        .build()?;

    context.with_post_scanlines(true);
    context.screen_burn_color = RGB::named(rltk::ROYALBLUE2);

    let mut gs = State {
        ecs: World::new(),

        #[cfg(feature = "mapgen_visualiser")] mapgen_running: true,
        #[cfg(feature = "mapgen_visualiser")] mapgen_index: 0,
        #[cfg(feature = "mapgen_visualiser")] mapgen_history: Vec::new(),
        #[cfg(feature = "mapgen_visualiser")] mapgen_timer: 0.,
    };

    // Register Components
    // -------------------------------------------------------------------------

    // Markers
    gs.ecs.register::<SimpleMarker<SerializeMe>>();

    // Tags
    gs.ecs.register::<Player>();
    gs.ecs.register::<Monster>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<Consumable>();
    gs.ecs.register::<Hidden>();
    gs.ecs.register::<EntityTrigger>();
    gs.ecs.register::<EntityMoved>();
    gs.ecs.register::<SingleActivation>();

    // Components
    // - Generic
    gs.ecs.register::<Name>();
    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<ParticleLifetime>();
    // - Combat
    gs.ecs.register::<CombatStats>();
    gs.ecs.register::<InflictsDamage>();
    gs.ecs.register::<SufferDamage>();
    gs.ecs.register::<Ranged>();
    gs.ecs.register::<AreaOfEffect>();
    gs.ecs.register::<MeleePowerBonus>();
    gs.ecs.register::<DefenseBonus>();
    // - Hunger
    gs.ecs.register::<HungerClock>();
    gs.ecs.register::<ProvidesFood>();

    // Intents
    gs.ecs.register::<WantsToMelee>();
    gs.ecs.register::<WantsToPickupItem>();
    gs.ecs.register::<WantsToDropItem>();
    gs.ecs.register::<WantsToUseItem>();
    gs.ecs.register::<WantsToRemoveItem>();

    // Items
    gs.ecs.register::<Equippable>();
    gs.ecs.register::<Equipped>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<ProvidesHealing>();
    gs.ecs.register::<Confusion>();
    gs.ecs.register::<MagicMapper>();

    // Special
    gs.ecs.register::<SerializationHelper>();

    // Register Resources
    // -------------------------------------------------------------------------

    #[cfg(feature = "mapgen_visualiser")]
    gs.ecs.insert(RunState::MapGeneration);
    #[cfg(not(feature = "mapgen_visualiser"))]
    gs.ecs.insert(RunState::MainMenu {
        menu_selection: MainMenuSelection::NewGame,
    });

    // Resource to get next marker identity
    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());

    gs.ecs.insert(RandomNumberGenerator::new());
    gs.ecs.insert(particle_system::ParticleBuilder::new());
    gs.ecs.insert(GameLog {
        entries: vec!["You awake in a dense, gloomy forest...".to_string()],
    });
    gs.ecs.insert(Map::new(MAP_WIDTH as i32, MAP_HEIGHT as i32, 1, None));

    // Player
    let player_entity = spawner::player(&mut gs.ecs, 0, 0);
    // Add player entity as resource
    gs.ecs.insert(player_entity);
    gs.ecs.insert(Point::new(0, 0)); // Player Pos

    gs.generate_world_map(1);

    return rltk::main_loop(context, gs);
}
