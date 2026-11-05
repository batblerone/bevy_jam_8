//! Pausing for gameplay and physics
//!
//! Since we use enhanced_input with observers, no AppSystem for it
//! So lets pause the input contexts while Pause(true). Needed for when
//! using controller. Don't wanna walk the witch around when using in
//! the menues and all.

use std::marker::PhantomData;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::game_manager::Pause;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Pause(true)), pause_physics)
        .add_systems(OnExit(Pause(true)), resume_physics);
}

pub trait PausableInputExt {
    /// Kind of like `add_input_context` but for pausing
    fn add_pausable_input_context<C: Component>(&mut self) -> &mut Self;
}

impl PausableInputExt for App {
    fn add_pausable_input_context<C: Component>(&mut self) -> &mut Self {
        self.add_input_context::<C>()
            .add_systems(OnEnter(Pause(true)), pause_context::<C>)
            .add_systems(OnExit(Pause(true)), resume_context::<C>)
    }
}

// Marks contexts to be switch offed that arent switched systematically
// (movement during dialog kinda deal)
#[derive(Component)]
struct PausedContext<C: Component>(PhantomData<C>);

fn pause_context<C: Component>(
    mut commands: Commands,
    contexts: Query<(Entity, &ContextActivity<C>)>,
) {
    for (entity, activity) in &contexts {
        if **activity {
            commands.entity(entity).insert((
                ContextActivity::<C>::INACTIVE,
                PausedContext::<C>(PhantomData),
            ));
        }
    }
}

fn resume_context<C: Component>(
    mut commands: Commands,
    contexts: Query<Entity, With<PausedContext<C>>>,
) {
    for entity in &contexts {
        commands.entity(entity)
            .insert(ContextActivity::<C>::INACTIVE) // incase things need a reset
            .insert(ContextActivity::<C>::ACTIVE)
            .remove::<PausedContext<C>>();
    }
}

fn pause_physics(mut time: ResMut<Time<Physics>>) {
    time.pause();
}

fn resume_physics(mut time: ResMut<Time<Physics>>) {
    time.unpause();
}
