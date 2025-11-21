use bevy::{input::mouse::MouseWheel, platform::collections::HashMap, prelude::*};

const FIELD_SIZE: f32 = 4.0;
const CHUNK_SIZE: usize = 16;
const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;
const CHUNK_SIZE_F32: f32 = CHUNK_SIZE as f32;
const CHUNK_HALF_SIZE: Vec2 = Vec2::splat(CHUNK_SIZE_F32 * FIELD_SIZE / 2.0);

#[derive(Component, Debug, Clone, Copy)]
struct ChunkEntity {
    position: IVec2,
}

struct ChunkData {
    tiles: [[Option<Entity>; CHUNK_SIZE]; CHUNK_SIZE],
}

impl ChunkData {
    fn new() -> Self {
        Self {
            tiles: [[None; CHUNK_SIZE]; CHUNK_SIZE],
        }
    }

    fn set(&mut self, local_pos: IVec2, entity: Entity) {
        self.tiles[local_pos.x as usize][local_pos.y as usize] = Some(entity);
    }
}

#[derive(Default, Resource)]
struct Map {
    chunks: HashMap<IVec2, ChunkData>,
}

impl Map {
    /// Converts global position to chunk position and local position within that chunk.
    ///
    /// # Arguments
    /// * `pos`: The global position as an IVec2.
    ///
    /// # Returns
    /// - `(chunk_pos, local_pos)`: A tuple where `chunk_pos` is the position of the chunk
    ///   containing the global position, and `local_pos` is the position within that chunk.
    const fn global_to_chunk(pos: IVec2) -> (IVec2, IVec2) {
        let chunk_pos = IVec2::new(
            pos.x.div_euclid(CHUNK_SIZE_I32),
            pos.y.div_euclid(CHUNK_SIZE_I32),
        );
        let local_pos = IVec2::new(
            pos.x.rem_euclid(CHUNK_SIZE_I32),
            pos.y.rem_euclid(CHUNK_SIZE_I32),
        );
        (chunk_pos, local_pos)
    }

    fn create_chunk(&mut self, pos: IVec2, commands: &mut Commands) {
        if self.chunks.insert(pos, ChunkData::new()).is_some() {
            // for now, we just panic if chunk exists
            panic!("Chunk at position {:?} already exists!", pos);
        }
        commands.spawn(ChunkEntity { position: pos });
    }

    fn try_place(&self, pos: IVec2, occlusion_map: Vec<IVec2>) -> bool {
        for offset in occlusion_map {
            let check_pos = pos + offset;
            let chunk_pos = IVec2::new(
                check_pos.x.div_euclid(CHUNK_SIZE_I32),
                check_pos.y.div_euclid(CHUNK_SIZE_I32),
            );
            let local_pos = IVec2::new(
                check_pos.x.rem_euclid(CHUNK_SIZE_I32),
                check_pos.y.rem_euclid(CHUNK_SIZE_I32),
            );
            if let Some(chunk) = self.chunks.get(&chunk_pos) {
                if chunk.tiles[local_pos.x as usize][local_pos.y as usize].is_some() {
                    return false;
                }
            } else {
                // Chunk does not exist -> not loaded yet -> placement fails
                // TODO: handle error and chunk loading properly
                return false;
            }
        }

        todo!();
    }
}

struct Building {
    occlusion_map: Vec<IVec2>,
    build_cursor_offset: Vec2,
}

#[derive(Resource, Default)]
struct BuildingRegistry {
    buildings: HashMap<String, Building>,
}

fn setup_buildings(mut registry: ResMut<BuildingRegistry>) {
    registry.buildings.insert(
        "core:barracks".to_string(),
        Building {
            occlusion_map: vec![
                IVec2::new(0, 0),
                IVec2::new(1, 0),
                IVec2::new(0, 1),
                IVec2::new(1, 1),
            ],
            // offset in top left direction to center the build cursor
            build_cursor_offset: Vec2::splat(-FIELD_SIZE / 2.0),
        },
    );
}

#[derive(Component)]
struct PlayerCamera {
    target_position: Vec2,
    target_scale: f32,
}

