//! Handle player input and translate it into movement through a character
//! controller. A character controller is the collection of systems that govern
//! the movement of characters.
//!
//! In our case, the character controller has the following logic:
//! - Set [`MovementController`] intent based on directional keyboard input.
//!   This is done in the `player` module, as it is specific to the player
//!   character.
//! - Apply movement based on [`MovementController`] intent and maximum speed.
//! - Wrap the character within the window.
//!
//! Note that the implementation used here is limited for demonstration
//! purposes. If you want to move the player in a smoother way,
//! consider using a [fixed timestep](https://github.com/bevyengine/bevy/blob/main/examples/movement/physics_in_fixed_timestep.rs).

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_input_context::<Movement>()
        .add_observer(walk)
        .add_observer(stop_walking)
        .add_observer(start_sprinting)
        .add_observer(stop_sprinting)
        .add_systems(PreUpdate, apply_intent.after(EnhancedInputSystems::Trigger));
}

/// Lets change our shit to bevy enhanced input
/// We only have one movement context, and that is the player (or npc) is moving
#[derive(Component, Default, Clone)]
pub struct Movement;

/// Structs for the movement vectors
#[derive(InputAction, Debug)]
#[action_output(Vec2)]
pub struct Walking;

/// Struct for toggling faster movement
#[derive(InputAction, Debug)]
#[action_output(bool)]
pub struct Sprinting;

/// These are the movement parameters for our character controller.
/// For now, this is only used for a single player, but it could power NPCs or
/// other players as well.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
#[require(MovementIntent)]
pub struct MovementController {
    /// Speed in pixels per second
    pub speed: f32,
    /// Sprint multiplier
    pub sprint_x: f32,
}

impl Default for MovementController {
    fn default() -> Self {
        Self {
            speed: 0.0,
            sprint_x: 3.0,
        }
    }
}

/// What the character is trying to do this frame, used for animation
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct MovementIntent {
    // the desired direction, zero when idle
    pub direction: Vec2,
    // last non zero direction normalized
    pub facing: Vec2,
    pub sprinting: bool,
}

impl Default for MovementIntent {
    fn default() -> Self {
        Self {
            direction: Vec2::ZERO,
            facing: Vec2::NEG_Y,
            sprinting: false,
        }
    }
}

impl MovementIntent {
    pub fn is_moving(&self) -> bool {
        self.direction != Vec2::ZERO
    }

    // Only have left and right sprite sheet
    #[allow(dead_code)]
    pub fn facing(&self) -> Facing {
        let f = self.facing;
        if f.x > 0.0 {
            Facing::Right
        } else {
            Facing::Left
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Facing {
    Right,
    Left,
}

/// Define the movement controller as a scene, that can be attached to other scenes.
/// We can then attach our movement and context to the parent scene.
pub fn movement_controller() -> impl Scene {
    bsn! {
        MovementController
        Movement
        Actions<Movement> [
            Action::<Walking>::new()
            DeadZone
            Bindings [
                {preset::cardinal::wasd_keys()}
                --
                {preset::cardinal::arrows()}
                --
                {preset::cardinal::dpad()}
                --
                {preset::axial::left_stick()}
            ]
            --
            Action::<Sprinting>::new()
            Bindings [
                Binding(KeyCode::ShiftLeft)
                --
                Binding(GamepadButton::West)
            ]
        ]
    }
}

/// Now define how movement actions affect the context
fn walk(walk: On<Fire<Walking>>, mut intents: Query<&mut MovementIntent>) {
    if let Ok(mut intent) = intents.get_mut(walk.context) {
        debug!("{}", walk.value);
        intent.direction = walk.value;
        if let Some(dir) = walk.value.try_normalize() {
            intent.facing = dir;
        }
    }
}

// Release all movement keys so movement stops, don't slide around
fn stop_walking(walk: On<Complete<Walking>>, mut intents: Query<&mut MovementIntent>) {
    if let Ok(mut intent) = intents.get_mut(walk.context) {
        intent.direction = Vec2::ZERO;
    }
}

fn start_sprinting(sprint: On<Start<Sprinting>>, mut intents: Query<&mut MovementIntent>) {
    if let Ok(mut intent) = intents.get_mut(sprint.context) {
        intent.sprinting = true;
    }
}

fn stop_sprinting(sprint: On<Complete<Sprinting>>, mut intents: Query<&mut MovementIntent>) {
    if let Ok(mut intent) = intents.get_mut(sprint.context) {
        intent.sprinting = false;
    }
}

/// Now apply the intents to the physics
fn apply_intent(
    mut move_things: Query<
        (&MovementIntent, &MovementController, &mut LinearVelocity),
        Changed<MovementIntent>,
    >,
) {
    for (intent, controller, mut velocity) in &mut move_things {
        let mut speed = controller.speed;
        if intent.sprinting {
            speed *= controller.sprint_x;
        }
        velocity.0 = intent.direction * speed;
    }
}
