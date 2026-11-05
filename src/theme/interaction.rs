use bevy::{
    input_focus::{InputFocus, InputFocusVisible},
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
};

use crate::{asset_tracking::LoadResource, audio::sound_effect};

pub(super) fn plugin(app: &mut App) {
    // Not a `PausableSystems` member: menus must react while the game is paused.
    app.add_systems(Update, apply_interaction_palette);
    // Hover sound for keyboard / gamepad navigation, like the mouse gets.
    app.add_systems(
        Update,
        play_sound_effect_on_focus.run_if(resource_changed::<InputFocus>),
    );

    app.load_resource::<InteractionAssets>();
    app.add_observer(play_sound_effect_on_click);
    app.add_observer(play_sound_effect_on_over);
}

/// Palette for widget interactions. Add this to an entity that supports
/// [`Interaction`]s, such as a button, to change its [`BackgroundColor`] based
/// on the current interaction state.
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct InteractionPalette {
    pub none: Color,
    pub hovered: Color,
    pub pressed: Color,
}

/// Better captures input events, reacts better by responding to `PointerOver` / `PointerOut` events instead.
///
/// A button focused by keyboard / gamepad navigation looks hovered too.
fn apply_interaction_palette(
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    mut palette_query: Query<(
        Entity,
        &InteractionPalette,
        &Hovered,
        Has<Pressed>,
        &mut BackgroundColor,
    )>,
) {
    for (entity, palette, hovered, pressed, mut bg) in &mut palette_query {
        let focused = focus_visible.0 && focus.get() == Some(entity);
        let color = if pressed {
            palette.pressed
        } else if hovered.get() || focused {
            palette.hovered
        } else {
            palette.none
        };
        bg.set_if_neq(BackgroundColor(color));
    }
}

fn apply_interaction_palette_on_click(
    click: On<PointerClick>,
    mut palette_query: Query<(&InteractionPalette, &mut BackgroundColor)>,
) {
    let Ok((palette, mut bg)) = palette_query.get_mut(click.event_target()) else {
        return;
    };

    *bg = palette.pressed.into();
}

fn apply_interaction_palette_on_release(
    click: On<PointerRelease>,
    mut palette_query: Query<(&InteractionPalette, &mut BackgroundColor)>,
) {
    let Ok((palette, mut bg)) = palette_query.get_mut(click.event_target()) else {
        return;
    };

    *bg = palette.hovered.into();
}

fn apply_interaction_palette_on_over(
    over: On<PointerOver>,
    mut palette_query: Query<(&InteractionPalette, &mut BackgroundColor)>,
) {
    let Ok((palette, mut bg)) = palette_query.get_mut(over.event_target()) else {
        return;
    };

    *bg = palette.hovered.into();
}

fn apply_interaction_palette_on_out(
    out: On<PointerOut>,
    mut palette_query: Query<(&InteractionPalette, &mut BackgroundColor)>,
) {
    let Ok((palette, mut bg)) = palette_query.get_mut(out.event_target()) else {
        return;
    };

    *bg = palette.none.into();
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct InteractionAssets {
    #[dependency]
    hover: Handle<AudioSource>,
    #[dependency]
    click: Handle<AudioSource>,
}

impl FromWorld for InteractionAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            hover: assets.load("audio/sound_effects/button_hover.ogg"),
            click: assets.load("audio/sound_effects/button_click.ogg"),
        }
    }
}

fn play_sound_effect_on_click(
    on: On<PointerClick>,
    interaction_assets: If<Res<InteractionAssets>>,
    interaction_entities: Query<Entity, With<InteractionPalette>>,
    mut commands: Commands,
) {
    if interaction_entities.contains(on.event_target()) {
        commands.spawn(sound_effect(interaction_assets.click.clone()));
    }
}

fn play_sound_effect_on_over(
    on: On<PointerOver>,
    interaction_assets: If<Res<InteractionAssets>>,
    interaction_entities: Query<Entity, With<InteractionPalette>>,
    mut commands: Commands,
) {
    if interaction_entities.contains(on.event_target()) {
        commands.spawn(sound_effect(interaction_assets.hover.clone()));
    }
}

/// Keyboard / gamepad moved focus to another button: play the hover sound.
fn play_sound_effect_on_focus(
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    interaction_assets: If<Res<InteractionAssets>>,
    interaction_entities: Query<Entity, With<InteractionPalette>>,
    mut commands: Commands,
) {
    if focus_visible.0
        && focus
            .get()
            .is_some_and(|e| interaction_entities.contains(e))
    {
        commands.spawn(sound_effect(interaction_assets.hover.clone()));
    }
}