impl Default for PlayerCamera {
    fn default() -> Self {
        Self {
            target_position: Vec2::ZERO,
            target_scale: Self::INITIAL_SCALE,
        }
    }
}

impl PlayerCamera {
    const SPEED: f32 = 500.0;
    const POSITION_INTERPOLATION_FACTOR: f32 = 0.3;
    const INITIAL_SCALE: f32 = 1.0;
    const MIN_SCALE: f32 = 0.05;
    const MAX_SCALE: f32 = 1.0;
    const SCALE_STEP: f32 = 0.05;
    const SCALE_INTERPOLATION_FACTOR: f32 = 0.1;
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        PlayerCamera::default(),
        Camera2d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, PlayerCamera::INITIAL_SCALE)),
        GlobalTransform::default(),
    ));
}

fn camera_controls(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut scroll_events: MessageReader<MouseWheel>,
    camera_query: Single<(&mut Transform, &mut PlayerCamera)>,
    time: Res<Time>,
) {
    let (mut transform, mut player_camera) = camera_query.into_inner();
    let delta_secs = time.delta_secs();

    // --- Zoom Controls ---
    let mut scale_delta = 0.0;
    for event in scroll_events.read() {
        scale_delta -= event.y;
    }

    if scale_delta != 0.0 {
        let new_target_scale = player_camera.target_scale + scale_delta * PlayerCamera::SCALE_STEP;
        player_camera.target_scale =
            new_target_scale.clamp(PlayerCamera::MIN_SCALE, PlayerCamera::MAX_SCALE);
    }

    // Smoothly interpolate camera zoom towards target zoom
    let current_zoom = transform.scale;
    let new_zoom = current_zoom.lerp(
        Vec3::splat(player_camera.target_scale),
        PlayerCamera::SCALE_INTERPOLATION_FACTOR,
    );
    transform.scale = new_zoom;

    // --- Movement Controls ---
    let mut direction = Vec2::ZERO;

    if keyboard_input.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    if direction != Vec2::ZERO {
        let movement = direction.normalize() * PlayerCamera::SPEED * delta_secs;
        // Adjust movement speed based on zoom level
        let movement = movement * player_camera.target_scale;
        player_camera.target_position += movement;
    }

    // Smoothly interpolate camera position towards target position
    let current_position = Vec2::new(transform.translation.x, transform.translation.y);
    let new_position = current_position.lerp(
        player_camera.target_position,
        PlayerCamera::POSITION_INTERPOLATION_FACTOR,
    );
    transform.translation.x = new_position.x;
    transform.translation.y = new_position.y;
}

fn setup_map(mut commands: Commands, mut map: ResMut<Map>) {
    for x in -2..2 {
        for y in -2..2 {
            map.create_chunk(IVec2::new(x, y), &mut commands);
        }
    }
}

fn debug_chunk_bounds(mut gizmos: Gizmos, query: Query<&ChunkEntity>) {
    for chunk in query {
        let chunk_world_pos = chunk.position.as_vec2() * CHUNK_SIZE_F32 * FIELD_SIZE;
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

fn debug_chunk_fields(mut gizmos: Gizmos, query: Query<&ChunkEntity>) {
    for chunk in query {
        let chunk_world_pos = chunk.position.as_vec2() * CHUNK_SIZE_F32 * FIELD_SIZE;
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let field_pos =
                    chunk_world_pos + Vec2::new(x as f32 * FIELD_SIZE, y as f32 * FIELD_SIZE);
                gizmos.circle_2d(
                    Isometry2d::from_translation(
                        field_pos + Vec2::splat(FIELD_SIZE / 2.0) - CHUNK_HALF_SIZE,
                    ),
                    FIELD_SIZE / 4.0,
                    Color::srgba(0.0, 0.3, 0.7, 0.2),
                );
            }
        }
    }
}

fn main() {
    App::new()
        .init_resource::<Map>()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_camera, setup_map))
        .add_systems(
            Update,
            (debug_chunk_bounds, debug_chunk_fields, camera_controls),
        )
        .run();
}
