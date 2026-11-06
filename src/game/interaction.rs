//! An interaction module to map out character interaction
//!
//! Uses different context than movement.rs. Handles events
//! that the character can interact with.

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::game::{items::Holding, pause::PausableInputExt};

pub(super) fn plugin(app: &mut App) {
    app.add_input_context::<CharActions>()
        .add_observer(interact)
        .add_observer(use_object);
}

/// Context for interactions
#[derive(Component, Default, Clone)]
pub struct CharActions;

/// Struct for interaction (Pickups, Delivering objects, Interacting with statiosn)
#[derive(InputAction, Debug)]
#[action_output(bool)]
pub struct CharInteract;

/// Struct for using items they are holding
#[derive(InputAction, Debug)]
#[action_output(bool)]
pub struct CharUse;

/// How far interactions reach, in pixels
#[derive(Component, Clone)]
pub struct InteractionReach(pub f32);

impl Default for InteractionReach {
    fn default() -> Self {
        Self(200.0)
    }
}

/// Something with this marker is interactable with by the player (doors, NPC, cauldrons)
#[derive(Component, Default, Clone)]
pub struct Interactable;

/// Someththing with this marker is something useable by the character (Beds, Shelves, Pickups)
#[derive(Component, Default, Clone)]
pub struct Useable;

/// Fireed on the target object when it is interacted with `by` a character
#[derive(EntityEvent)]
pub struct Interacted {
    pub entity: Entity, // Target
    pub by: Entity,     // Who interacted with it
}

/// when the character has nothing to hold on to
#[derive(EntityEvent)]
pub struct InteractedWithNothing {
    pub entity: Entity, // The character
}

/// Same thing as interacted, but for used
#[derive(EntityEvent)]
pub struct Used {
    pub entity: Entity,
    pub by: Entity,
}

/// A scene to compose our character interaction suite
pub fn interaction_controller() -> impl Scene {
    bsn! {
        InteractionReach
        CharActions
        Actions<CharActions> [
            Action::<CharInteract>::new()
            ActionSettings { require_reset: true }
            Bindings [
                Binding(KeyCode::KeyE)
                --
                Binding(GamepadButton::South)
            ]
            --
            Action::<CharUse>::new()
            ActionSettings { require_reset: true }
            Bindings [
                Binding(KeyCode::KeyQ)
                --
                Binding(GamepadButton::East)
            ]
        ]
    }
}

/// Observers that actually do things
fn interact(
    press: On<Start<CharInteract>>,
    mut commands: Commands,
    spatial: SpatialQuery,
    chars: Query<(&Position, &InteractionReach)>,
    targets: Query<(Entity, &Position), With<Interactable>>,
    collider_of: Query<&ColliderOf>,
) {
    let by = press.context;
    if let Some(entity) = nearest_in_reach(by, &spatial, &chars, &targets, &collider_of) {
        commands.trigger(Interacted { entity, by });
    } else {
        commands.trigger(InteractedWithNothing { entity: by });
    }
}

// Holding something, use it, else find nearest reachable in reach
fn use_object(
    press: On<Start<CharUse>>,
    mut commands: Commands,
    spatial: SpatialQuery,
    chars: Query<(&Position, &InteractionReach)>,
    targets: Query<(Entity, &Position), With<Useable>>,
    collider_of: Query<&ColliderOf>,
    holding: Query<&Holding>,
) {
    let by = press.context;
    if let Ok(held) = holding.get(by) {
        commands.trigger(Used {
            entity: held.item(),
            by,
        });
        return;
    }
    if let Some(entity) = nearest_in_reach(by, &spatial, &chars, &targets, &collider_of) {
        commands.trigger(Used { entity, by });
    }
}

// Nearest entity matching `targets` with a collider that overlaps the players
// reach circle. Must have marker on it.
fn nearest_in_reach<F: bevy::ecs::query::QueryFilter>(
    character: Entity,
    spatial: &SpatialQuery,
    chars: &Query<(&Position, &InteractionReach)>,
    targets: &Query<(Entity, &Position), F>,
    collider_of: &Query<&ColliderOf>,
) -> Option<Entity> {
    let Ok((origin, reach)) = chars.get(character) else {
        return None;
    };

    spatial
        .shape_intersections(
            &Collider::circle(reach.0),
            origin.0,
            0.0,
            &SpatialQueryFilter::default(),
        )
        .into_iter()
        .filter(|&hit| collider_of.get(hit).map(|c| c.body) != Ok(character) && hit != character)
        .filter_map(|hit| {
            targets.get(hit).ok().or_else(|| {
                let body = collider_of.get(hit).ok()?.body;
                targets.get(body).ok()
            })
        })
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(origin.0)
                .total_cmp(&b.distance_squared(origin.0))
        })
        .map(|(entity, _)| entity)
}
