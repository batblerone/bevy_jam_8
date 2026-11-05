//! Player sprite animation.
//! This is based on multiple examples and may be very different for your game.
//! - [Sprite flipping](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_flipping.rs)
//! - [Sprite animation](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_animation.rs)
//! - [Timers](https://github.com/bevyengine/bevy/blob/latest/examples/time/timers.rs)

use bevy::prelude::*;
use rand::prelude::*;
use std::time::Duration;

use crate::{
    AppSystems, PausableSystems,
    audio::sound_effect,
    game::{movement::MovementIntent, player::PlayerAssets},
};

pub(super) fn plugin(app: &mut App) {
    // Animate and play sound effects based on controls.
    app.add_systems(
        Update,
        (
            update_animation_timer.in_set(AppSystems::TickTimers),
            (
                (update_animation_movement, update_animation_atlas).chain(),
                trigger_step_sound_effect,
            )
                .chain()
                .in_set(AppSystems::Update),
        )
            .in_set(PausableSystems),
    );
}

/// Update the sprite direction and animation state (idling/walking).
fn update_animation_movement(
    mut player_query: Query<(&MovementIntent, &mut PlayerAnimation, &mut Sprite)>,
) {
    for (intent, mut animation, mut sprite) in &mut player_query {
        let state = if intent.is_moving() {
            PlayerAnimationState::Walking
        } else {
            PlayerAnimationState::Idling
        };

        animation.update_state(state);

        // Use last facing direction, including when idle.
        // Ignore vertical-only facing to retain the last flip.
        if intent.facing.x.abs() > f32::EPSILON {
            sprite.flip_x = intent.facing.x < 0.0;
        }
    }
}

/// Advance animation based on movement and sprinting.
fn update_animation_timer(
    time: Res<Time>,
    mut query: Query<(&MovementIntent, &mut PlayerAnimation)>,
) {
    for (intent, mut animation) in &mut query {
        animation.update_timer(time.delta(), intent.sprinting);
    }
}

/// Update the texture atlas to reflect changes in the animation.
fn update_animation_atlas(mut query: Query<(&PlayerAnimation, &mut Sprite)>) {
    for (animation, mut sprite) in &mut query {
        let Some(atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };

        let target_index = animation.get_atlas_index();
        if atlas.index != target_index {
            atlas.index = target_index;
        }
    }
}

/// If the player is moving, play a step sound effect synchronized with the
/// animation.
fn trigger_step_sound_effect(
    mut commands: Commands,
    player_assets: If<Res<PlayerAssets>>,
    step_query: Query<&PlayerAnimation>,
) {
    for animation in &step_query {
        if animation.changed() && (animation.frame == 2 || animation.frame == 5) {
            let rng = &mut rand::rng();
            let random_step = player_assets.steps.choose(rng).unwrap().clone();

            commands.spawn(sound_effect(random_step));
        }
    }
}

/// Component that tracks player's animation state.
/// It is tightly bound to the texture atlas we use.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct PlayerAnimation {
    timer: Timer,
    frame: usize,
    state: PlayerAnimationState,
}

#[derive(Reflect, PartialEq, Eq, Clone, Copy, Default)]
pub enum PlayerAnimationState {
    #[default]
    Idling,
    Walking,
}

impl Default for PlayerAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerAnimation {
    /// The number of idle frames.
    const IDLE_FRAMES: usize = 1;
    /// The duration of each idle frame. Also used for directional animation.
    const IDLE_INTERVAL: Duration = Duration::from_millis(500);
    /// The number of walking frames.
    const WALKING_FRAMES: usize = 6;
    /// The duration of each walking frame.
    const WALKING_INTERVAL: Duration = Duration::from_millis(50);
    /// same but for sprinting
    const SPRIINT_INTERVAL: Duration = Duration::from_millis(10);
    /// The number of frames per directional animation loop
    const FRAMES_PER_DIRECTION: usize = 1; // One frame each direction

    pub fn new() -> Self {
        Self {
            timer: Timer::new(Self::WALKING_INTERVAL, TimerMode::Repeating),
            frame: 0,
            state: PlayerAnimationState::Idling,
        }
    }

    /// Update animation state if it changes.
    pub fn update_state(&mut self, state: PlayerAnimationState) {
        if self.state != state {
            self.state = state;
            self.frame = 0;
            self.timer.reset();
        }
    }

    /// Update the timer
    pub fn update_timer(&mut self, delta: Duration, sprinting: bool) {
        if self.state == PlayerAnimationState::Idling {
            return;
        }

        let interval = if sprinting {
            Self::SPRIINT_INTERVAL
        } else {
            Self::WALKING_INTERVAL
        };

        self.timer.set_duration(interval);
        self.timer.tick(delta);

        let advance = self.timer.times_finished_this_tick() as usize;

        if advance > 0 {
            self.frame = (self.frame + advance) % Self::WALKING_FRAMES;
        }
    }

    /// Returns a sprite index in the atlas based on direction.
    /// We can specify that each direction occupies a n to n + x number
    /// of indicies. For now our n is 1 and each animation is only
    /// 1 frame, so 1 + 0.
    pub fn get_atlas_index(&self) -> usize {
        match self.state {
            PlayerAnimationState::Idling => 0,
            PlayerAnimationState::Walking => self.frame,
        }
    }

    pub fn changed(&self) -> bool {
        self.state == PlayerAnimationState::Walking && self.timer.just_finished()
    }
}
