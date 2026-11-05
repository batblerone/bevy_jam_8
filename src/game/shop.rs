//! The shop: one fixed room, described as an ASCII grid and spawned as entities.
//!
//! The loader is split in two halves so the Tiled map can
//! replace the ASCII grid later without touching the rest of the game:
//!
//! 1. **Data**: [`ShopLayout::parse`] turns [`LAYOUT_TEXT`] into a grid of
//!    [`Tile`]s. It is pure Rust (no ECS), so it is unit tested at the bottom
//!    of this file, and a typo in the layout is a clear panic at startup
//!    instead of a silently missing station.
//! 2. **Entities**: [`spawn_shop`] walks the grid and spawns floor, walls,
//!    stations and markers using the bundle functions [`wall`], [`station`]
//!    and friends. The Tiled loader should call those same functions, so
//!    nothing else in the game ever knows which loader built the room.
//!
//! Coordinates: the grid uses `(col, row)` with row 0 at the *top* of the text,
//! while world Y points *up*. [`ShopLayout::tile_to_world`] and
//! [`ShopLayout::world_to_tile`] are the only places that convert between the
//! two. The room is centred on the world origin.

use std::fmt;

use avian2d::prelude::*;
use bevy::{camera::ScalingMode, prelude::*, transform::TransformSystems};

use crate::{
    MainCamera,
    game::{physics::GameLayer, player::Player},
    game_manager::{RunSetup, Screen},
};

/// Size of one grid cell in world units.
pub const TILE_SIZE: f32 = 32.0;

/// Empty space (in tiles) kept around the room when fitting the camera.
/// Extra at the top so the timeline bar does not cover the back wall.
const CAMERA_MARGIN_TILES: Vec2 = Vec2::new(2.0, 3.0);

/// The shop floor plan. Every row must be the same width.
///
/// | Char | Meaning                          |
/// |------|----------------------------------|
/// | `#`  | wall                             |
/// | `.`  | floor                            |
/// | `P`  | witch spawn (floor)              |
/// | `D`  | customer door (floor)            |
/// | `Q`  | customer queue spot (floor)      |
/// | `B`  | ingredient crate                 |
/// | `F`  | flask shelf                      |
/// | `M`  | mortar                           |
/// | `R`  | drying rack                      |
/// | `C`  | cauldron                         |
/// | `K`  | counter                          |
/// | `S`  | customer service counter         |
/// | `T`  | trash                            |
/// | `G`  | grow bed                         |
/// | `Z`  | bed                              |
#[rustfmt::skip]
pub const LAYOUT_TEXT: [&str; 10] = [
    "####################",
    "#BBBF..M..RRR..GGG.#",
    "#..................#",
    "#K..C....C....K.TZ.#",
    "#........P.........#",
    "#KKKKKKKSKKKKKKKKKK#",
    "#.......Q..........#",
    "#.......Q..........#",
    "#.......Q..........#",
    "########D###########",
];

pub(super) fn plugin(app: &mut App) {
    // Parse once at startup. The layout is a constant, so an error here is a
    // programmer mistake and should stop the game with a readable message.
    let layout = ShopLayout::parse(&LAYOUT_TEXT)
        .unwrap_or_else(|err| panic!("shop LAYOUT_TEXT is invalid: {err}"));
    app.insert_resource(layout);

    app.add_systems(OnEnter(Screen::Gameplay), spawn_shop.after(RunSetup));
    app.add_systems(
        Update,
        (
            fit_camera_to_shop.run_if(resource_changed::<ShopLayout>),
            place_player_at_spawn.run_if(in_state(Screen::Gameplay)),
        ),
    );
    // After gameplay and physics have moved things, before transforms are
    // propagated to `GlobalTransform` for rendering.
    app.add_systems(PostUpdate, y_sort.before(TransformSystems::Propagate));
}

// The room is split into named z bands ([`z`]), and inside the `WORLD` band anything
// lower on screen draws in front of anything higher up. That is Y-sorting:
// it lets the witch walk "behind" a cauldron when she is above it.

/// Named z bands. Higher draws on top.
#[allow(dead_code)] // Not every band has users yet.
pub mod z {
    /// Floor tiles, rugs, garden soil.
    pub const FLOOR: f32 = 0.0;
    /// Spilled potions, scorch marks.
    pub const FLOOR_DECALS: f32 = 1.0;
    /// Flat placeholder walls. Below the world band so nothing hides behind them.
    pub const WALLS: f32 = 2.0;
    /// Stations, witch, customers, cats. Y-sorted within `WORLD..WORLD + 10`.
    pub const WORLD: f32 = 10.0;
    /// Hanging herbs, beams, steam and fire.
    pub const OVERHEAD: f32 = 30.0;
    /// Progress bars, highlights, floating text.
    pub const WORLD_UI: f32 = 40.0;
}

