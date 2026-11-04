//! This is the module that collects our game logic.
//!
//! Logic is constructed individual in separate modules
//! and then organized as plugins. The plugins are then shared
//! here for inclusion in the main App.

use bevy::prelude::*;

pub mod data;

mod animation;
mod hud;
pub mod level;
mod movement;
pub mod physics;
pub mod player;
pub mod progression;
pub mod score;
mod shop;

mod user_interface;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        progression::plugin,
        score::plugin,
        level::plugin,
        shop::plugin,
        player::plugin,
        movement::plugin,
        physics::plugin,
        hud::plugin,
        animation::plugin,
        user_interface::plugin,
    ));
}
