use bevy::prelude::*;

pub mod cauldron;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(cauldron::plugin);
}