/// z change per world unit of height. The world band is 10 deep, so this keeps
/// anything within ±500 world units of the origin inside it (the room is ±160).
const Y_SORT_FACTOR: f32 = 0.01;

/// Put this on anything drawn in the room that should sort by its Y.
///
/// Uses the entity's *local* `Transform`, so it belongs on top-level entities
/// or on children of a parent sitting at the origin (like the shop root).
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct YSort;

pub fn y_sort(mut query: Query<&mut Transform, With<YSort>>) {
    for mut transform in &mut query {
        let target = z::WORLD + 5.0 - transform.translation.y * Y_SORT_FACTOR;
        // Only write when it changes, so change detection stays quiet for
        // things that are standing still.
        if transform.translation.z != target {
            transform.translation.z = target;
        }
    }
}

// ---------------------------------------------------------------------------
// Tiles and stations: "kind enum + data table"
// ---------------------------------------------------------------------------

/// What one character of the layout means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum Tile {
    Wall,
    Floor,
    Station(StationKind),
    PlayerSpawn,
    Queue,
    Door,
}

impl Tile {
    /// Map a layout character to a tile. `None` for characters the legend does
    /// not know, which [`ShopLayout::parse`] reports as an error.
    pub fn from_char(c: char) -> Option<Self> {
        Some(match c {
            '#' => Tile::Wall,
            '.' => Tile::Floor,
            'P' => Tile::PlayerSpawn,
            'Q' => Tile::Queue,
            'D' => Tile::Door,
            'B' => Tile::Station(StationKind::Crate),
            'F' => Tile::Station(StationKind::FlaskShelf),
            'M' => Tile::Station(StationKind::Mortar),
            'R' => Tile::Station(StationKind::DryingRack),
            'C' => Tile::Station(StationKind::Cauldron),
            'K' => Tile::Station(StationKind::Counter),
            'S' => Tile::Station(StationKind::ServiceCounter),
            'T' => Tile::Station(StationKind::Trash),
            'G' => Tile::Station(StationKind::GrowBed),
            'Z' => Tile::Station(StationKind::Bed),
            _ => return None,
        })
    }
}

/// Every kind of station the witch can use.
///
/// `Default` and `#[reflect(Default)]` are there so the Tiled custom-property
/// export can offer this enum as a dropdown later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Default)]
pub enum StationKind {
    #[default]
    Counter,
    Crate,
    FlaskShelf,
    Mortar,
    DryingRack,
    Cauldron,
    Bed,
    Trash,
    GrowBed,
    ServiceCounter,
}

/// Per-kind data for a station. Placeholder art for now: a coloured block.
pub struct StationSpec {
    /// Entity name shown in the inspector.
    pub name: &'static str,
    /// Short text drawn on the placeholder block so you can tell them apart.
    pub label: &'static str,
    pub color: Color,
    /// Footprint in tiles. Also the collider size. Counters fill their tile
    /// completely so a row of them forms a solid, seamless barrier.
    pub size: Vec2,
}

impl StationKind {
    /// Every kind, in declaration order. Asset loaders walk this list.
    #[allow(dead_code)] // For the station asset loader once art arrives.
    pub const ALL: [StationKind; 10] = [
        StationKind::Counter,
        StationKind::Crate,
        StationKind::FlaskShelf,
        StationKind::Mortar,
        StationKind::DryingRack,
        StationKind::Cauldron,
        StationKind::Bed,
        StationKind::Trash,
        StationKind::GrowBed,
        StationKind::ServiceCounter,
    ];

