#![allow(unused)]
//! The credits menu.

use bevy::{ecs::spawn::SpawnIter, prelude::*};

use crate::{
    asset_tracking::LoadResource,
    audio::music,
    menus::Menu,
    menus::navigation::{MenuBack, action_just_started},
    theme::prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::Credits), spawn_credits_menu);
    app.add_systems(
        Update,
        go_back.run_if(in_state(Menu::Credits).and_then(action_just_started::<MenuBack>)),
    );

    // app.load_resource::<CreditsAssets>();
    // app.add_systems(OnEnter(Menu::Credits), start_credits_music);
}

fn spawn_credits_menu(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Credits Menu"),
        GlobalZIndex(2),
        DespawnOnExit(Menu::Credits),
        children![
            widget::header("Created by"),
            created_by(),
            widget::header("Assets"),
            assets(),
            widget::button("Back", go_back_on_click),
        ],
    ));
}

fn created_by() -> impl Bundle {
    grid(vec![
        ["Justin Clouthier and \n Justin MacKinnon", "Programming"],
        [
            "David Erskine and \n Duncan Osmond",
            "Art and Sprite Design",
        ],
        [
            "David Gibson, Oscar, and \n Heather LeBlanc",
            "Music and Sound Design",
        ],
    ])
}

fn assets() -> impl Bundle {
    grid(vec![
        ["Button SFX", "CC0 by Jaszunio15"],
        [
            "Fluffing a Duck Remix",
            "A remix of 'Fluffing a Duck' by Kevin MacLeod, CC BY 3.0'",
        ],
        [
            "Bevy logo",
            "All rights reserved by the Bevy Foundation, permission granted for splash screen use when unmodified",
        ],
    ])
}

fn grid(content: Vec<[&'static str; 2]>) -> impl Bundle {
    (
        Name::new("Grid"),
        Node {
            display: Display::Grid,
            row_gap: px(10),
            column_gap: px(30),
            grid_template_columns: RepeatedGridTrack::px(2, 400.0),
            ..default()
        },
        Children::spawn(SpawnIter(content.into_iter().flatten().enumerate().map(
            |(i, text)| {
                (
                    widget::label(text),
                    Node {
                        justify_self: if i.is_multiple_of(2) {
                            JustifySelf::End
                        } else {
                            JustifySelf::Start
                        },
                        ..default()
                    },
                )
            },
        ))),
    )
}

fn go_back_on_click(_: On<PointerClick>, mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Main);
}

fn go_back(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Main);
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct CreditsAssets {
    #[dependency]
    music: Handle<AudioSource>,
}

// impl FromWorld for CreditsAssets {
//     fn from_world(world: &mut World) -> Self {
//         return;
//     }
// }

// fn start_credits_music(mut commands: Commands, credits_music: Res<CreditsAssets>) {
//     commands.spawn((
//         Name::new("Credits Music"),
//         DespawnOnExit(Menu::Credits),
//         music(credits_music.music.clone()),
//     ));
// }
