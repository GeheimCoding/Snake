use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;
use bincode::{Decode, Encode, config};
use rand::prelude::{IndexedRandom, SliceRandom};
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::path::Path;
use std::time::Duration;
use std::{fs, io};

#[derive(Resource)]
struct Constants {
    size: f32,
    mesh_handle: Handle<Mesh>,
    color_handle: Handle<ColorMaterial>,
    head_texture_handle: Handle<Image>,
    apple_texture_handle: Handle<Image>,
}

#[derive(Resource)]
struct AppleCrunch {
    handles: Vec<Handle<AudioSource>>,
}

#[derive(Event)]
struct MovementEvent;

#[derive(Event)]
struct AppleEatenEvent(Entity);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("Snake"),
                ..default()
            }),
            ..default()
        }))
        .add_event::<MovementEvent>()
        .add_event::<AppleEatenEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                trigger_movement,
                change_direction,
                (grow, update_score, play_crunch_sound).run_if(on_event::<AppleEatenEvent>),
                (
                    move_head,
                    adjust_head_direction,
                    eat_apple,
                    remove_tail.run_if(not(on_event::<AppleEatenEvent>)),
                )
                    .chain()
                    .run_if(on_event::<MovementEvent>),
            ),
        )
        .run();
}

#[derive(Component, PartialEq)]
struct Head;

#[derive(Component)]
struct Body;

impl PartialEq<Head> for Body {
    fn eq(&self, _: &Head) -> bool {
        false
    }
}

#[derive(Component)]
struct Tail;

impl PartialEq<Head> for Tail {
    fn eq(&self, _: &Head) -> bool {
        false
    }
}

#[derive(Component)]
struct BodyPart;

#[derive(Component)]
struct NextBodyPart(Option<Entity>);

#[derive(Component)]
struct Apple;

#[derive(Component)]
struct Score(u32);

#[derive(Component, Encode, Decode)]
struct HighScore(u32);

#[derive(Component, Default, Clone)]
enum Direction {
    Up,
    Down,
    Left,
    #[default]
    Right,
}

#[derive(Component, Clone)]
struct LastDirection(Direction);

#[derive(Component)]
struct MovementTimer(Timer);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    window: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    let size = 50.0;
    let speed = Duration::from_millis(100);
    let color = Color::srgb(0.3, 0.5, 0.3);
    let color_handle = color_materials.add(color);
    let mesh_handle = meshes.add(Rectangle::from_size(Vec2::splat(size)));

    let constants = Constants {
        size,
        mesh_handle: mesh_handle.clone(),
        color_handle: color_handle.clone(),
        head_texture_handle: asset_server.load("textures/head.png"),
        apple_texture_handle: asset_server.load("textures/apple.png"),
    };

    commands.spawn(MovementTimer(Timer::new(speed, TimerMode::Repeating)));
    commands.spawn((Direction::default(), LastDirection(Direction::default())));

    let head_position = Vec2::default();
    let head = spawn_part(
        &mut commands,
        Head,
        head_position,
        &constants,
        NextBodyPart(None),
    );
    let body_position = Vec2::new(-size, 0.0);
    let body = spawn_part(
        &mut commands,
        Body,
        body_position,
        &constants,
        NextBodyPart(Some(head)),
    );
    let tail_position = Vec2::new(-2.0 * size, 0.0);
    spawn_part(
        &mut commands,
        Tail,
        tail_position,
        &constants,
        NextBodyPart(Some(body)),
    );

    spawn_apple(
        &mut commands,
        size,
        constants.apple_texture_handle.clone(),
        vec![head_position, body_position, tail_position],
    );

    let resolution = &window.single().resolution;
    commands.spawn((
        Score(0),
        Text2d::new("Score: 0"),
        TextFont {
            font: asset_server.load("fonts/upheavtt.ttf"),
            font_size: 50.0,
            ..default()
        },
        Anchor::TopLeft,
        Transform::from_translation(Vec3::new(
            resolution.width() / -2.0 + 20.0,
            resolution.height() / 2.0,
            0.0,
        )),
    ));

    let high_score = load_high_score().expect("could not read high score");
    commands.spawn((
        Text2d::new(format!("High Score: {}", high_score.0)),
        high_score,
        TextFont {
            font: asset_server.load("fonts/upheavtt.ttf"),
            font_size: 50.0,
            ..default()
        },
        Anchor::TopRight,
        Transform::from_translation(Vec3::new(
            resolution.width() / 2.0 - 20.0,
            resolution.height() / 2.0,
            0.0,
        )),
    ));

    commands.spawn(Camera2d);
    commands.insert_resource(constants);

    let handles = (1..=4)
        .map(|i| format!("sounds/apple-crunch-{i}.wav"))
        .map(|name| asset_server.load(name))
        .collect();
    commands.insert_resource(AppleCrunch { handles });
}