    pub fn spec(self) -> StationSpec {
        let (name, label, color, size) = match self {
            StationKind::Counter => ("Counter", "", srgb(0.55, 0.36, 0.22), Vec2::ONE),
            StationKind::ServiceCounter => (
                "Service Counter",
                "Serve",
                srgb(0.85, 0.65, 0.20),
                Vec2::ONE,
            ),
            StationKind::Crate => (
                "Ingredient Crate",
                "Crate",
                srgb(0.70, 0.50, 0.30),
                Vec2::splat(0.9),
            ),
            StationKind::FlaskShelf => (
                "Flask Shelf",
                "Flask",
                srgb(0.45, 0.75, 0.85),
                Vec2::splat(0.9),
            ),
            StationKind::Mortar => ("Mortar", "Mortar", srgb(0.60, 0.60, 0.65), Vec2::splat(0.7)),
            StationKind::DryingRack => (
                "Drying Rack",
                "Dry",
                srgb(0.65, 0.55, 0.30),
                Vec2::splat(0.9),
            ),
            StationKind::Cauldron => ("Cauldron", "Brew", srgb(0.25, 0.25, 0.30), Vec2::splat(0.9)),
            StationKind::Bed => ("Bed", "Bed", srgb(0.55, 0.35, 0.65), Vec2::ONE),
            StationKind::Trash => ("Trash", "Trash", srgb(0.35, 0.40, 0.35), Vec2::splat(0.7)),
            StationKind::GrowBed => ("Grow Bed", "Grow", srgb(0.30, 0.55, 0.25), Vec2::ONE),
        };
        StationSpec {
            name,
            label,
            color,
            size,
        }
    }
}

const fn srgb(r: f32, g: f32, b: f32) -> Color {
    Color::srgb(r, g, b)
}

// ---------------------------------------------------------------------------
// Components the loader puts on entities
// ---------------------------------------------------------------------------

/// Root entity of the spawned room. Everything else is its child.
#[derive(Component, Reflect, Clone, Default)]
#[reflect(Component)]
pub struct Shop;

/// Any station the witch can interact with.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct Station {
    pub kind: StationKind,
}

/// A solid wall segment (merged from several wall tiles).
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct Wall;

/// Where the witch appears at the start of a run.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct PlayerSpawn;

/// A spot where a customer waits. `index` 0 is right at the service counter,
/// higher numbers are further back in the line.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct QueueSpot {
    pub index: usize,
}

/// Where customers enter and leave.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct CustomerDoor;

// ---------------------------------------------------------------------------
// The parsed layout
// ---------------------------------------------------------------------------

/// Why a layout failed to parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    Empty,
    RaggedRow {
        row: usize,
        expected: usize,
        found: usize,
    },
    UnknownChar {
        row: usize,
        col: usize,
        ch: char,
    },
    PlayerSpawnCount(usize),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutError::Empty => write!(f, "the layout has no rows"),
            LayoutError::RaggedRow {
                row,
                expected,
                found,
            } => write!(
                f,
                "row {row} is {found} characters wide, but row 0 is {expected}"
            ),
            LayoutError::UnknownChar { row, col, ch } => {
                write!(f, "unknown character '{ch}' at row {row}, column {col}")
            }
            LayoutError::PlayerSpawnCount(n) => {
                write!(f, "expected exactly one 'P' (player spawn), found {n}")
            }
        }
    }
}

/// The shop as data: a grid of tiles plus helpers for converting positions.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ShopLayout {
    width: usize,
    height: usize,
    /// Row-major, row 0 is the top line of the text.
    tiles: Vec<Tile>,
}

impl ShopLayout {
    pub fn parse(rows: &[&str]) -> Result<Self, LayoutError> {
        let height = rows.len();
        let width = rows.first().ok_or(LayoutError::Empty)?.chars().count();
        if width == 0 {
            return Err(LayoutError::Empty);
        }

        let mut tiles = Vec::with_capacity(width * height);
        for (row, line) in rows.iter().enumerate() {
            let found = line.chars().count();
            if found != width {
                return Err(LayoutError::RaggedRow {
                    row,
                    expected: width,
                    found,
                });
            }
            for (col, ch) in line.chars().enumerate() {
                let tile = Tile::from_char(ch).ok_or(LayoutError::UnknownChar { row, col, ch })?;
                tiles.push(tile);
            }
        }

        let spawns = tiles.iter().filter(|t| **t == Tile::PlayerSpawn).count();
        if spawns != 1 {
            return Err(LayoutError::PlayerSpawnCount(spawns));
        }

        Ok(Self {
            width,
            height,
            tiles,
        })
    }

    /// The tile at `(col, row)`, or `None` outside the grid.
    pub fn get(&self, col: usize, row: usize) -> Option<Tile> {
        (col < self.width && row < self.height).then(|| self.tiles[row * self.width + col])
    }

