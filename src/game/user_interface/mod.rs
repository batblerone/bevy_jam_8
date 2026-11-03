//! Module that collects our game UI logic and plugins.

use bevy::prelude::*;

mod timeline_ui;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        timeline_ui::TimelineUiPlugin,
    ));
}
