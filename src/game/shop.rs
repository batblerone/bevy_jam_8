//! The shop is painted in Tiled. Point objects on its `Gameplay` object layer
//! become gameplay entities; station behavior lives in `game::stations` scenes.
//!
//! Marker names are `PlayerSpawn`, `CustomerDoor`, `QueueSpot:<index>`, and
//! `Station:<StationKind>` (for example `Station:Cauldron`). Neither a custom
//! Tiled property export nor tile IDs are required to place a station.

use avian2d::prelude::*;
use bevy::{camera::ScalingMode, prelude::*, transform::TransformSystems};
use bevy_ecs_tiled::prelude::*;

use crate::{
    MainCamera, game::{
        animation::{CauldronAnimation, GrowBedAnimation}, physics::GameLayer, player::Player, stations::{
            StationAssets, StationKind, cauldron::Cauldron, grow_bed::GrowBed, stations,
        },
    }, game_manager::{RunSetup, Screen},
};

/// Current tile size in ShopMap.tmx. Also used as the station footprint unit.
pub const TILE_SIZE: f32 = 128.0;
const CAMERA_VIEW_HEIGHT: f32 = TILE_SIZE * 15.0;
const Y_SORT_FACTOR: f32 = 0.001;

pub(super) fn plugin(app: &mut App) {
    // Marker objects only need a name, so there's no requirement to export
    // reflected types into the project or write files on WASM.
    let mut config = TiledPluginConfig::default();
    config.tiled_types_export_file = None;
    app.add_plugins((
        TiledPlugin(config),
        TiledPhysicsPlugin::<TiledPhysicsAvianBackend>::default(),
    ));

    app.add_systems(
        OnEnter(Screen::Gameplay),
        (spawn_shop, set_gameplay_camera).after(RunSetup),
    );
    app.add_systems(OnExit(Screen::Gameplay), restore_camera);
    app.add_systems(
        Update,
        (
            configure_tiled_colliders,
            hide_collision_layer,
            follow_player_camera,
        )
            .run_if(in_state(Screen::Gameplay)),
    );
    // Read marker GlobalTransforms only AFTER Tiled's nested transforms are
    // propagated. Entities spawned by these systems appear the next frame.
    app.add_systems(
        PostUpdate,
        (
            cleanup_removed_markers,
            spawn_tiled_objects,
            place_player_at_spawn,
        )
            .chain()
            .after(TransformSystems::Propagate)
            .run_if(in_state(Screen::Gameplay)),
    );
    app.add_systems(PostUpdate, y_sort.before(TransformSystems::Propagate));
}

#[allow(dead_code)]
pub mod z {
    pub const FLOOR: f32 = 0.0;
    pub const FLOOR_DECALS: f32 = 1.0;
    pub const WALLS: f32 = 2.0;
    pub const WORLD: f32 = 10.0;
    pub const OVERHEAD: f32 = 30.0;
    pub const WORLD_UI: f32 = 40.0;
}

/// Sort top-level gameplay sprites by their Y coordinate.
/// Station scenes are children of the origin-based Shop root, not of nested
/// Tiled object-layer entities, so their local coordinates are world XY.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct YSort;

pub fn y_sort(mut query: Query<&mut Transform, With<YSort>>) {
    for mut transform in &mut query {
        let target = z::WORLD + 5.0 - transform.translation.y * Y_SORT_FACTOR;
        if transform.translation.z != target {
            transform.translation.z = target;
        }
    }
}

#[derive(Component, Reflect, Clone, Default)]
#[reflect(Component)]
pub struct Shop;

#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct PlayerSpawn;

#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct CustomerDoor;

#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct QueueSpot {
    pub index: usize,
}

/// The original Tiled marker responsible for a spawned gameplay entity.
/// This lets us clean up scenes when the map hot-reloads its object layer.
#[derive(Component, Clone, Copy, Debug)]
struct FromTiledMarker(Entity);