    /// Every tile with its `(col, row)` grid position.
    pub fn iter(&self) -> impl Iterator<Item = (UVec2, Tile)> + '_ {
        self.tiles.iter().enumerate().map(|(i, tile)| {
            let pos = UVec2::new((i % self.width) as u32, (i / self.width) as u32);
            (pos, *tile)
        })
    }

    /// Size of the whole room in world units.
    pub fn world_size(&self) -> Vec2 {
        Vec2::new(self.width as f32, self.height as f32) * TILE_SIZE
    }

    /// World position of the *centre* of tile `(col, row)`.
    pub fn tile_to_world(&self, tile: UVec2) -> Vec2 {
        let half = Vec2::new(self.width as f32, self.height as f32) / 2.0;
        Vec2::new(
            tile.x as f32 + 0.5 - half.x,
            // Rows count down the screen, world Y counts up.
            half.y - tile.y as f32 - 0.5,
        ) * TILE_SIZE
    }

    /// The tile containing world position `pos`, or `None` outside the room.
    #[allow(dead_code)] // For focus / interaction.
    pub fn world_to_tile(&self, pos: Vec2) -> Option<UVec2> {
        let half = Vec2::new(self.width as f32, self.height as f32) / 2.0;
        let col = (pos.x / TILE_SIZE + half.x).floor();
        let row = (half.y - pos.y / TILE_SIZE).floor();
        let in_bounds =
            col >= 0.0 && row >= 0.0 && col < self.width as f32 && row < self.height as f32;
        in_bounds.then(|| UVec2::new(col as u32, row as u32))
    }

    /// World-space rectangle covering grid cells `min..max` (max exclusive).
    fn cells_to_world_rect(&self, cells: URect) -> Rect {
        let top_left = self.tile_to_world(cells.min) + Vec2::new(-0.5, 0.5) * TILE_SIZE;
        let size = (cells.max - cells.min).as_vec2() * TILE_SIZE;
        Rect::from_corners(top_left, top_left + Vec2::new(size.x, -size.y))
    }

    /// The single player spawn tile, in world space. `parse` guarantees it exists.
    pub fn player_spawn(&self) -> Vec2 {
        let (tile, _) = self
            .iter()
            .find(|(_, t)| *t == Tile::PlayerSpawn)
            .expect("parse guarantees one player spawn");
        self.tile_to_world(tile)
    }

    /// Queue tiles ordered front-to-back: the one nearest the service counter
    /// first. Falls back to top-to-bottom order if there is no service counter.
    pub fn queue_tiles(&self) -> Vec<UVec2> {
        let counter = self
            .iter()
            .find(|(_, t)| *t == Tile::Station(StationKind::ServiceCounter))
            .map(|(pos, _)| pos.as_vec2());
        let mut spots: Vec<UVec2> = self
            .iter()
            .filter(|(_, t)| *t == Tile::Queue)
            .map(|(pos, _)| pos)
            .collect();
        match counter {
            Some(c) => spots.sort_by(|a, b| {
                a.as_vec2()
                    .distance_squared(c)
                    .total_cmp(&b.as_vec2().distance_squared(c))
            }),
            None => spots.sort_by_key(|p| (p.y, p.x)),
        }
        spots
    }

    /// Wall tiles merged into as few rectangles as possible (grid cells,
    /// `max` exclusive). Horizontal runs are found first, then runs with the
    /// same span on consecutive rows are stacked. Fewer, longer colliders
    /// means smooth sliding with no snagging on seams between tiles.
    pub fn wall_rects(&self) -> Vec<URect> {
        let mut finished = Vec::new();
        // Rectangles that reached the previous row and may grow downwards.
        let mut open: Vec<URect> = Vec::new();

        for row in 0..self.height {
            let mut still_open = Vec::new();
            let mut col = 0;
            while col < self.width {
                if self.get(col, row) != Some(Tile::Wall) {
                    col += 1;
                    continue;
                }
                let start = col;
                while self.get(col, row) == Some(Tile::Wall) {
                    col += 1;
                }
                let (x0, x1, y) = (start as u32, col as u32, row as u32);
                let rect = match open.iter().position(|r| r.min.x == x0 && r.max.x == x1) {
                    Some(i) => {
                        let mut r = open.swap_remove(i);
                        r.max.y = y + 1;
                        r
                    }
                    None => URect::new(x0, y, x1, y + 1),
                };
                still_open.push(rect);
            }
            // Anything that did not continue onto this row is complete.
            finished.append(&mut open);
            open = still_open;
        }
        finished.append(&mut open);
        finished
    }
}

// ---------------------------------------------------------------------------
// Bundle functions: shared by this ASCII loader and the future Tiled loader
// ---------------------------------------------------------------------------

