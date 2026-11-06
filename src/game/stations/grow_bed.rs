use avian2d::parry::simba::simd::SimdBool;
use bevy::prelude::*;

use crate::{game::{data::{Ingredient, IngredientKind}, interaction::Interacted, items::ItemPlaced}, game_manager::{GamePhase, NightTickEvent}};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (
        GrowBed::on_plant_tick.run_if(in_state(GamePhase::Night)),
    ));
    app.add_observer(observe_grow_bed_interact);
    app.add_observer(observe_grow_bed_interact_seed);
}

#[derive(Component, Debug, Clone)]
pub struct Seed{
    pub kind: IngredientKind,
    pub name: &'static str,
    pub grow_time: f32,
}

#[derive(Component, Debug, Clone)]
pub struct GrowBed {
    pub station_id: u32,
    pub is_planted: bool,
    pub is_harvestable: bool,
    pub growth_stage: u8,
    pub is_broken: bool,
    pub is_locked: bool,
    pub active_seed: Option<Seed>,
    pub remaining_grow_time: f32,
}

impl GrowBed {
    pub fn new(station_id: u32) -> Self {
        println!("GrowBed (station_id: {}) initialized.", station_id);
        Self {
            station_id,
            is_planted: false,
            is_harvestable: false,
            growth_stage: 0,
            is_broken: false,
            is_locked: false,
            active_seed: None,
            remaining_grow_time: 0.0
        }
    }

    pub fn set_seed(&mut self, seed: Option<Seed>){
        self.active_seed = seed;
        self.start_growth();
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
                self.growth_stage = ((1.0 - ratio) / 0.2).floor() as u8 + 1;
            }
            
            if self.remaining_grow_time <= 0.0 {
                self.complete_grow();
            }
        }
    }

    pub fn complete_grow(&mut self){
        self.growth_stage = 6;
        self.is_harvestable = true;
    }

    pub fn harvest_crop(&mut self){
        
    }
}

pub fn observe_grow_bed_interact(
    event: On<Interacted>,
    mut grow_bed_query: Query<&mut GrowBed>
){
    if let Ok(mut grow_bed) = grow_bed_query.get_mut(event.entity) {
        if(grow_bed.is_harvestable){

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