#[derive(Component)]
struct MarkerHandled;

#[derive(Component)]
struct PlayerPositioned;

/// Spawn the actual TMX file, rather than procedural floor/walls/stations.
pub fn spawn_shop(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Name::new("Shop"),
        Shop,
        TiledMap(asset_server.load("map/ShopMap.tmx")),
        TilemapAnchor::Center,
        // Tiled layers use a narrow band of depth; the gameplay entities get
        // Y-sorted in a separate band above the map artwork.
        TiledMapLayerZOffset(0.1),
        TiledPhysicsSettings::<TiledPhysicsAvianBackend> {
            objects_layer_filter: TiledFilter::None,
            tiles_layer_filter: TiledFilter::Names(vec!["Collision".into()]),
            ..default()
        },
        DespawnOnExit(Screen::Gameplay),
    ));
}

/// The Tiled collision backend uses tileset collision geometry. The supplied
/// TSX defines a rectangle for tile ID 244 (global GID 245).
/// Assign the same Avian collision layer as our previous shop walls.
fn configure_tiled_colliders(
    mut commands: Commands,
    mut events: MessageReader<TiledEvent<ColliderCreated>>,
) {
    for event in events.read() {
        commands
            .entity(event.origin)
            .try_insert((RigidBody::Static, GameLayer::wall_layers()));
    }
}

/// Hide the collision layer so that it is not there but there
pub fn hide_collision_layer(mut layers: Query<(&TiledName, &mut Visibility), Added<TiledLayer>>) {
    for (name, mut visibility) in &mut layers {
        if name.0 == "Collision" {
            *visibility = Visibility::Hidden;
        }
    }
}

/// Parse a point object's name. Simple Tiled names are deliberately used
/// rather than requiring custom-property export/import before the map runs.
#[derive(Debug, PartialEq, Eq)]
enum MarkerKind {
    Station(StationKind),
    Player,
    Door,
    Queue(usize),
}

fn parse_marker_name(name: &str) -> Option<MarkerKind> {
    match name {
        "PlayerSpawn" => Some(MarkerKind::Player),
        "CustomerDoor" => Some(MarkerKind::Door),
        _ => name
            .strip_prefix("QueueSpot:")
            .and_then(|value| value.parse().ok())
            .map(MarkerKind::Queue)
            .or_else(|| {
                name.strip_prefix("Station:")
                    .and_then(StationKind::from_tiled_name)
                    .map(MarkerKind::Station)
            }),
    }
}

/// Materialize gameplay scenes when the Tiled map's point objects exist.
fn spawn_tiled_objects(
    mut commands: Commands,
    shop: Single<Entity, With<Shop>>,
    assets: Option<Res<StationAssets>>,
    markers: Query<
        (
            Entity,
            &TiledName,
            &TiledObject,
            &TiledObjectId,
            &GlobalTransform,
        ),
        (With<TiledObject>, Without<MarkerHandled>),
    >,
) {
    for (marker, name, shape, id, transform) in &markers {
        let Some(kind) = parse_marker_name(&name.0) else {
            continue;
        };
        if !matches!(shape, TiledObject::Point) {
            warn!("Gameplay marker '{}' must be a Tiled point", name.0);
            commands.entity(marker).insert(MarkerHandled);
            continue;
        }

        let position = transform.translation().xy();
        match kind {
            MarkerKind::Station(kind) => {
                // The loading screen should make StationAssets available. If
                // not, leave this marker unhandled and retry next frame.
                let Some(assets) = assets.as_ref() else {
                    continue;
                };
                let scale = kind.spec().scale;
                let mut station = commands.spawn_scene(stations(kind, assets));
                station.insert((
                    Name::new(format!("{} #{}", kind.spec().name, id.0)),
                    Transform::from_translation(position.extend(z::WORLD))
                        .with_scale(Vec3::splat(scale)),
                    ChildOf(*shop),
                    FromTiledMarker(marker),
                ));
                match kind {
                    StationKind::Cauldron => {
                        station.insert((Cauldron::new(), CauldronAnimation::default()));
                    },
                    StationKind::GrowBed => {
                        station.insert((GrowBed::new(), GrowBedAnimation::default()));
                    },
                    _ => {}
                }
            }
            MarkerKind::Player => {
                // The existing player scene is spawned in level.rs. Its
                // position is set separately after this marker is available.
                commands.entity(marker).insert(PlayerSpawn);
            }
            MarkerKind::Door => {
                commands.spawn((
                    Name::new("Customer Door"),
                    CustomerDoor,
                    Transform::from_translation(position.extend(z::WORLD)),
                    ChildOf(*shop),
                    FromTiledMarker(marker),
                ));
            }
            MarkerKind::Queue(index) => {
                commands.spawn((
                    Name::new(format!("Queue Spot {index}")),
                    QueueSpot { index },
                    Transform::from_translation(position.extend(z::WORLD)),
                    ChildOf(*shop),
                    FromTiledMarker(marker),
                ));
            }
        }
        commands.entity(marker).insert(MarkerHandled);
    }
}

