//! Silly things that the witch can pick up
//!
//! Holding is one to one relationship pointing at whoever holds it, holder
//! gets [`Holding`] automatically.

use avian2d::prelude::*;
use bevy::{prelude::*, transform::TransformSystems};

use crate::{
    game::{
        data::IngredientKind,
        interaction::{Interactable, Interacted, InteractedWithNothing},
        movement::MovementIntent,
        physics::GameLayer,
        shop::{self, YSort, z},
        stations::{Station, StationKind},
    },
    game_manager::Screen,
};

pub(super) fn plugin(app: &mut App) {
    app.add_observer(pick_up_item)
        .add_observer(interact_with_station)
        .add_observer(drop_held_item)
        .add_observer(on_held)
        .add_observer(on_released)
        .add_systems(
            PostUpdate,
            carry_held_items
                // After y-sorting set the holder's z, before rendering transforms.
                .after(shop::y_sort)
                .before(TransformSystems::Propagate),
        );
}

/// What an item is. Add potions etc. here as the game grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Ingredient(IngredientKind),
    EmptyFlask,
}

impl ItemKind {
    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Ingredient(IngredientKind::Fireweed) => "Fireweed",
            ItemKind::Ingredient(IngredientKind::Silkworm) => "Silkworm",
            ItemKind::Ingredient(IngredientKind::Glowcap) => "Glowcap",
            ItemKind::EmptyFlask => "Flask",
        }
    }

    /// Placeholder art colour until item sprites exist.
    pub fn color(self) -> Color {
        match self {
            ItemKind::Ingredient(IngredientKind::Fireweed) => Color::srgb(0.90, 0.35, 0.20),
            ItemKind::Ingredient(IngredientKind::Silkworm) => Color::srgb(0.92, 0.92, 0.85),
            ItemKind::Ingredient(IngredientKind::Glowcap) => Color::srgb(0.75, 0.95, 0.35),
            ItemKind::EmptyFlask => Color::srgb(0.70, 0.88, 0.95),
        }
    }
}

/// Size of an item's placeholder square and pickup collider, in pixels.
const ITEM_SIZE: f32 = 12.0;
/// Where a held item sits relative to the witch (x is flipped with facing).
const HOLD_OFFSET: Vec2 = Vec2::new(12.0, -2.0);
/// How far in front of the witch a dropped item lands.
const DROP_DISTANCE: f32 = 18.0;

#[derive(Component, Clone, Copy, Debug)]
#[allow(dead_code)] // `kind` is for brewing / serving code to read.
pub struct Item {
    pub kind: ItemKind,
}

/// On an item: who (player or station) is holding it.
#[derive(Component, Debug)]
#[relationship(relationship_target = Holding)]
pub struct HeldBy(pub Entity);

/// On a holder: the one item it holds. Managed by Bevy, never insert by hand.
#[derive(Component, Debug)]
#[relationship_target(relationship = HeldBy)]
pub struct Holding(Entity);

impl Holding {
    pub fn item(&self) -> Entity {
        self.0
    }
}

/// On a station: interacting with empty hands hands out a new item of this kind.
#[derive(Component, Clone, Copy, Debug)]
pub struct Dispenses(pub ItemKind);

impl StationKind {
    /// Stations you can set a held item down on.
    pub fn accepts_items(self) -> bool {
        matches!(
            self,
            StationKind::Counter
                | StationKind::ServiceCounter
                | StationKind::Cauldron
                | StationKind::Mortar
                | StationKind::DryingRack
                | StationKind::GrowBed
        )
    }
}

/// Fired on a station when an item is put on it. Brewing, serving, planting...
/// observe this (`On<ItemPlaced>`) in their own modules.
#[derive(EntityEvent, Debug)]
#[allow(dead_code)] // Fields are for the brewing / serving observers to read.
pub struct ItemPlaced {
    /// The station.
    pub entity: Entity,
    pub item: Entity,
    pub by: Entity,
}

/// An item lying loose in the world. Spawn with `HeldBy(holder)` added to
/// start it in someone's hands.
pub fn item(kind: ItemKind) -> impl Bundle {
    (
        Name::new(kind.name()),
        Item { kind },
        Interactable,
        Sprite::from_color(kind.color(), Vec2::splat(ITEM_SIZE)),
        // Static sensor: findable by the interaction reach query, never blocks.
        RigidBody::Static,
        Collider::rectangle(ITEM_SIZE, ITEM_SIZE),
        Sensor,
        GameLayer::pickup_layers(),
        YSort,
        DespawnOnExit(Screen::Gameplay),
    )
}

