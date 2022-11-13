use rltk::{DistanceAlg, Point, RGB, Rltk, VirtualKeyCode};
use specs::prelude::*;
use crate::{CombatStats, Equipped, Hidden, HungerClock, HungerState, InBackpack, Name, Player, Position, RunState, State, Viewshed};
use crate::gamelog::GameLog;
use crate::map::Map;
use crate::saveload_system::does_save_exist;

// Enums
// =========================================================================

#[derive(PartialEq, Copy, Clone)]
pub enum ItemMenuResult {
	Cancel,
	NoResponse,
	Selected,
}

#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuSelection {
	NewGame,
	LoadGame,
	Quit,
}

#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuResult {
	NoSelection { selected: MainMenuSelection },
	Selected { selected: MainMenuSelection },
}

#[derive(PartialEq, Copy, Clone)]
pub enum GameOverResult {
	NoSelection,
	QuitToMenu,
}

// Main Menu
// =========================================================================

pub fn draw_main_menu (gs: &State, ctx: &mut Rltk) -> MainMenuResult {
	let save_exists = does_save_exist();
	let runstate = gs.ecs.fetch::<RunState>();

	ctx.print_color_centered(
		15,
		RGB::named(rltk::GOLD),
		RGB::named(rltk::BLACK),
		"Rogue",
	);

	if let RunState::MainMenu { menu_selection: selection } = *runstate {
		ctx.print_color_centered(
			24,
			if selection == MainMenuSelection::NewGame
				{ RGB::named(rltk::CYAN) } else
				{ RGB::named(rltk::WHITE) },
			RGB::named(rltk::BLACK),
			"New Game",
		);
		ctx.print_color_centered(
			26,
			if selection == MainMenuSelection::LoadGame
				{ RGB::named(rltk::CYAN) } else
				{ RGB::named(if save_exists { rltk::WHITE } else { rltk::DARK_GRAY } ) },
			RGB::named(rltk::BLACK),
			"Continue",
		);
		ctx.print_color_centered(
			28,
			if selection == MainMenuSelection::Quit
				{ RGB::named(rltk::CYAN) } else
				{ RGB::named(rltk::WHITE) },
			RGB::named(rltk::BLACK),
			"Quit",
		);

		match ctx.key {
			None => return MainMenuResult::NoSelection { selected: selection },
			Some(key) => {
				match key {
					VirtualKeyCode::Escape => {
						return MainMenuResult::NoSelection {
							selected: MainMenuSelection::Quit,
						};
					}
					VirtualKeyCode::Up | VirtualKeyCode::W => {
						let mut new_selection;
						match selection {
							MainMenuSelection::NewGame => new_selection = MainMenuSelection::Quit,
							MainMenuSelection::LoadGame => new_selection = MainMenuSelection::NewGame,
							MainMenuSelection::Quit => new_selection = MainMenuSelection::LoadGame,
						}
						if new_selection == MainMenuSelection::LoadGame && !save_exists {
							new_selection = MainMenuSelection::NewGame;
						}
						return MainMenuResult::NoSelection { selected: new_selection };
					}
					VirtualKeyCode::Down | VirtualKeyCode::S => {
						let mut new_selection;
						match selection {
							MainMenuSelection::NewGame => new_selection = MainMenuSelection::LoadGame,
							MainMenuSelection::LoadGame => new_selection = MainMenuSelection::Quit,
							MainMenuSelection::Quit => new_selection = MainMenuSelection::NewGame,
						}
						if new_selection == MainMenuSelection::LoadGame && !save_exists {
							new_selection = MainMenuSelection::Quit;
						}
						return MainMenuResult::NoSelection { selected: new_selection };
					}
					VirtualKeyCode::Return => {
						return MainMenuResult::Selected { selected: selection };
					}
					_ => return MainMenuResult::NoSelection { selected: selection },
				}
			}
		}
	}

	return MainMenuResult::NoSelection { selected: MainMenuSelection::NewGame };
}

// Game Interface
// =========================================================================

