use std::collections::HashMap;

use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    graphics::create_polygon_mesh,
    map::{
        CHUNK_HALF_SIZE, CHUNK_SIZE, CHUNK_SIZE_F32, CHUNK_SIZE_I32, ChunkEntity, FIELD_SIZE, Map,
    },
    player_camera::{PlayerCamera, PlayerCameraPlugin},
    toasts::{ToastMessage, ToastsPlugin},
    user_controls::UserControlsPlugin,
};

mod graphics;
mod map;
mod module_loader;
mod player_camera;
mod toasts;
mod user_controls;

/// Trait for building construction logic.
trait BuildingBuilder: Send + Sync + 'static {
    fn build(&self, entry: &BuildingEntry, commands: &mut Commands, position: IVec2);
}

impl<F> BuildingBuilder for F
where
    F: Fn(&BuildingEntry, &mut Commands, IVec2) + Send + Sync + 'static,
{
    fn build(&self, entry: &BuildingEntry, commands: &mut Commands, position: IVec2) {
        (self)(entry, commands, position)
    }
}

struct BuildingEntry {
    occlusion_map: Vec<IVec2>,
    build_cursor_offset: Vec2,
    mesh_handle: Handle<Mesh>,
    material_handle: Handle<ColorMaterial>,
    description: Option<String>,
    builder: Box<dyn BuildingBuilder>,
}

impl std::fmt::Debug for BuildingEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuildingEntry")
            .field("occlusion_map", &self.occlusion_map)
            .field("build_cursor_offset", &self.build_cursor_offset)
            .field("mesh_handle", &self.mesh_handle)
            .field("material_handle", &self.material_handle)
            .field("description", &self.description)
            .finish()
    }
}

#[derive(Resource, Default)]
struct BuildingRegistry {
    buildings: HashMap<String, BuildingEntry>,
}

impl BuildingRegistry {
    fn register(&mut self, id: impl Into<String>, entry: BuildingEntry) {
        let id = id.into();
        info!("Registering building: {} -> {:?}", id, entry);
        self.buildings.insert(id, entry);
    }
}

#[derive(Component)]
struct Barracks;
const BARRACKS_ID: &str = "core:barracks";

fn setup_buildings(
    mut registry: ResMut<BuildingRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let barracks_mesh = create_polygon_mesh(5, FIELD_SIZE);
    let barracks_mesh_handle = meshes.add(barracks_mesh);
    let barracks_material = ColorMaterial::from_color(Color::srgb(0.6, 0.2, 0.2));
    let barracks_material_handle = materials.add(barracks_material);
    let barracks_builder = Box::new(
        |entry: &BuildingEntry, commands: &mut Commands, position: IVec2| {
            commands.spawn((
                Barracks,
                Transform::from_translation(Vec3::new(
                    position.x as f32 * FIELD_SIZE + FIELD_SIZE / 2.0,
                    position.y as f32 * FIELD_SIZE + FIELD_SIZE / 2.0,
                    0.0,
                )),
                GlobalTransform::default(),
                Mesh2d(entry.mesh_handle.clone()),
                MeshMaterial2d(entry.material_handle.clone()),
            ));
        },
    );
    let barracks_entry = BuildingEntry {
        occlusion_map: vec![
            IVec2::new(0, 0),
            IVec2::new(1, 0),
            IVec2::new(0, 1),
            IVec2::new(1, 1),
        ],
        // offset in top left direction to center the build cursor
        build_cursor_offset: Vec2::splat(-FIELD_SIZE / 2.0),
        mesh_handle: barracks_mesh_handle,
        material_handle: barracks_material_handle,
        description: Some("Used to train infantry units.".to_string()),
        builder: barracks_builder,
    };
    registry.register(BARRACKS_ID, barracks_entry);
}

fn setup_map(
    mut commands: Commands,
    mut map: ResMut<Map>,
    mut toasts: MessageWriter<ToastMessage>,
) {
    for x in -2..2 {
        for y in -2..2 {
            map.create_chunk(IVec2::new(x, y), &mut commands);
            let success = map.try_place(
                IVec2::new(x * CHUNK_SIZE_I32 + 2, y * CHUNK_SIZE_I32 + 2),
                &[IVec2::new(0, 0)],
            );
            assert!(success, "Placement should succeed here");
            toasts.write(ToastMessage {
                content: format!("Loaded chunk at {}", IVec2::new(x, y)),
            });
        }
    }

    commands.spawn(());
}

#[derive(Resource, Default)]
struct CursorBuilding {
    building_id: Option<String>,
}

struct MouseCursorPosition {
    /// Exact world position of the cursor.
    world_position: Vec2,
    /// Position snapped to grid coordinates as `(chunk_position, local_position)`.
    grid_position: (IVec2, IVec2),
}

#[derive(Resource, Default)]
struct MouseCursor {
    position: Option<MouseCursorPosition>,
}

impl MouseCursor {
    #[inline]
    fn world_position(&self) -> Option<Vec2> {
        self.position.as_ref().map(|v| v.world_position)
    }