fn load_high_score() -> io::Result<HighScore> {
    let file = File::open("assets/saves/high_score");
    if let Err(err) = file {
        match err.kind() {
            ErrorKind::NotFound => Ok(HighScore(0)),
            _ => Err(err),
        }
    } else {
        let mut content = vec![];
        file?.read_to_end(&mut content)?;
        Ok(bincode::decode_from_slice(&content, config::standard())
            .expect("failed to decode high score")
            .0)
    }
}

fn save_high_score(high_score: &HighScore) -> io::Result<()> {
    let path = Path::new("assets/saves");
    fs::create_dir_all(path)?;
    let mut file = File::create(path.join("high_score"))?;

    let encoded = bincode::encode_to_vec(high_score, config::standard())
        .expect("failed to encode high score");
    file.write_all(&encoded)?;

    Ok(())
}

fn trigger_movement(
    mut query: Query<&mut MovementTimer>,
    mut movement_event: EventWriter<MovementEvent>,
    time: Res<Time>,
) {
    if query.single_mut().0.tick(time.delta()).just_finished() {
        movement_event.send(MovementEvent);
    }
}

fn move_head(
    mut commands: Commands,
    mut query: Query<(&mut LastDirection, &Direction)>,
    mut head_query: Query<(Entity, &mut Transform), With<Head>>,
    constants: Res<Constants>,
) {
    let size = constants.size;
    let (mut last_direction, direction) = query.single_mut();
    let (head, transform) = head_query.single_mut();
    let offset = Vec2::from(match direction {
        Direction::Up => (0.0, size),
        Direction::Down => (0.0, -size),
        Direction::Left => (-size, 0.0),
        Direction::Right => (size, 0.0),
    });

    let new_head = spawn_part(
        &mut commands,
        Head,
        transform.translation.truncate() + offset,
        &constants,
        NextBodyPart(None),
    );
    commands
        .entity(head)
        .remove::<Head>()
        .remove::<Sprite>()
        .insert((
            Body,
            NextBodyPart(Some(new_head)),
            Mesh2d(constants.mesh_handle.clone()),
            MeshMaterial2d(constants.color_handle.clone()),
        ));
    last_direction.0 = direction.clone();
}

fn adjust_head_direction(
    mut q_head: Query<&mut Transform, With<Head>>,
    q_direction: Query<&Direction>,
) {
    let mut transform = q_head.single_mut();
    match q_direction.single() {
        Direction::Up => {
            transform.rotate_z(f32::to_radians(90.0));
        }
        Direction::Down => {
            transform.rotate_z(f32::to_radians(-90.0));
        }
        Direction::Left => {
            transform.rotate_z(f32::to_radians(180.0));
        }
        Direction::Right => {}
    }
}

fn change_direction(
    mut query: Query<(&mut Direction, &LastDirection)>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let (mut direction, last_direction) = query.single_mut();

    let mut pressed_direction = Vec2::default();
    if keys.any_just_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        pressed_direction.y += 1.0;
    }
    if keys.any_just_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        pressed_direction.y -= 1.0;
    }
    if keys.any_just_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        pressed_direction.x -= 1.0;
    }
    if keys.any_just_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        pressed_direction.x += 1.0;
    }

    if matches!(last_direction.0, Direction::Left | Direction::Right) {
        *direction = match pressed_direction.y {
            1.0 => Direction::Up,
            -1.0 => Direction::Down,
            _ => direction.clone(),
        }
    } else {
        *direction = match pressed_direction.x {
            -1.0 => Direction::Left,
            1.0 => Direction::Right,
            _ => direction.clone(),
        }
    }
}

