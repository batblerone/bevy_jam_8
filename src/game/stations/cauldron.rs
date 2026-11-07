use bevy::prelude::*;

use crate::game::data::{AlchemyDatabase, Ingredient, IngredientKind, Potion};
use crate::game::interaction::{Interacted};
use crate::game::items::{ItemPlaced};
use crate::game_manager::{GamePhase, NightTickEvent};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (
        Cauldron::on_brew_tick.run_if(in_state(GamePhase::Night)),
    ));
    app.add_observer(observe_cauldron_interact);
    app.add_observer(observe_cauldron_interact_ingredient);
}

#[derive(Component, Debug, Clone)]
pub struct Cauldron {
    pub ingredients: Vec<IngredientKind>,
    pub is_brewing: bool,
    pub remaining_brew_time: f32,
    pub brewed_potion: Option<Potion>,
    pub is_broken: bool,
    pub is_locked: bool,
}

impl Cauldron {
    pub fn new() -> Self {
        Self {
            ingredients: Vec::new(),
            is_brewing: false,
            remaining_brew_time: 0.0,
            brewed_potion: None,
            is_broken: false,
            is_locked: true,
        }
    }

    pub fn on_brew_tick(
        mut cauldrons: Query<&mut Self>,
        alchemy_database: Res<AlchemyDatabase>,
        mut messages: MessageReader<NightTickEvent>, // Note: typically EventReader in Bevy
    ) {
        for event in messages.read() {
            for mut cauldron in cauldrons.iter_mut() {
                // Call the self-contained method directly on each instance
                cauldron.tick_brewing(event.delta_time.as_secs_f32(), &alchemy_database);
            }
        }
    }

    pub fn tick_brewing(&mut self, delta_time: f32, alchemy_database: &AlchemyDatabase) {
        if self.is_brewing {
            self.remaining_brew_time -= delta_time;
            if self.remaining_brew_time <= 0.0 {
                self.complete_brew(alchemy_database);
            }
        }
    }

    pub fn add_ingredient(&mut self, ingredient: IngredientKind) {
        if !self.is_broken {
            self.ingredients.push(ingredient);
        }
    }

    pub fn start_brewing(&mut self) {
        if !self.is_brewing && !self.is_broken && !self.ingredients.is_empty() {
            self.is_brewing = true;
            self.remaining_brew_time = 30.0;
        }
    }

    pub fn complete_brew(&mut self, alchemy_database: &AlchemyDatabase) {
        self.brewed_potion = alchemy_database
            .find_potion_by_ingredients(&self.ingredients)
            .cloned();

        self.is_brewing = false;
        self.remaining_brew_time = 0.0;
        self.ingredients.clear();

        if self.brewed_potion.is_none() {
            self.is_broken = true;
        }
    }

    pub fn is_harvestable(&self) -> bool {
        !self.is_brewing && !self.is_broken && self.brewed_potion.is_some()
    }

    pub fn harvest_brewed_potion(&mut self) -> Option<Potion> {
        self.brewed_potion.take()
    }

    pub fn repair_cauldron(&mut self) {
        self.is_broken = false;
    }

    pub fn unlock_cauldron(&mut self) {
        self.is_locked = false;
    }
}

pub fn observe_cauldron_interact(
    event: On<Interacted>,
    mut cauldron_query: Query<&mut Cauldron>
){
    if let Ok(mut cauldron) = cauldron_query.get_mut(event.entity) {
        if(cauldron.is_harvestable()){
            let brewed_potion = cauldron.harvest_brewed_potion();

            // TODO: Give brewed potion to player
        }
    }
}

pub fn observe_cauldron_interact_ingredient(
    event: On<ItemPlaced>,
    mut cauldron_query: Query<&mut Cauldron>,
    ingredient_query: Query<&Ingredient>,
){  
    if let Ok(ingredient) = ingredient_query.get(event.item){
        if let Ok(mut cauldron) = cauldron_query.get_mut(event.entity) {
            if !cauldron.is_brewing {
                cauldron.add_ingredient(ingredient.kind);
                cauldron.start_brewing();
            }
        }
    }
}