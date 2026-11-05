//! The game's menus and transitions between them.

mod credits;
mod main;
pub mod navigation;
mod pause;
mod settings;

use crate::game_manager::*;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        navigation::plugin,
        credits::plugin,
        main::plugin,
        settings::plugin,
        pause::plugin,
    ));
}