const WALL_COLOR: Color = Color::srgb(0.22, 0.18, 0.24);
const FLOOR_COLORS: [Color; 2] = [Color::srgb(0.47, 0.36, 0.27), Color::srgb(0.43, 0.33, 0.25)];
const LABEL_COLOR: Color = Color::srgb(0.97, 0.95, 0.85);

/// One floor tile. Purely visual, no collider.
pub fn floor(color: Color) -> impl Bundle {
    (
        Name::new("Floor"),
        Sprite::from_color(color, Vec2::splat(TILE_SIZE)),
    )
}

/// A solid wall rectangle of `size` world units.
pub fn wall(size: Vec2) -> impl Bundle {
    (
        Name::new("Wall"),
        Wall,
        Sprite::from_color(WALL_COLOR, size),
        RigidBody::Static,
        Collider::rectangle(size.x, size.y),
        GameLayer::wall_layers(),
    )
}

/// A station of `kind`. Position it with a `Transform` alongside this bundle.
pub fn station(kind: StationKind) -> impl Bundle {
    let spec = kind.spec();
    let size = spec.size * TILE_SIZE;
    (
        Name::new(spec.name),
        Station { kind },
        Sprite::from_color(spec.color, size),
        RigidBody::Static,
        Collider::rectangle(size.x, size.y),
        GameLayer::station_layers(),
        YSort,
        children![(
            Name::new("Label"),
            Text2d::new(spec.label),
            TextFont::from_font_size(9.0),
            TextColor(LABEL_COLOR),
            // Just above the block it labels.
            Transform::from_xyz(0.0, 0.0, 0.01),
        )],
    )
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawns the whole room from [`ShopLayout`]. Runs on entering gameplay, after
/// [`RunSetup`], and is cleaned up by `DespawnOnExit` on the root.
pub fn spawn_shop(mut commands: Commands, layout: Res<ShopLayout>) {
    let root = commands
        .spawn((
            Name::new("Shop"),
            Shop,
            Transform::default(),
            Visibility::default(),
            DespawnOnExit(Screen::Gameplay),
        ))
        .id();

    // Floor under everything that is not a wall (stations stand on floor too).
    for (tile, kind) in layout.iter() {
        if kind == Tile::Wall {
            continue;
        }
        let color = FLOOR_COLORS[((tile.x + tile.y) % 2) as usize];
        let pos = layout.tile_to_world(tile).extend(z::FLOOR);
        commands.spawn((
            floor(color),
            Transform::from_translation(pos),
            ChildOf(root),
        ));
    }

    for cells in layout.wall_rects() {
        let rect = layout.cells_to_world_rect(cells);
        commands.spawn((
            wall(rect.size()),
            Transform::from_translation(rect.center().extend(z::WALLS)),
            ChildOf(root),
        ));
    }

    for (tile, kind) in layout.iter() {
        // Y-sorting overwrites z for stations, so the starting z only matters
        // for the first frame.
        let pos = layout.tile_to_world(tile).extend(z::WORLD);
        match kind {
            Tile::Station(kind) => {
                commands.spawn((
                    station(kind),
                    Transform::from_translation(pos),
                    ChildOf(root),
                ));
            }
            Tile::PlayerSpawn => {
                commands.spawn((
                    Name::new("Player Spawn"),
                    PlayerSpawn,
                    Transform::from_translation(pos),
                    ChildOf(root),
                ));
            }
            Tile::Door => {
                commands.spawn((
                    Name::new("Customer Door"),
                    CustomerDoor,
                    Transform::from_translation(pos),
                    ChildOf(root),
                ));
            }
            Tile::Wall | Tile::Floor | Tile::Queue => {}
        }
    }

    for (index, tile) in layout.queue_tiles().into_iter().enumerate() {
        let pos = layout.tile_to_world(tile).extend(z::WORLD);
        commands.spawn((
            Name::new(format!("Queue Spot {index}")),
            QueueSpot { index },
            Transform::from_translation(pos),
            ChildOf(root),
        ));
    }
}

/// Moves a newly spawned witch onto the `P` tile.
///
/// The player is spawned as a scene in `level.rs`, so instead of threading the
/// position through the scene we move her the first frame she exists.
fn place_player_at_spawn(
    layout: Res<ShopLayout>,
    mut players: Query<&mut Transform, Added<Player>>,
) {
    let spawn = layout.player_spawn();
    for mut transform in &mut players {
        transform.translation.x = spawn.x;
        transform.translation.y = spawn.y;
    }
}

/// Zooms the main camera so the whole room (plus a margin) is always visible,
/// whatever the window size. Re-runs if the layout resource is ever replaced.
fn fit_camera_to_shop(
    layout: Res<ShopLayout>,
    mut projection: Single<&mut Projection, With<MainCamera>>,
) {
    let view = layout.world_size() + CAMERA_MARGIN_TILES * TILE_SIZE;
    **projection = Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::AutoMin {
            min_width: view.x,
            min_height: view.y,
        },
        ..OrthographicProjection::default_2d()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> ShopLayout {
        ShopLayout::parse(&LAYOUT_TEXT).expect("LAYOUT_TEXT parses")
    }

    #[test]
    fn shipped_layout_parses() {
        let layout = layout();
        assert_eq!((layout.width, layout.height), (20, 10));
    }

    #[test]
    fn every_station_kind_appears_in_layout() {
        let layout = layout();
        for kind in StationKind::ALL {
            assert!(
                layout.iter().any(|(_, t)| t == Tile::Station(kind)),
                "{kind:?} is missing from LAYOUT_TEXT"
            );
        }
    }

    #[test]
    fn rejects_bad_layouts() {
        assert_eq!(ShopLayout::parse(&[]), Err(LayoutError::Empty));
        assert_eq!(
            ShopLayout::parse(&["#P#", "##"]),
            Err(LayoutError::RaggedRow {
                row: 1,
                expected: 3,
                found: 2
            })
        );
        assert_eq!(
            ShopLayout::parse(&["#P?"]),
            Err(LayoutError::UnknownChar {
                row: 0,
                col: 2,
                ch: '?'
            })
        );
        assert_eq!(
            ShopLayout::parse(&["#..#"]),
            Err(LayoutError::PlayerSpawnCount(0))
        );
        assert_eq!(
            ShopLayout::parse(&["#PP#"]),
            Err(LayoutError::PlayerSpawnCount(2))
        );
    }

    #[test]
    fn tile_world_round_trip() {
        let layout = layout();
        // Top-left tile centre is up and to the left of the origin.
        assert_eq!(layout.tile_to_world(UVec2::ZERO), Vec2::new(-304.0, 144.0));
        for (tile, _) in layout.iter() {
            let world = layout.tile_to_world(tile);
            assert_eq!(layout.world_to_tile(world), Some(tile));
            // Any point inside the tile maps back to it too.
            assert_eq!(layout.world_to_tile(world + Vec2::splat(15.0)), Some(tile));
        }
        assert_eq!(layout.world_to_tile(Vec2::new(10_000.0, 0.0)), None);
    }

    #[test]
    fn walls_merge_into_few_rects_covering_every_wall_tile() {
        let layout = layout();
        let rects = layout.wall_rects();
        // Top wall, left wall, right wall, bottom wall split by the door.
        assert_eq!(rects.len(), 5, "{rects:?}");

        let wall_tiles = layout.iter().filter(|(_, t)| *t == Tile::Wall).count() as u32;
        let covered: u32 = rects.iter().map(|r| r.width() * r.height()).sum();
        assert_eq!(covered, wall_tiles);
    }

    #[test]
    fn queue_starts_at_service_counter() {
        let layout = layout();
        let queue = layout.queue_tiles();
        assert_eq!(
            queue,
            vec![UVec2::new(8, 6), UVec2::new(8, 7), UVec2::new(8, 8)]
        );
    }
}

#[test]
fn y_sort_draws_lower_in_front_and_stays_in_world_band() {
    use bevy::ecs::system::RunSystemOnce;

    let mut world = World::new();
    let high = world
        .spawn((YSort, Transform::from_xyz(0.0, 150.0, 0.0)))
        .id();
    let low = world
        .spawn((YSort, Transform::from_xyz(0.0, -150.0, 0.0)))
        .id();
    world.run_system_once(y_sort).unwrap();

    let z_of = |e| world.get::<Transform>(e).unwrap().translation.z;
    assert!(z_of(low) > z_of(high), "lower entity must draw on top");
    for e in [high, low] {
        assert!(
            (z::WORLD..z::OVERHEAD).contains(&z_of(e)),
            "z {} out of band",
            z_of(e)
        );
    }
}

#[test]
fn player_always_gets_y_sorted() {
    let mut world = World::new();
    let player = world.spawn(Player).id();
    assert!(world.get::<YSort>(player).is_some());
}
