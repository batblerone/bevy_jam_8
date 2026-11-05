//! Keyboard and gamepad control for every menu.
//!
//! Menu input is its own bevy_enhanced_input context, [`MenuControls`], on a
//! single always-alive entity. It is *not* pausable: it has to keep working
//! while the game is paused, which is exactly when the gameplay contexts are
//! switched off (see `game/pause.rs`)
//!
//! The mouse still works as before; hovering a button also moves focus to it,
//! so switching between mouse and gamepad never leaves two buttons "selected".

use std::time::Duration;

use bevy::{
    camera::NormalizedRenderTarget,
    input_focus::{
        FocusCause, InputFocus, InputFocusVisible,
        directional_navigation::DirectionalNavigationPlugin,
    },
    math::CompassOctant,
    picking::{
        backend::HitData,
        pointer::{Location, PointerId},
    },
    prelude::*,
    ui::{
        UiGlobalTransform, UiSystems,
        auto_directional_navigation::{AutoDirectionalNavigation, AutoDirectionalNavigator},
    },
};
use bevy_enhanced_input::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(DirectionalNavigationPlugin)
        .add_input_context::<MenuControls>()
        .add_systems(Startup, spawn_menu_controls)
        .add_systems(
            PostUpdate,
            // Needs this frame's layout to know where the buttons are.
            focus_first_button.after(UiSystems::Layout),
        )
        .add_observer(navigate)
        .add_observer(confirm)
        .add_observer(focus_follows_mouse);
}

// The context

#[derive(Component, Default, Clone)]
pub struct MenuControls;

#[derive(InputAction)]
#[action_output(Vec2)]
pub struct MenuNavigate;

#[derive(InputAction)]
#[action_output(bool)]
pub struct MenuConfirm;

/// Escape / gamepad East. Closes the current menu or goes up one level.
#[derive(InputAction)]
#[action_output(bool)]
pub struct MenuBack;

/// P / gamepad Start. Opens the pause menu in gameplay, closes any menu.
#[derive(InputAction)]
#[action_output(bool)]
pub struct TogglePause;

fn spawn_menu_controls(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Name::new("Menu Controls")
        MenuControls
        Actions<MenuControls> [
            Action::<MenuNavigate>::new()
            DeadZone
            // Move once on press, then repeat while held (like key repeat).
            ~Pulse::new(0.12).with_initial_delay(0.4)
            Bindings [
                {preset::cardinal::arrows()}
                --
                {preset::cardinal::dpad()}
                --
                {preset::axial::left_stick()}
            ]
            --
            Action::<MenuConfirm>::new()
            Bindings [
                Binding(KeyCode::Enter)
                --
                Binding(KeyCode::Space)
                --
                Binding(GamepadButton::South)
            ]
            --
            Action::<MenuBack>::new()
            Bindings [
                Binding(KeyCode::Escape)
                --
                Binding(GamepadButton::East)
            ]
            --
            Action::<TogglePause>::new()
            Bindings [
                Binding(KeyCode::Escape)
                --
                Binding(GamepadButton::Start)
            ]
        ]
    });
}

/// Run condition: true on the frame action `South` started (was pressed).
///
/// ```ignore
/// go_back.run_if(in_state(Menu::Pause).and_then(action_just_started::<MenuBack>))
/// ```
pub fn action_just_started<A: InputAction>(actions: Query<&ActionEvents, With<Action<A>>>) -> bool {
    actions
        .iter()
        .any(|events| events.contains(ActionEvents::START))
}


type NavigableButtons<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static InheritedVisibility,
    ),
    With<AutoDirectionalNavigation>,
>;

/// Whenever a menu is up and nothing valid is focused (menu just opened, or
/// the focused button was despawned), focus its top-most button. That way
/// gamepad A works immediately, without a d-pad press first.
fn focus_first_button(mut focus: ResMut<InputFocus>, buttons: NavigableButtons) {
    let is_usable = |(_, node, _, visible): &(
        Entity,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
    )| { visible.get() && node.size() != Vec2::ZERO };
    if focus
        .get()
        .is_some_and(|entity| buttons.get(entity).is_ok_and(|b| is_usable(&b)))
    {
        return;
    }
    let top_most = buttons
        .iter()
        .filter(is_usable)
        // UI y grows downwards: smallest y is the top. Ties go left to right.
        .min_by(|(_, _, a, _), (_, _, b, _)| {
            (a.translation.y, a.translation.x)
                .partial_cmp(&(b.translation.y, b.translation.x))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    if let Some((entity, ..)) = top_most {
        focus.set(entity, FocusCause::Navigated);
    }
}

fn navigate(
    nav: On<Fire<MenuNavigate>>,
    mut navigator: AutoDirectionalNavigator,
    mut focus_visible: ResMut<InputFocusVisible>,
) {
    // Snap to the dominant axis so a slightly diagonal stick still moves
    // straight up/down a list instead of trying a diagonal neighbour.
    let v = nav.value;
    let direction = if v.x.abs() > v.y.abs() {
        if v.x > 0.0 {
            CompassOctant::East
        } else {
            CompassOctant::West
        }
    } else if v.y > 0.0 {
        CompassOctant::North
    } else {
        CompassOctant::South
    };
    focus_visible.0 = true;
    // No neighbour that way (end of the list): stay put.
    let _ = navigator.navigate(direction);
}

fn confirm(
    _: On<Start<MenuConfirm>>,
    mut commands: Commands,
    focus: Res<InputFocus>,
    mut focus_visible: ResMut<InputFocusVisible>,
    buttons: Query<(), With<AutoDirectionalNavigation>>,
) {
    let Some(button) = focus.get().filter(|e| buttons.contains(*e)) else {
        return;
    };
    focus_visible.0 = true;
    // Same event a mouse click produces, so every existing button action,
    // the click sound and the pressed palette react to it.
    commands.trigger(PointerClick {
        entity: button,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 0,
                    height: 0,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        hit: HitData {
            camera: Entity::PLACEHOLDER,
            depth: 0.0,
            position: None,
            normal: None,
            extra: None,
        },
        count: 1,
        duration: Duration::from_secs_f32(0.1),
    });
}

/// Hovering a button with the mouse focuses it, and hides the gamepad focus
/// highlight (the hover colour already shows where the mouse is).
fn focus_follows_mouse(
    over: On<PointerOver>,
    mut focus: ResMut<InputFocus>,
    mut focus_visible: ResMut<InputFocusVisible>,
    buttons: Query<(), With<AutoDirectionalNavigation>>,
) {
    if buttons.contains(over.entity) {
        focus.set(over.entity, FocusCause::Navigated);
        focus_visible.0 = false;
    }
}
