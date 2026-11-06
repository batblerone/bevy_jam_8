use bevy::prelude::*;

pub mod cauldron;
pub mod grow_bed;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(cauldron::plugin);
    app.add_plugins(grow_bed::plugin);
}