pub fn draw_ui (ecs: &World, ctx: &mut Rltk) {
	// Border
	ctx.draw_box(
		0, 43,
		79, 6,
		RGB::named(rltk::WHITE),
		RGB::named(rltk::BLACK),
	);

	// Depth
	let map = ecs.fetch::<Map>();
	let depth = format!(" Depth: {} ", map.depth);
	ctx.print_color(
		4, 43,
		RGB::named(rltk::YELLOW),
		RGB::named(rltk::BLACK),
		&depth,
	);

	// Player Health
	let combat_stats = ecs.read_storage::<CombatStats>();
	let players = ecs.read_storage::<Player>();
	let hunger = ecs.read_storage::<HungerClock>();
	for (_player, stats, hc) in (&players, &combat_stats, &hunger).join() {
		let health = format!(" HP: {} / {} ", stats.hp, stats.max_hp);
		ctx.print_color(
			17, 43,
			RGB::named(rltk::YELLOW),
			RGB::named(rltk::BLACK),
			&health,
		);

		ctx.draw_bar_horizontal(
			34, 43, 28,
			stats.hp, stats.max_hp,
			RGB::named(rltk::RED),
			RGB::named(rltk::DARK_GRAY),
		);

		let mut fg = RGB::new();
		let mut msg = "";

		match hc.state {
			HungerState::WellFed => {
				fg = RGB::named(rltk::LAWN_GREEN);
				msg = " Well Fed ";
			}
			HungerState::Normal => {}
			HungerState::Hungry => {
				fg = RGB::named(rltk::ORANGE);
				msg = " Hungry ";
			}
			HungerState::Starving => {
				fg = RGB::named(rltk::RED3);
				msg = " Starving ";
			}
		}

		if msg != "" {
			ctx.print_color(
				66, 43,
				fg, RGB::named(rltk::BLACK),
				msg
			);
		}
	}

	// Log
	let log = ecs.fetch::<GameLog>();
	let mut y = 44;
	for s in log.entries.iter().rev() {
		if y < 49 { ctx.print(2, y, s) }
		y += 1;
	}

	// Tooltips
	draw_tooltips(ecs, ctx);
}

fn draw_tooltips (ecs: &World, ctx: &mut Rltk) {
	let map = ecs.fetch::<Map>();
	let names = ecs.read_storage::<Name>();
	let positions = ecs.read_storage::<Position>();
	let hidden = ecs.read_storage::<Hidden>();

	let mouse_pos = ctx.mouse_pos();

	if mouse_pos.0 >= map.width || mouse_pos.1 >= map.width { return; }

	let mut tooltip : Vec<String> = Vec::new();
	for (name, position, _hidden) in (&names, &positions, !&hidden).join() {
		let idx = map.xy_idx(position.x, position.y);

		if position.x == mouse_pos.0
		&& position.y == mouse_pos.1
		&& map.visible_tiles[idx] {
			tooltip.push(name.name.to_string());
		}
	}

	if tooltip.is_empty() { return; }

	let mut width : i32 = 0;
	for s in tooltip.iter() {
		if width < s.len() as i32 {
			width = s.len() as i32;
		}
	}
	width += 3;

	if mouse_pos.0 > 40 {
		let arrow_pos = Point::new(mouse_pos.0 - 2, mouse_pos.1);
		let left_x = mouse_pos.0 - width;
		let mut y = mouse_pos.1;

		for s in tooltip.iter() {
			ctx.print_color(
				left_x, y,
				RGB::named(rltk::BLACK),
				RGB::named(rltk::GREY),
				s,
			);

			let padding = (width - s.len() as i32) - 1;
			for i in 0..padding {
				ctx.print_color(
					arrow_pos.x - i,
					y,
					RGB::named(rltk::BLACK),
					RGB::named(rltk::GREY),
					&" ".to_string(),
				);
			}
			y += 1;
		}
		ctx.print_color(
			arrow_pos.x,
			arrow_pos.y,
			RGB::named(rltk::BLACK),
			RGB::named(rltk::GREY),
			&"->".to_string(),
		);
	} else {
		let arrow_pos = Point::new(mouse_pos.0 + 1, mouse_pos.1);
		let left_x = mouse_pos.0 + 3;
		let mut y = mouse_pos.1;

		for s in tooltip.iter() {
			ctx.print_color(
				left_x + 1, y,
				RGB::named(rltk::BLACK),
				RGB::named(rltk::GREY),
				s,
			);

			let padding = (width - s.len() as i32) - 1;
			for i in 0..padding {
				ctx.print_color(
					arrow_pos.x + 1 + i,
					y,
					RGB::named(rltk::BLACK),
					RGB::named(rltk::GREY),
					&" ".to_string(),
				);
			}
			y += 1;
		}
		ctx.print_color(
			arrow_pos.x,
			arrow_pos.y,
			RGB::named(rltk::BLACK),
			RGB::named(rltk::GREY),
			&"<-".to_string(),
		);
	}
}

// Inventory
// =========================================================================