fn remove_tail(mut commands: Commands, query: Query<(Entity, &NextBodyPart), With<Tail>>) {
    let (tail, next_part) = query.single();
    commands.entity(tail).despawn();
    commands
        .entity(next_part.0.expect("expected tail to have a next_part"))
        .remove::<Body>()
        .insert(Tail);
}

fn spawn_part<Part: Component + PartialEq<Head>>(
    commands: &mut Commands,
    part: Part,
    position: Vec2,
    constants: &Constants,
    next_part: NextBodyPart,
) -> Entity {
    if part == Head {
        commands
            .spawn((
                part,
                BodyPart,
                next_part,
                Sprite::from_image(constants.head_texture_handle.clone()),
                Transform::from_xyz(position.x, position.y, 0.0),
            ))
            .id()
    } else {
        commands
            .spawn((
                part,
                BodyPart,
                next_part,
                Mesh2d(constants.mesh_handle.clone()),
                MeshMaterial2d(constants.color_handle.clone()),
                Transform::from_xyz(position.x, position.y, 0.0),
            ))
            .id()
    }
}

fn spawn_apple(
    commands: &mut Commands,
    size: f32,
    apple_texture: Handle<Image>,
    body_part_positions: Vec<Vec2>,
) {
    let mut spawn_points = Vec::new();
    for x in -11..11 {
        for y in -6..6 {
            spawn_points.push(Vec2::new(x as f32 * size, y as f32 * size));
        }
    }
    for position in body_part_positions {
        spawn_points.retain(|p| p != &position);
    }

    spawn_points.shuffle(&mut rand::rng());
    commands.spawn((
        Apple,
        Sprite::from_image(apple_texture),
        Transform::from_translation(
            spawn_points
                .first()
                .expect("expected spawn point")
                .extend(0.0),
        ),
    ));
}

fn eat_apple(
    head_query: Query<&Transform, With<Head>>,
    apple_query: Query<(Entity, &Transform), With<Apple>>,
    mut apple_eaten_event: EventWriter<AppleEatenEvent>,
) {
    let head_transform = head_query.single();
    let (apple, apple_transform) = apple_query.single();

    if head_transform.translation.truncate() == apple_transform.translation.truncate() {
        apple_eaten_event.send(AppleEatenEvent(apple));
    }
}

fn play_crunch_sound(mut commands: Commands, apple_crunch: Res<AppleCrunch>) {
    let handle = apple_crunch
        .handles
        .choose(&mut rand::rng())
        .expect("handles");

    commands.spawn((AudioPlayer(handle.clone()), PlaybackSettings::DESPAWN));
}

fn grow(
    mut commands: Commands,
    mut apple_eaten_event: EventReader<AppleEatenEvent>,
    constants: Res<Constants>,
    body_parts: Query<&Transform, With<BodyPart>>,
) {
    for apple in apple_eaten_event.read() {
        commands.entity(apple.0).despawn();
    }

    let positions = body_parts
        .iter()
        .map(|t| t.translation.truncate())
        .collect::<Vec<_>>();

    spawn_apple(
        &mut commands,
        constants.size,
        constants.apple_texture_handle.clone(),
        positions,
    );
}

fn update_score(
    mut set: ParamSet<(
        Query<(&mut Text2d, &mut Score)>,
        Query<(&mut Text2d, &mut HighScore)>,
    )>,
) {
    let current_score;
    {
        let mut q_score = set.p0();
        let (mut text, mut score) = q_score.single_mut();
        score.0 += 1;
        current_score = score.0;
        text.0 = format!("Score: {}", score.0);
    }

    let mut q_high_score = set.p1();
    let (mut text, mut high_score) = q_high_score.single_mut();
    if high_score.0 < current_score {
        high_score.0 += 1;
        text.0 = format!("High Score: {}", high_score.0);
        save_high_score(&high_score).expect("could not save high score");
    }
}