/// Delete runtime copies when a Tiled map reload removes their source marker.
fn cleanup_removed_markers(
    mut commands: Commands,
    runtime: Query<(Entity, &FromTiledMarker)>,
    tiled_objects: Query<(), With<TiledObject>>,
) {
    for (entity, source) in &runtime {
        if !tiled_objects.contains(source.0) {
            commands.entity(entity).despawn();
        }
    }
}

/// The player exists before the asynchronously loaded map. Wait until a
/// PlayerSpawn marker has a propagated world-space position before moving it.
fn place_player_at_spawn(
    mut commands: Commands,
    markers: Query<(&TiledName, &GlobalTransform), With<TiledObject>>,
    mut players: Query<(Entity, &mut Transform), (With<Player>, Without<PlayerPositioned>)>,
) {
    let Some(position) = markers
        .iter()
        .find(|(name, _)| name.0 == "PlayerSpawn")
        .map(|(_, transform)| transform.translation().xy())
    else {
        return;
    };

    for (entity, mut transform) in &mut players {
        transform.translation.x = position.x;
        transform.translation.y = position.y;
        commands.entity(entity).insert(PlayerPositioned);
    }
}

fn set_gameplay_camera(mut camera: Single<&mut Projection, With<MainCamera>>) {
    **camera = Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::FixedVertical {
            viewport_height: CAMERA_VIEW_HEIGHT,
        },
        ..OrthographicProjection::default_2d()
    });
}

fn follow_player_camera(
    players: Query<&Transform, With<Player>>,
    mut camera: Single<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    if let Ok(player) = players.single() {
        camera.translation.x = player.translation.x;
        camera.translation.y = player.translation.y;
    }
}

fn restore_camera(mut cameras: Query<(&mut Transform, &mut Projection), With<MainCamera>>) {
    for (mut transform, mut projection) in &mut cameras {
        transform.translation.x = 0.0;
        transform.translation.y = 0.0;
        *projection = Projection::Orthographic(OrthographicProjection::default_2d());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tiled_markers() {
        assert_eq!(
            parse_marker_name("Station:Cauldron"),
            Some(MarkerKind::Station(StationKind::Cauldron))
        );
        assert_eq!(
            parse_marker_name("Station:GrowBed"),
            Some(MarkerKind::Station(StationKind::GrowBed))
        );
        assert_eq!(parse_marker_name("QueueSpot:2"), Some(MarkerKind::Queue(2)));
        assert_eq!(parse_marker_name("PlayerSpawn"), Some(MarkerKind::Player));
        assert_eq!(parse_marker_name("CustomerDoor"), Some(MarkerKind::Door));
        assert_eq!(parse_marker_name("Station:NoSuchStation"), None);
    }
}
