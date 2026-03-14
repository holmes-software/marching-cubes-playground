mod marching_cubes;

use bevy::{
    asset::RenderAssetUsages, mesh::Indices, picking::mesh_picking::MeshPickingPlugin, prelude::*,
    render::render_resource::PrimitiveTopology,
};

use marching_cubes::marching_cubes as run_marching_cubes;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

const CELL_SIZE: f32 = 1.0;
const NODE_RADIUS: f32 = 0.08;
const CAMERA_ORBIT_SPEED: f32 = 1.8; // radians per second
const CAMERA_VERTICAL_ANGLE: f32 = 0.5; // radians above horizon
const MIN_RESOLUTION: usize = 1;
const MAX_RESOLUTION: usize = 8;

// ─────────────────────────────────────────────────────────────────────────────
// Resources
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Resource)]
struct VoxelGrid {
    /// Number of cells per axis (corners = resolution + 1 per axis).
    resolution: usize,
    /// Flat x-major density array. Length = (resolution+1)^3.
    densities: Vec<f32>,
    /// Whether the grid has changed and the mesh needs to be rebuilt.
    dirty: bool,
}

impl VoxelGrid {
    fn new(resolution: usize) -> Self {
        let side = resolution + 1;
        let len = side * side * side;
        Self {
            resolution,
            densities: vec![-1.0; len],
            dirty: false,
        }
    }

    fn stride(&self) -> usize {
        self.resolution + 1
    }

    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        let s = self.stride();
        x * s * s + y * s + z
    }

    fn toggle(&mut self, x: usize, y: usize, z: usize) {
        let i = self.index(x, y, z);
        self.densities[i] = if self.densities[i] < 0.0 { 1.0 } else { -1.0 };
        self.dirty = true;
    }

    fn value(&self, x: usize, y: usize, z: usize) -> f32 {
        self.densities[self.index(x, y, z)]
    }

    /// Appropriate orbit camera distance to see the whole grid.
    fn camera_distance(&self) -> f32 {
        self.resolution as f32 * CELL_SIZE * 2.4 + 2.5
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Components
// ─────────────────────────────────────────────────────────────────────────────

/// Marks a corner-node sphere and stores its grid index.
#[derive(Component)]
struct GridNode {
    xi: usize,
    yi: usize,
    zi: usize,
}

/// Marks the entity that holds the marching-cubes isosurface mesh.
#[derive(Component)]
struct IsosurfaceMesh;

/// Orbiting camera state.
#[derive(Component)]
struct OrbitCamera {
    angle: f32, // horizontal angle in radians
    distance: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Messages
// ─────────────────────────────────────────────────────────────────────────────

/// Fired when a node is clicked; carries its grid position.
#[derive(Message, Clone)]
struct NodeClicked {
    xi: usize,
    yi: usize,
    zi: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MeshPickingPlugin)
        .insert_resource(VoxelGrid::new(1))
        .add_message::<NodeClicked>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                camera_orbit,
                change_resolution,
                handle_node_clicked,
                rebuild_mesh.run_if(|grid: Res<VoxelGrid>| grid.dirty),
            ),
        )
        .run();
}

// ─────────────────────────────────────────────────────────────────────────────
// Setup
// ─────────────────────────────────────────────────────────────────────────────

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grid: Res<VoxelGrid>,
) {
    // Ambient light (Resource in Bevy 0.18)
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 400.0,
        ..default()
    });

    commands.insert_resource(ClearColor(Color::srgb(0.05, 0.07, 0.09)));

    // Camera — nodes are centered at origin
    let dist = grid.camera_distance();
    let center = Vec3::ZERO;
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(orbit_position(0.0, dist, center)).looking_at(center, Vec3::Y),
        OrbitCamera {
            angle: 0.0,
            distance: dist,
        },
    ));

    // Directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Corner node spheres
    let node_mesh = meshes.add(Sphere::new(NODE_RADIUS));
    spawn_nodes(&mut commands, &mut materials, &grid, node_mesh);

    // Isosurface mesh entity (starts empty)
    let iso_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.55, 0.72),
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let empty = meshes.add(empty_mesh());
    commands.spawn((
        Mesh3d(empty),
        MeshMaterial3d(iso_mat),
        Transform::default(),
        IsosurfaceMesh,
    ));
}