/// While held: invisible to the interaction query, and z comes from the holder.
fn on_held(held: On<Insert<HeldBy>>, mut commands: Commands) {
    commands
        .entity(held.entity)
        .try_insert(ColliderDisabled)
        .try_remove::<YSort>();
}

/// Back on the floor (dropped, or the holder despawned): findable and sorted again.
fn on_released(released: On<Remove<HeldBy>>, mut commands: Commands) {
    // `try_` because this also runs when the item itself is despawned.
    commands
        .entity(released.entity)
        .try_remove::<ColliderDisabled>()
        .try_insert(YSort);
}

/// Keep every held item at its holder.
fn carry_held_items(
    holders: Query<(&Transform, Option<&MovementIntent>), Without<Item>>,
    mut items: Query<(&HeldBy, &mut Transform), With<Item>>,
) {
    for (held_by, mut transform) in &mut items {
        let Ok((holder, intent)) = holders.get(held_by.0) else {
            continue;
        };
        let offset = match intent {
            // A character: in the hand on the side it faces, just in front.
            Some(intent) => {
                let side = if intent.facing.x < 0.0 { -1.0 } else { 1.0 };
                Vec3::new(HOLD_OFFSET.x * side, HOLD_OFFSET.y, 0.1)
            }
            // A station: centred on top of it.
            None => Vec3::new(0.0, 4.0, 0.5),
        };
        transform.translation = holder.translation + offset;
    }
}

/// E on a loose item with empty hands: pick it up.
fn pick_up_item(
    interacted: On<Interacted>,
    mut commands: Commands,
    items: Query<(), With<Item>>,
    holders: Query<Has<Holding>>,
) {
    let (item, by) = (interacted.entity, interacted.by);
    if items.contains(item) && holders.get(by) == Ok(false) {
        commands.entity(item).insert(HeldBy(by));
    }
}

/// E on a station.
fn interact_with_station(
    interacted: On<Interacted>,
    mut commands: Commands,
    holders: Query<Option<&Holding>>,
    stations: Query<(&Station, &Transform, Option<&Dispenses>, Option<&Holding>)>,
) {
    let (target, by) = (interacted.entity, interacted.by);
    let Ok((station, station_transform, dispenses, on_station)) = stations.get(target) else {
        return;
    };
    let Ok(in_hand) = holders.get(by).map(|h| h.map(Holding::item)) else {
        return;
    };

    match (in_hand, on_station.map(Holding::item)) {
        // Empty hands, something on the station: take it.
        (None, Some(item)) => {
            commands.entity(item).insert(HeldBy(by));
        }
        // Empty hands, empty station: dispense if it's a crate / shelf.
        (None, None) => {
            if let Some(Dispenses(kind)) = dispenses {
                commands.spawn((
                    item(*kind),
                    // Start at the station so there's no one-frame flash at the origin.
                    Transform::from_translation(station_transform.translation),
                    HeldBy(by),
                ));
            }
        }
        // Holding something at the trash: throw it away.
        (Some(item), _) if station.kind == StationKind::Trash => {
            commands.entity(item).despawn();
        }
        // Holding something at a free station that takes items: put it down there.
        (Some(item), None) if station.kind.accepts_items() => {
            commands.entity(item).insert(HeldBy(target));
            commands.trigger(ItemPlaced {
                entity: target,
                item,
                by,
            });
        }
        // Station full, or doesn't take items.
        _ => {}
    }
}

/// E with nothing in reach while holding: drop it on the floor in front.
fn drop_held_item(
    nothing: On<InteractedWithNothing>,
    mut commands: Commands,
    holders: Query<(&Holding, &Transform, Option<&MovementIntent>)>,
    mut transforms: Query<&mut Transform, Without<Holding>>,
) {
    let Ok((holding, holder, intent)) = holders.get(nothing.entity) else {
        return;
    };
    let item = holding.item();
    let facing = intent.map_or(Vec2::NEG_Y, |i| i.facing);
    if let Ok(mut transform) = transforms.get_mut(item) {
        let spot = holder.translation.truncate() + facing * DROP_DISTANCE;
        transform.translation = spot.extend(z::WORLD);
    }
    commands.entity(item).remove::<HeldBy>();
}