pub fn show_inventory (gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
	let player_entity = gs.ecs.fetch::<Entity>();
	let names = gs.ecs.read_storage::<Name>();
	let backpack = gs.ecs.read_storage::<InBackpack>();
	let entities = gs.ecs.entities();

	let inventory = (&backpack, &names).join()
		.filter(|item| item.0.owner == *player_entity);
	let count = inventory.count();

	let mut y = (25 - (count / 2)) as i32;
	ctx.draw_box(
		15, y - 2, 32, (count + 3) as i32,
		RGB::named(rltk::WHITE),
		RGB::named(rltk::BLACK),
	);
	ctx.print_color(
		18, y - 2,
		RGB::named(rltk::GOLD),
		RGB::named(rltk::BLACK),
		" Inventory "
	);
	ctx.print_color(
		18, y + count as i32 + 1,
		RGB::named(rltk::GREY),
		RGB::named(rltk::BLACK),
		" ESCAPE to cancel "
	);

	let mut equippable : Vec<Entity> = Vec::new();
	let mut j = 0;
	let inventory_items = (&entities, &backpack, &names).join()
		.filter(|item| item.1.owner == *player_entity);

	for (entity, _, name) in inventory_items {
		ctx.set(
			17, y,
			RGB::named(rltk::WHITE),
			RGB::named(rltk::BLACK),
			rltk::to_cp437('('),
		);
		ctx.set(
			18, y,
			RGB::named(rltk::WHITE),
			RGB::named(rltk::BLACK),
			97 + j as rltk::FontCharType,
		);
		ctx.set(
			19, y,
			RGB::named(rltk::WHITE),
			RGB::named(rltk::BLACK),
			rltk::to_cp437(')'),
		);

		ctx.print(21, y, &name.name.to_string());
		equippable.push(entity);
		y += 1;
		j += 1;
	}

	match ctx.key {
		None => (ItemMenuResult::NoResponse, None),
		Some(key) => {
			match key {
				VirtualKeyCode::Escape => (ItemMenuResult::Cancel, None),
				_ => {
					let selection = rltk::letter_to_option(key);
					if selection > -1 && selection < count as i32 {
						return (
							ItemMenuResult::Selected,
							Some(equippable[selection as usize])
						);
					}
					return (ItemMenuResult::NoResponse, None);
				},
			}
		}
	}
}

// Drop Item Menu
// =========================================================================

pub fn drop_item_menu (gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
	let player_entity = gs.ecs.fetch::<Entity>();
	let names = gs.ecs.read_storage::<Name>();
	let backpack = gs.ecs.read_storage::<InBackpack>();
	let entities = gs.ecs.entities();

	let inventory = (&backpack, &names).join()
		.filter(|item| item.0.owner == *player_entity);
	let count = inventory.count();

	let mut y = (25 - (count / 2)) as i32;
	ctx.draw_box(
		15, y - 2, 32, (count + 3) as i32,
		RGB::named(rltk::WHITE),
		RGB::named(rltk::BLACK),
	);
	ctx.print_color(
		18, y - 2,
		RGB::named(rltk::GOLD),
		RGB::named(rltk::BLACK),
		" Drop Which Item? "
	);
	ctx.print_color(
		18, y + count as i32 + 1,
		RGB::named(rltk::GREY),
		RGB::named(rltk::BLACK),
		" ESCAPE to cancel "
	);

	let mut equippable : Vec<Entity> = Vec::new();
	let mut j = 0;
	let inventory_items = (&entities, &backpack, &names).join()
		.filter(|item| item.1.owner == *player_entity);

	for (entity, _, name) in inventory_items {
		ctx.set(
			17, y,
			RGB::named(rltk::WHITE),
			RGB::named(rltk::BLACK),
			rltk::to_cp437('('),
		);
		ctx.set(
			18, y,
			RGB::named(rltk::WHITE),
			RGB::named(rltk::BLACK),
			97 + j as rltk::FontCharType,
		);
		ctx.set(
			19, y,
			RGB::named(rltk::WHITE),
			RGB::named(rltk::BLACK),
			rltk::to_cp437(')'),
		);

		ctx.print(21, y, &name.name.to_string());
		equippable.push(entity);
		y += 1;
		j += 1;
	}

	match ctx.key {
		None => (ItemMenuResult::NoResponse, None),
		Some(key) => {
			match key {
				VirtualKeyCode::Escape => (ItemMenuResult::Cancel, None),
				_ => {
					let selection = rltk::letter_to_option(key);
					if selection > -1 && selection < count as i32 {
						return (
							ItemMenuResult::Selected,
							Some(equippable[selection as usize])
						);
					}
					return (ItemMenuResult::NoResponse, None);
				},
			}
		}
	}
}

// Remove Item
// =========================================================================

