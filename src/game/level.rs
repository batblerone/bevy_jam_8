//! The level: a single shop that has two regions.
//!
//! A herb growing region where herbs are planted for the next night.
//! And a brewing and custoer counter that the player will interact with
//! throughout the game.

use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    PausableSystems,
    asset_tracking::LoadResource,
    audio::music,
    game::{
        player::{Player, PlayerAssets, SelectedPlayer, player},
        progression::LevelProgress,
    },
    game_manager::{GamePhase, RunSeed, Screen},
};

/// A custom colour for showing transparent backgrounds (May be useful later)
const CUSTOM_BG_COLOR: Color = Color::srgb_u8(99, 197, 207);

/// Remembers the app's background colour (set by the splash screen) while
/// gameplay uses the custom colour instead.
#[derive(Resource)]
struct PreviousClearColor(Color);

fn use_custom_background_colour(mut commands: Commands, clear: Res<ClearColor>) {
    commands.insert_resource(PreviousClearColor(clear.0));
    commands.insert_resource(ClearColor(CUSTOM_BG_COLOR));
}

fn restore_background(mut commands: Commands, previous: Option<Res<PreviousClearColor>>) {
    if let Some(previous) = previous {
        commands.insert_resource(ClearColor(previous.0));
        commands.remove_resource::<PreviousClearColor>();
    }
}

pub(super) fn plugin(app: &mut App) {
    app.load_resource::<LevelAssets>();

    // `spawn_level` itself is registered in screens/gameplay.rs as
    // `spawn_level.after(RunSetup)` (tutorial Step 1.3).
    app.add_systems(OnEnter(Screen::Gameplay), use_custom_background_colour);
    app.add_systems(OnExit(Screen::Gameplay), restore_background);
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct LevelAssets {
    #[dependency]
    music: Handle<AudioSource>,
    /// Not drawn yet: water comes from the clear colour. Kept for a scrolling backdrop.
    #[allow(dead_code)]
    background: Handle<Image>,
}

impl FromWorld for LevelAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            music: assets.load("audio/music/Fluffing A Duck.ogg"),
            background: assets.load("images/background.png"),
        }
    }
}

/// Spawns the level: music, the crow, and enough river to fill the screen.
pub fn spawn_level(
    mut commands: Commands,
    level_assets: Res<LevelAssets>,
    player_assets: Res<PlayerAssets>,
    selected: Res<SelectedPlayer>,
    seed: Res<RunSeed>,
    progress: Res<LevelProgress>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    commands.spawn((
        Name::new("Level"),
        Transform::default(),
        Visibility::default(),
        DespawnOnExit(Screen::Gameplay),
        children![(
            Name::new("Gameplay Music"),
            music(level_assets.music.clone())
        )],
    ));

    // The player is a scene, so it's spawned on its own. `place_new_player`
    // moves it into open water once the rows below exist.
    commands.spawn_scene(player(selected.0, &player_assets));
}