    #[inline]
    fn grid_position(&self) -> Option<(IVec2, IVec2)> {
        self.position.as_ref().map(|v| v.grid_position)
    }
}

fn update_cursor_position(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<PlayerCamera>>,
    mut cursor_position: ResMut<MouseCursor>,
) {
    let window = window.into_inner();
    let (camera, camera_transform) = camera_query.into_inner();
    if let Some(screen_pos) = window.cursor_position()
        && let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, screen_pos)
    {
        let grid_x = (world_pos.x / FIELD_SIZE).floor() as i32;
        let grid_y = (world_pos.y / FIELD_SIZE).floor() as i32;
        let grid_pos = IVec2::new(grid_x, grid_y);
        let (chunk_pos, local_pos) = Map::global_to_chunk(grid_pos);
        cursor_position.position = Some(MouseCursorPosition {
            world_position: world_pos,
            grid_position: (chunk_pos, local_pos),
        });
    } else {
        cursor_position.position = None;
        return;
    }
}

fn player_controls(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut map: ResMut<Map>,
    registry: Res<BuildingRegistry>,
    mut toasts: MessageWriter<ToastMessage>,
    cursor: Res<MouseCursor>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyB) {
        if let Some((chunk_pos, local_pos)) = cursor.grid_position()
            && let Some(entry) = registry.buildings.get(BARRACKS_ID)
        {
            let global_pos = Map::chunk_to_global(chunk_pos, local_pos);
            let success = map.try_place(global_pos, &entry.occlusion_map);
            if success {
                toasts.write(ToastMessage {
                    content: format!("Placed Barracks at {}", global_pos),
                });
            } else {
                toasts.write(ToastMessage {
                    content: format!(
                        "Failed to place Barracks at {}: Space occupied or chunk not loaded",
                        global_pos
                    ),
                });
            }
        }
    }
}

fn debug_chunk_bounds(mut gizmos: Gizmos, query: Query<&ChunkEntity>) {
    for chunk in query {
        let chunk_world_pos =
            chunk.position().as_vec2() * CHUNK_SIZE_F32 * FIELD_SIZE + CHUNK_HALF_SIZE;
        gizmos
            .grid_2d(
                Isometry2d::from_translation(chunk_world_pos),
                UVec2::splat(1),
                Vec2::splat(CHUNK_SIZE_F32 * FIELD_SIZE),
                Color::srgba(0.0, 0.7, 0.0, 0.5),
            )
            .outer_edges();
    }
}

fn debug_chunk_fields(
    mut gizmos: Gizmos,
    query: Query<&ChunkEntity>,
    map: Res<Map>,
    cursor: Res<MouseCursor>,
) {
    let color_occupied = Color::srgba(0.7, 0.0, 0.0, 0.4);
    let color_free = Color::srgba(0.0, 0.7, 0.0, 0.2);
    let color_hover_occupied = Color::srgba(1.0, 0.3, 0.0, 0.6);
    let color_hover_free = Color::srgba(0.0, 0.3, 1.0, 0.6);
    for chunk in query {
        let chunk_world_pos =
            chunk.position().as_vec2() * CHUNK_SIZE_F32 * FIELD_SIZE + CHUNK_HALF_SIZE;
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let field_pos =
                    chunk_world_pos + Vec2::new(x as f32 * FIELD_SIZE, y as f32 * FIELD_SIZE);
                let color = if map.is_occupied(chunk.position(), IVec2::new(x as i32, y as i32)) {
                    if let Some((cursor_chunk_pos, cursor_local_pos)) = cursor.grid_position()
                        && cursor_chunk_pos == chunk.position()
                        && cursor_local_pos == IVec2::new(x as i32, y as i32)
                    {
                        color_hover_occupied
                    } else {
                        color_occupied
                    }
                } else {
                    if let Some((cursor_chunk_pos, cursor_local_pos)) = cursor.grid_position()
                        && cursor_chunk_pos == chunk.position()
                        && cursor_local_pos == IVec2::new(x as i32, y as i32)
                    {
                        color_hover_free
                    } else {
                        color_free
                    }
                };
                gizmos.circle_2d(
                    Isometry2d::from_translation(
                        field_pos + Vec2::splat(FIELD_SIZE / 2.0) - CHUNK_HALF_SIZE,
                    ),
                    FIELD_SIZE / 4.0,
                    color,
                );
            }
        }
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    Game,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PlayerCameraPlugin,
            ToastsPlugin,
            UserControlsPlugin,
        ))
        .init_resource::<Map>()
        .init_resource::<BuildingRegistry>()
        .init_resource::<CursorBuilding>()
        .init_resource::<MouseCursor>()
        .init_state::<AppState>()
        .add_systems(Startup, (setup_map, setup_buildings))
        .add_systems(
            Update,
            (
                debug_chunk_bounds,
                debug_chunk_fields,
                player_controls,
                update_cursor_position,
            ),
        )
        .run();
}