pub fn remove_item_menu (gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
	let player_entity = gs.ecs.fetch::<Entity>();
	let names = gs.ecs.read_storage::<Name>();
	let equipped = gs.ecs.read_storage::<Equipped>();
	let entities = gs.ecs.entities();

	let inventory = (&equipped, &names).join()
		.filter(|item| item.0.owner == *player_entity);
	let count = inventory.count();

	let mut y = (25 - (count / 2)) as i32;
	ctx.draw_box(
		15, y - 2, 31, count as i32 + 3,
		RGB::named(rltk::WHITE),
		RGB::named(rltk::BLACK),
	);
	ctx.print_color(
		18, y - 2,
		RGB::named(rltk::GOLD),
		RGB::named(rltk::BLACK),
		" Remove which item? "
	);
	ctx.print_color(
		18, y + count as i32 + 1,
		RGB::named(rltk::GREY),
		RGB::named(rltk::BLACK),
		" ESCAPE to cancel "
	);

	let mut equippable : Vec<Entity> = Vec::new();
	let mut j = 0;
	let inventory_items = (&entities, &equipped, &names).join()
		.filter(|item| item.1.owner == *player_entity);

	for (entity, _, name) in inventory_items {
		ctx.set(
			17, y,
			RGB::named(rltk::WHITE),
			RGB::named(rltk::BLACK),
			rltk::to_cp437('('),
		);
		ctx.set(
			18, y,
			RGB::named(rltk::WHITE),
			RGB::named(rltk::BLACK),
			97 + j as rltk::FontCharType,
		);
		ctx.set(
			19, y,
			RGB::named(rltk::WHITE),
			RGB::named(rltk::BLACK),
			rltk::to_cp437(')'),
		);

		ctx.print(21, y, &name.name.to_string());
		equippable.push(entity);
		y += 1;
		j += 1;
	}

	match ctx.key {
		None => (ItemMenuResult::NoResponse, None),
		Some(key) => {
			match key {
				VirtualKeyCode::Escape => (ItemMenuResult::Cancel, None),
				_ => {
					let selection = rltk::letter_to_option(key);
					if selection > -1 && selection < count as i32 {
						return (ItemMenuResult::Selected, Some(equippable[selection as usize]));
					}

					return (ItemMenuResult::NoResponse, None);
				}
			}
		}
	}
}

// Ranged Targeting
// =========================================================================

pub fn ranged_target (gs: &mut State, ctx: &mut Rltk, range: i32)
	-> (ItemMenuResult, Option<Point>)
{
	let player_entity = gs.ecs.fetch::<Entity>();
	let player_pos = gs.ecs.fetch::<Point>();
	let viewsheds = gs.ecs.read_storage::<Viewshed>();

	ctx.print_color(
		5, 0,
		RGB::named(rltk::YELLOW),
		RGB::named(rltk::BLACK),
		" Select Target: ",
	);

	// Highlight available target cells
	let mut available_cells = Vec::new();
	let visible = viewsheds.get(*player_entity);
	if let Some(visible) = visible {
		for idx in visible.visible_tiles.iter() {
			let distance = DistanceAlg::Pythagoras.distance2d(*player_pos, *idx);
			if distance <= range as f32 {
				ctx.set_bg(
					idx.x, idx.y,
					RGB::named(rltk::BLUE),
				);
				available_cells.push(idx);
			}
		}
	} else {
		return (ItemMenuResult::Cancel, None);
	}

	// Draw mouse cursor
	let mouse_pos = ctx.mouse_pos();
	let mut valid_target = false;

	for idx in available_cells.iter() {
		if idx.x == mouse_pos.0 && idx.y == mouse_pos.1 {
			valid_target = true;
		}
	}

	if valid_target {
		ctx.set_bg(
			mouse_pos.0, mouse_pos.1,
			RGB::named(rltk::CYAN),
		);

		if ctx.left_click {
			return (
				ItemMenuResult::Selected,
				Some(Point::new(mouse_pos.0, mouse_pos.1))
			);
		}
	} else {
		ctx.set_bg(
			mouse_pos.0, mouse_pos.1,
			RGB::named(rltk::RED),
		);

		if ctx.left_click {
			return (ItemMenuResult::Cancel, None);
		}
	}

	return (ItemMenuResult::NoResponse, None);
}

// Game Over
// =========================================================================

pub fn game_over (ctx: &mut Rltk) -> GameOverResult {
	ctx.print_color_centered(
		15,
		RGB::named(rltk::GOLD),
		RGB::named(rltk::BLACK),
		"You die"
	);
	ctx.print_color_centered(
		17,
		RGB::named(rltk::WHITE),
		RGB::named(rltk::BLACK),
		"Lost and alone"
	);
	ctx.print_color_centered(
		19,
		RGB::named(rltk::WHITE),
		RGB::named(rltk::BLACK),
		"Forgotten"
	);

	ctx.print_color_centered(
		24,
		RGB::named(rltk::GREY),
		RGB::named(rltk::BLACK),
		"Press space"
	);

	match ctx.key {
		None => GameOverResult::NoSelection,
		Some(key) => {
			if key == VirtualKeyCode::Space {
				GameOverResult::QuitToMenu
			} else {
				GameOverResult::NoSelection
			}
		},
	}
}