//! This is the module that collects our game logic.
//!
//! Logic is constructed individual in separate modules
//! and then organized as plugins. The plugins are then shared
//! here for inclusion in the main App.

use bevy::prelude::*;

mod animation;
pub mod data;
mod hud;
mod interaction;
pub mod items;
pub mod level;
mod movement;
pub mod pause;
pub mod physics;
pub mod player;
pub mod progression;
pub mod score;
pub mod shop;
mod stations;
mod user_interface;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        progression::plugin,
        score::plugin,
        level::plugin,
        shop::plugin,
        player::plugin,
        movement::plugin,
        interaction::plugin,
        physics::plugin,
        hud::plugin,
        animation::plugin,
        items::plugin,
        pause::plugin,
        user_interface::plugin,
    ));
}
