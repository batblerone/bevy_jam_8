use bevy::prelude::*;

use crate::{game::{data::{IngredientAssets, IngredientKind, ingredients}, interaction::Interacted, items::ItemPlaced}, game_manager::{GamePhase, NightTickEvent}};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (
        GrowBed::on_plant_tick.run_if(in_state(GamePhase::Night)),
        update_bouncing_items,
    ));
    app.add_observer(observe_grow_bed_interact);
    app.add_observer(observe_grow_bed_interact_seed);
}

#[derive(Component, Debug, Clone)]
pub struct Seed{
    pub kind: IngredientKind,
    pub grow_time: f32,
}

#[derive(Component, Debug, Clone)]
pub struct GrowBed {
    pub is_planted: bool,
    pub is_harvestable: bool,
    pub growth_stage: u8,
    pub is_broken: bool,
    pub is_locked: bool,
    pub active_seed: Option<Seed>,
    pub remaining_grow_time: f32,
}

impl GrowBed {
    pub fn new() -> Self {
        Self {
            is_planted: true,
            is_harvestable: false,
            growth_stage: 0,
            is_broken: false,
            is_locked: false,
            active_seed: Some(Seed {
                kind: IngredientKind::Fireweed,
                grow_time: 10.0
            }),
            remaining_grow_time: 10.0
        }
    }

    pub fn set_seed(&mut self, new_seed: Option<Seed>){
        self.active_seed = new_seed;
        
        if let Some(_) = self.active_seed.as_ref() {
            self.start_growth();
        }
    }

    pub fn start_growth(&mut self){
        if let Some(seed) = self.active_seed.as_ref() {
            self.is_planted = true;
            self.remaining_grow_time = seed.grow_time;
        }
    }

    pub fn on_plant_tick(
        mut grow_beds: Query<&mut Self>,
        mut messages: MessageReader<NightTickEvent>,
    ){
        for event in messages.read() {
            for mut grow_bed in grow_beds.iter_mut() {
                // Call the self-contained method directly on each instance
                grow_bed.tick_plant(event.delta_time.as_secs_f32());
            }
        }
    }

    pub fn tick_plant(&mut self, delta_time: f32) {
        if self.is_planted && !self.is_harvestable && self.remaining_grow_time > 0.0 {
            self.remaining_grow_time -= delta_time;
            if let Some(seed) = self.active_seed.as_ref() {
                let ratio = (self.remaining_grow_time / seed.grow_time).clamp(0.0, 1.0);
                // Maps 1.0 -> 1, 0.0 -> 6, and every 20% interval in between
                self.growth_stage = ((1.0 - ratio) / 0.25).floor() as u8 + 1;
            }
            
            if self.remaining_grow_time <= 0.0 {
                self.complete_grow();
            }
        }
    }

    pub fn complete_grow(&mut self){
        self.growth_stage = 6;
        self.is_harvestable = true;
        println!("Harvestable!!!!!");
    }

    /// If the grow bed is harvestable, launches the ingredient lightly away from
    /// the harvesting entity and resets the grow timer to start a new growth cycle.
    pub fn harvest_crop(
        &mut self, 
        commands: &mut Commands,
        bed_transform: Option<&Transform>,
        player_transform: Option<&Transform>,
        assets: Option<Res<IngredientAssets>>,
    ){
        println!("Harvest triggered!");
        if self.is_planted && self.is_harvestable && self.remaining_grow_time <= 0.0 {
            // Calculate the drop direction (defaults to upward/forward if transforms missing)
            let mut drop_direction = Vec3::Y; // Default upward bounce
            let spawn_position = bed_transform.map(|t| t.translation).unwrap_or(Vec3::ZERO);
            
            if let (Some(bed), Some(player)) = (bed_transform, player_transform) {
                // Vector pointing from player to grow bed
                let mut flat_direction = bed.translation - player.translation;
                flat_direction.y = 0.0; // Keep it on the ground plane
                
                if flat_direction != Vec3::ZERO {
                    // Normalize and add a small upward arc lift
                    drop_direction = flat_direction.normalize() + Vec3::Y * 0.5;
                }
            }
            let initial_velocity = drop_direction.normalize() * 5.0;

            // Extract the ingredient kind before resetting/clearing
            if let Some(seed) = self.active_seed.as_ref() && let Some(assets) = assets.as_ref() {
                let ingredient_kind = seed.kind; 

                // --- SPAWN INGREDIENT ENTITY HERE ---
                let mut ingredient = commands.spawn_scene(ingredients(ingredient_kind, assets));
                ingredient.insert((
                    Transform::from_translation(spawn_position),
                    ingredient_kind,
                    BouncingIngredient {
                        velocity: initial_velocity,
                        gravity: 9.8,       // Pulls the item down over time
                        friction: 4.0,      // Dwindles horizontal movement quickly
                    },
                ));
            }

            // Reset the grow bed state
            self.is_harvestable = false;
            if let Some(seed) = self.active_seed.as_ref() {
                self.remaining_grow_time = seed.grow_time;
            }

            
            println!("Harvest Complete!");
        }
    }
}

pub fn observe_grow_bed_interact(
    event: On<Interacted>,
    mut commands: Commands,
    mut grow_bed_query: Query<&mut GrowBed>,
    transform_query: Query<&Transform>,
    assets: Option<Res<IngredientAssets>>,
){
    if let Ok(mut grow_bed) = grow_bed_query.get_mut(event.entity) {
        if grow_bed.is_harvestable {
            let bed_transform = transform_query.get(event.entity).ok();
            let player_transform = transform_query.get(event.by).ok();

            grow_bed.harvest_crop(&mut commands, bed_transform, player_transform, assets);
        }
    }
}

pub fn observe_grow_bed_interact_seed(
    event: On<ItemPlaced>,
    mut grow_bed_query: Query<&mut GrowBed>,
    seed_query: Query<&Seed>,
    game_phase: ResMut<State<GamePhase>>,
){  
    if let Ok(seed) = seed_query.get(event.item){
        if let Ok(mut grow_bed) = grow_bed_query.get_mut(event.entity) {
            if !grow_bed.is_planted && *game_phase == GamePhase::Day {
                grow_bed.set_seed(Some(seed.clone()));
            }
        }
    }
}

#[derive(Component)]
pub struct BouncingIngredient {
    pub velocity: Vec3,
    pub gravity: f32,
    pub friction: f32,
}

pub fn update_bouncing_items(
    mut query: Query<(&mut Transform, &mut BouncingIngredient)>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();

    for (mut transform, mut bounce) in query.iter_mut() {
        // 1. Apply movement based on current velocity
        transform.translation += bounce.velocity * delta;

        // 2. Dwindle the horizontal velocity (X and Z) using friction
        bounce.velocity.x -= bounce.velocity.x * bounce.friction * delta;
        bounce.velocity.z -= bounce.velocity.z * bounce.friction * delta;

        // 3. Apply gravity to the vertical velocity (Y)
        bounce.velocity.y -= bounce.gravity * delta;

        // 4. Fake Ground Collision (Assumes bed position floor is Y = 0.0)
        // If your beds sit on a different height, change 0.0 to your local floor level
        if transform.translation.y <= 0.0 {
            transform.translation.y = 0.0;
            
            // If it still has vertical downward energy, give it a tiny micro-bounce
            if bounce.velocity.y < -1.0 {
                bounce.velocity.y = -bounce.velocity.y * 0.4; // Reverse and cut energy in half
            } else {
                // Otherwise, stop all movement entirely
                bounce.velocity = Vec3::ZERO;
            }
        }
    }
}