fn spawn_nodes(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    grid: &VoxelGrid,
    node_mesh: Handle<Mesh>,
) {
    let side = grid.resolution + 1;
    // Center offset so the grid is centered at origin
    let offset = grid.resolution as f32 * CELL_SIZE * 0.5;
    for xi in 0..side {
        for yi in 0..side {
            for zi in 0..side {
                let pos = Vec3::new(
                    xi as f32 * CELL_SIZE - offset,
                    yi as f32 * CELL_SIZE - offset,
                    zi as f32 * CELL_SIZE - offset,
                );
                let mat = materials.add(node_material(grid.value(xi, yi, zi)));
                commands
                    .spawn((
                        Mesh3d(node_mesh.clone()),
                        MeshMaterial3d(mat),
                        Transform::from_translation(pos),
                        GridNode { xi, yi, zi },
                    ))
                    .observe(
                        |click: On<Pointer<Click>>,
                         mut writer: MessageWriter<NodeClicked>,
                         nodes: Query<&GridNode>| {
                            if let Ok(node) = nodes.get(click.entity) {
                                writer.write(NodeClicked {
                                    xi: node.xi,
                                    yi: node.yi,
                                    zi: node.zi,
                                });
                            }
                        },
                    );
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Systems
// ─────────────────────────────────────────────────────────────────────────────

/// Handle NodeClicked messages: toggle density, update sphere color.
fn handle_node_clicked(
    mut reader: MessageReader<NodeClicked>,
    mut grid: ResMut<VoxelGrid>,
    nodes: Query<(&GridNode, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for msg in reader.read() {
        grid.toggle(msg.xi, msg.yi, msg.zi);
        let new_val = grid.value(msg.xi, msg.yi, msg.zi);
        for (node, mat_handle) in &nodes {
            if node.xi == msg.xi && node.yi == msg.yi && node.zi == msg.zi {
                if let Some(mat) = materials.get_mut(&mat_handle.0) {
                    mat.base_color = node_color(new_val);
                }
                break;
            }
        }
    }
}

/// Rebuild the isosurface mesh when the grid is dirty.
fn rebuild_mesh(
    mut grid: ResMut<VoxelGrid>,
    iso_query: Query<&Mesh3d, With<IsosurfaceMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    grid.dirty = false;

    // Center offset used when placing nodes
    let offset = grid.resolution as f32 * CELL_SIZE * 0.5;

    let raw_positions = run_marching_cubes(&grid.densities, grid.resolution as u8, CELL_SIZE);

    // Shift positions so they align with the centered node positions
    let positions: Vec<[f32; 3]> = raw_positions
        .iter()
        .map(|p| [p[0] - offset, p[1] - offset, p[2] - offset])
        .collect();

    let Ok(mesh_handle) = iso_query.single() else {
        return;
    };
    let Some(mesh) = meshes.get_mut(&mesh_handle.0) else {
        return;
    };

    *mesh = build_isosurface_mesh(positions);
}

/// Orbit the camera with left/right arrow keys.
fn camera_orbit(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_query: Query<(&mut Transform, &mut OrbitCamera)>,
) {
    let Ok((mut transform, mut orbit)) = camera_query.single_mut() else {
        return;
    };

    if keys.pressed(KeyCode::ArrowLeft) {
        orbit.angle -= CAMERA_ORBIT_SPEED * time.delta_secs();
    }
    if keys.pressed(KeyCode::ArrowRight) {
        orbit.angle += CAMERA_ORBIT_SPEED * time.delta_secs();
    }

    let pos = orbit_position(orbit.angle, orbit.distance, Vec3::ZERO);
    *transform = Transform::from_translation(pos).looking_at(Vec3::ZERO, Vec3::Y);
}

/// Increase/decrease grid resolution with up/down arrow keys.
#[allow(clippy::too_many_arguments)]
fn change_resolution(
    keys: Res<ButtonInput<KeyCode>>,
    mut grid: ResMut<VoxelGrid>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    node_entities: Query<Entity, With<GridNode>>,
    iso_query: Query<&Mesh3d, With<IsosurfaceMesh>>,
    mut camera_query: Query<(&mut Transform, &mut OrbitCamera)>,
) {
    let pressed_up = keys.just_pressed(KeyCode::ArrowUp);
    let pressed_down = keys.just_pressed(KeyCode::ArrowDown);

    if !pressed_up && !pressed_down {
        return;
    }

    let new_res = if pressed_up {
        (grid.resolution + 1).min(MAX_RESOLUTION)
    } else {
        grid.resolution.saturating_sub(1).max(MIN_RESOLUTION)
    };

    if new_res == grid.resolution {
        return;
    }

    // Reset the grid
    *grid = VoxelGrid::new(new_res);

    // Despawn all existing node spheres
    for entity in &node_entities {
        commands.entity(entity).despawn();
    }

    // Spawn new node spheres
    let node_mesh = meshes.add(Sphere::new(NODE_RADIUS));
    spawn_nodes(&mut commands, &mut materials, &grid, node_mesh);

    // Reset the isosurface mesh to empty
    if let Ok(mesh_handle) = iso_query.single()
        && let Some(mesh) = meshes.get_mut(&mesh_handle.0)
    {
        *mesh = empty_mesh();
    }

    // Adjust camera distance
    if let Ok((mut transform, mut orbit)) = camera_query.single_mut() {
        orbit.distance = grid.camera_distance();
        let pos = orbit_position(orbit.angle, orbit.distance, Vec3::ZERO);
        *transform = Transform::from_translation(pos).looking_at(Vec3::ZERO, Vec3::Y);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn node_color(value: f32) -> Color {
    if value >= 0.0 {
        Color::srgb(0.92, 0.92, 0.9)
    } else {
        Color::srgb(0.45, 0.12, 0.15)
    }
}

fn node_material(value: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: node_color(value),
        unlit: true,
        ..default()
    }
}

fn orbit_position(angle: f32, distance: f32, center: Vec3) -> Vec3 {
    let horiz = distance * CAMERA_VERTICAL_ANGLE.cos();
    let x = horiz * angle.cos();
    let y = distance * CAMERA_VERTICAL_ANGLE.sin();
    let z = horiz * angle.sin();
    center + Vec3::new(x, y, z)
}

fn empty_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new())
}

fn build_isosurface_mesh(positions: Vec<[f32; 3]>) -> Mesh {
    if positions.is_empty() {
        return empty_mesh();
    }

    let n = positions.len();

    // Compute flat normals per triangle (3 consecutive verts)
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(n);
    for tri in positions.chunks(3) {
        let normal = if tri.len() == 3 {
            let a = Vec3::from(tri[0]);
            let b = Vec3::from(tri[1]);
            let c = Vec3::from(tri[2]);
            (b - a).cross(c - a).normalize_or_zero().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };
        for _ in 0..tri.len() {
            normals.push(normal);
        }
    }

    let indices: Vec<u32> = (0..n as u32).collect();

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}
