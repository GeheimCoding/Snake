use bevy::prelude::*;

#[derive(Resource)]
struct Size(f32);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("Snake"),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Size(50.0))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
struct Player;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    size: Res<Size>,
) {
    commands.spawn(Camera2d);
    commands.spawn((
        Player,
        Mesh2d(meshes.add(Rectangle::from_size(Vec2::splat(size.0)))),
        MeshMaterial2d(color_materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
}
