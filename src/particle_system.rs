use rltk::{RGB, Rltk};
use specs::prelude::*;
use crate::{ParticleLifetime, Position, Renderable};

// Builder
// =========================================================================

struct ParticleRequest {
	x: i32,
	y: i32,
	fg: RGB,
	bg: RGB,
	glyph: rltk::FontCharType,
	lifetime: f32,
}

pub struct ParticleBuilder {
	requests : Vec<ParticleRequest>,
}

impl ParticleBuilder {
	pub fn new() -> ParticleBuilder {
		ParticleBuilder { requests: Vec::new() }
	}

	pub fn request (
		&mut self,
		x: i32, y: i32,
		fg: RGB, bg: RGB,
		glyph: rltk::FontCharType,
		lifetime: f32,
	) {
		self.requests.push(ParticleRequest {
			x, y, fg, bg, glyph, lifetime,
		});
	}
}

// Systems
// =========================================================================

pub fn cull_dead_particles (ecs: &mut World, ctx: &Rltk) {
	let mut dead_particles : Vec<Entity> = Vec::new();
	{
		let mut particles = ecs.write_storage::<ParticleLifetime>();
		let entities = ecs.entities();
		for (entity, mut particle) in (&entities, &mut particles).join() {
			particle.lifetime_ms -= ctx.frame_time_ms;
			if particle.lifetime_ms < 0. {
				dead_particles.push(entity);
			}
		}
	}

	for dead in dead_particles.iter() {
		ecs.delete_entity(*dead).expect("Particles just won't die");
	}
}

pub struct ParticleSpawnSystem {}

impl<'a> System<'a> for ParticleSpawnSystem {
	type SystemData = (
		Entities<'a>,
		WriteStorage<'a, Position>,
		WriteStorage<'a, Renderable>,
		WriteStorage<'a, ParticleLifetime>,
		WriteExpect<'a, ParticleBuilder>,
	);

	fn run(&mut self, data: Self::SystemData) {
		let (
			entities, mut positions, mut renderables, mut particles,
			mut particle_builder,
		) = data;

		for new_particle in particle_builder.requests.iter() {
			let p = entities.create();
			positions.insert(p, Position {
				x: new_particle.x,
				y: new_particle.y,
			}).expect("Failed to position particles");
			renderables.insert(p, Renderable {
				fg: new_particle.fg,
				bg: new_particle.bg,
				glyph: new_particle.glyph,
				render_order: 0,
			}).expect("Failed to render particles");
			particles.insert(p, ParticleLifetime {
				lifetime_ms: new_particle.lifetime,
			}).expect("Failed to force particle to die of old age");
		}

		particle_builder.requests.clear();
	}
}