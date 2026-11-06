use bevy::prelude::*;

use crate::game::data::{AlchemyDatabase, Ingredient, IngredientKind, Potion};
<<<<<<< HEAD
use crate::game::interaction::Interacted;
use crate::game::items::ItemPlaced;
=======
use crate::game::interaction::{Interacted};
use crate::game::items::{ItemPlaced};
>>>>>>> ddd83dee4f0bf99ed792ac542c7a3260aae5ab23
use crate::game_manager::{GamePhase, NightTickEvent};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            Cauldron::on_brew_tick.run_if(in_state(GamePhase::Night)),
            sync_cauldron_visuals,
            animate_cauldron_sprites,
        ),
    );
<<<<<<< HEAD
    app.add_observer(observe_cauldron_interact);
    app.add_observer(observe_cauldron_interact_ingredient);
=======
=======
    app.add_systems(Update, (
        Cauldron::on_brew_tick.run_if(in_state(GamePhase::Night)),
        sync_cauldron_visuals,
        animate_cauldron_sprites,
    ));
    app.add_observer(observe_cauldron_interact);
    app.add_observer(observe_cauldron_interact_ingredient);
>>>>>>> fe4f6aacd26d3a92c61e97f1eb388f615984abc8
>>>>>>> ddd83dee4f0bf99ed792ac542c7a3260aae5ab23
}

#[derive(Component, Debug, Clone)]
pub struct Cauldron {
    pub station_id: u32,
    pub ingredients: Vec<IngredientKind>,
    pub is_brewing: bool,
    pub remaining_brew_time: f32,
    pub brewed_potion: Option<Potion>,
    pub is_broken: bool,
    pub is_locked: bool,
}

impl Cauldron {
    pub fn new(station_id: u32) -> Self {
        println!("Cauldron (station_id: {}) initialized.", station_id);
        Self {
            station_id: station_id,
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

<<<<<<< HEAD
=======
<<<<<<< HEAD
    pub fn remove_ingredient_by_type(&mut self, ingredient: IngredientKind) {
        if let Some(pos) = self.ingredients.iter().position(|&x| x == ingredient) {
            self.ingredients.remove(pos);
        }
    }

    pub fn remove_ingredient_by_index(&mut self, index: usize) {
        if index < self.ingredients.len() {
            self.ingredients.remove(index);
        }
    }

    pub fn clear_ingredients(&mut self) {
        if !self.is_brewing {
            self.ingredients.clear();
            self.reset_brewing();
        }
    }

=======
>>>>>>> fe4f6aacd26d3a92c61e97f1eb388f615984abc8
>>>>>>> ddd83dee4f0bf99ed792ac542c7a3260aae5ab23
    pub fn start_brewing(&mut self) {
        if !self.is_brewing && !self.is_broken && !self.ingredients.is_empty() {
            self.is_brewing = true;
            self.remaining_brew_time = 30.0;
        }
    }

    pub fn complete_brew(&mut self, alchemy_database: &AlchemyDatabase) {
        println!(
            "Cauldron (station_id: {}) completed brewing with ingredients: {:?}",
            self.station_id, self.ingredients
        );
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
<<<<<<< HEAD
=======
<<<<<<< HEAD

    pub fn cauldron_interact(&mut self) {}
=======
>>>>>>> fe4f6aacd26d3a92c61e97f1eb388f615984abc8
>>>>>>> ddd83dee4f0bf99ed792ac542c7a3260aae5ab23
}

#[derive(Component, Debug, Default)]
pub struct CauldronAnimation {
    pub timer: Timer,
    pub first_frame: usize,
    pub last_frame: usize,
}

impl CauldronAnimation {
    pub fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            first_frame: 0,
            last_frame: 0,
        }
    }
}

fn sync_cauldron_visuals(
    mut query: Query<(&Cauldron, &mut CauldronAnimation, &mut Sprite), Changed<Cauldron>>,
) {
    for (cauldron, mut anim, mut sprite) in query.iter_mut() {
        let (first, last, speed) = if cauldron.is_locked {
            (0, 0, 0.1) // Locked frame
        } else if cauldron.is_broken {
            (1, 1, 0.1) // Broken frame
        } else if cauldron.is_brewing {
            (4, 7, 0.15) // Brewing animation frame indices 4 through 7
        } else if !cauldron.ingredients.is_empty() {
            (2, 3, 0.2) // Filling / filled animation loop
        } else {
            (0, 0, 0.1) // Default Idle
        };

        // If the animation state changed, reset the indices and timer
        if anim.first_frame != first || anim.last_frame != last {
            anim.first_frame = first;
            anim.last_frame = last;
            anim.timer = Timer::from_seconds(speed, TimerMode::Repeating);
            if let Some(ref mut atlas) = sprite.texture_atlas {
                atlas.index = first;
            }
        }
    }
}

fn animate_cauldron_sprites(
    time: Res<Time>,
    mut query: Query<(&mut CauldronAnimation, &mut Sprite)>,
) {
    for (mut anim, mut sprite) in query.iter_mut() {
        // Skip updating static icons/frames
        if anim.first_frame == anim.last_frame {
            continue;
        }

        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            if let Some(ref mut atlas) = sprite.texture_atlas {
                if atlas.index >= anim.last_frame {
                    atlas.index = anim.first_frame;
                } else {
                    atlas.index += 1;
                }
            }
        }
    }
}
<<<<<<< HEAD

pub fn observe_cauldron_interact(event: On<Interacted>, mut cauldron_query: Query<&mut Cauldron>) {
    if let Ok(mut cauldron) = cauldron_query.get_mut(event.entity) {
        if (cauldron.is_harvestable()) {
=======
<<<<<<< HEAD
=======

pub fn observe_cauldron_interact(
    event: On<Interacted>,
    mut cauldron_query: Query<&mut Cauldron>
){
    if let Ok(mut cauldron) = cauldron_query.get_mut(event.entity) {
        if(cauldron.is_harvestable()){
>>>>>>> ddd83dee4f0bf99ed792ac542c7a3260aae5ab23
            let brewed_potion = cauldron.harvest_brewed_potion();

            // TODO: Give brewed potion to player
        }
    }
}

pub fn observe_cauldron_interact_ingredient(
    event: On<ItemPlaced>,
    mut cauldron_query: Query<&mut Cauldron>,
    ingredient_query: Query<&Ingredient>,
<<<<<<< HEAD
) {
    println!("{}, {}, {}", event.by, event.item, event.entity);

    if let Ok(mut ingredient) = ingredient_query.get(event.item) {
        if let Ok(mut cauldron) = cauldron_query.get_mut(event.entity) {
            if (!cauldron.is_brewing) {
=======
){
    println!("{}, {}, {}", event.by, event.item, event.entity);
  
    if let Ok(mut ingredient) = ingredient_query.get(event.item){
        if let Ok(mut cauldron) = cauldron_query.get_mut(event.entity) {
            if(!cauldron.is_brewing){
>>>>>>> ddd83dee4f0bf99ed792ac542c7a3260aae5ab23
                cauldron.add_ingredient(ingredient.kind);
                cauldron.start_brewing();
            }
        }
    }
}
<<<<<<< HEAD
=======
>>>>>>> fe4f6aacd26d3a92c61e97f1eb388f615984abc8
>>>>>>> ddd83dee4f0bf99ed792ac542c7a3260aae5ab23
