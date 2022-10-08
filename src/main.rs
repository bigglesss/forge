use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::{render_resource::{Extent3d, TextureDimension, TextureFormat}, settings::WgpuSettings},
    utils::hashbrown::HashMap, pbr::wireframe::{WireframePlugin, WireframeConfig, Wireframe}, tasks::{AsyncComputeTaskPool, Task}
};
use bevy_egui::{egui::{self, Color32}, EguiContext, EguiPlugin};

use bevy::render::mesh::{self, PrimitiveTopology};
use bevy::render::{render_resource::SamplerDescriptor, texture::ImageSampler};
use bevy::window::PresentMode;

use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin};

use futures_lite::future;

use materials::{CustomMaterial, WaterMaterial};
use wgpu_types::{AddressMode, FilterMode, Features};

use wow_chunky::{chunks, files};


mod materials;

fn get_adts_in_range(origin: (i32, i32), range: i32) -> Vec<(u32, u32)> {
    if range == 0 {
        return vec![(origin.0 as u32, origin.1 as u32)]
    }

    let mut adts: Vec<(u32, u32)> = Vec::new();
    for x in -range..=range {
        for y in -range..=range {
            let adt_x = (origin.0 + x) as u32;
            let adt_y = (origin.1 + y) as u32;
            adts.push((adt_x, adt_y));
        }
    }

    adts
}

fn main() {
    let wdt = files::WDT::from_file(PathBuf::from("./test_data/Azeroth/Azeroth.wdt"))
        .expect("WDT should parse correctly.");

    App::new()
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Immediate,
            ..default()
        })
        .insert_resource(WgpuSettings {
            features: Features::POLYGON_MODE_LINE,
            ..default()
        })
        .insert_resource(Msaa { samples: 4 })

        .insert_resource(wdt)

        .insert_resource(HashMap::<(String, usize), Handle<Image>>::new())
        .insert_resource(HashMap::<(u32, u32), Vec<Entity>>::new())
        // TODO: Should actually link to an adt key + chunk key, so the ui system can find the types from the stored adts (which should be a hashmap).
        .insert_resource(HashMap::<ChunkCoords, (String, Option<chunks::adt::MTEX>, chunks::adt::MCNK)>::new())

        .insert_resource(Vec::<files::ADT>::new())

        .add_plugins(DefaultPlugins)
        .add_plugin(MaterialPlugin::<CustomMaterial>::default())
        .add_plugin(MaterialPlugin::<WaterMaterial>::default())
        .add_plugin(NoCameraPlayerPlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(WireframePlugin)
        .insert_resource(MovementSettings {
            sensitivity: 0.00010,
            speed: 100.0,
        })
        .add_startup_system(setup)
        .add_system(chunk_queuer)
        .add_system(chunk_loader.after(chunk_queuer))
        .add_system(render_terrain.after(chunk_loader))
        .add_system(chunk_coordinates.after(chunk_loader))
        .add_system(input)
        .add_system(ui)
        .run();
}

fn generate_image_from_buffer(width: u32, height: u32, data: &Vec<u8>) -> Image {
    let mut tex = Image::new(
        Extent3d {
            width,
            height,
            ..default()
        },
        TextureDimension::D2,
        data.clone(),
        TextureFormat::Rgba8Unorm,
    );

    // Wrap u and v values, to allow for easier tiling.
    tex.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        ..default()
    });

    tex
}

fn process_blp(raw_filename: &String, textures: &mut ResMut<Assets<Image>>) -> Handle<Image> {
    let specular_filename = format!(
        "./test_data/{}_s.blp",
        raw_filename.replace("\\", "/").replace(".blp", "")
    );
    let normal_filename = format!("./test_data/{}", raw_filename.replace("\\", "/"));

    let specular_path = PathBuf::from(&specular_filename);
    let normal_path = PathBuf::from(&normal_filename);

    let path = if specular_path.exists() {
        specular_path
    } else {
        normal_path
    };

    // TODO: Specular textures are being loaded, but probably not being used properly.
    // In-game textures look noticably less flat, even with constrast turned up. Look into improving the lighting quality or handling speculars properly?
    let blp = files::BLP::try_from(path.clone())
        .expect(format!("BLPs should be valid: {:?}", &path).as_str());

    let texture = generate_image_from_buffer(blp.width, blp.height, &blp.mipmaps[0].decompressed);
    let texture_handle = textures.add(texture);

    texture_handle
}

fn process_alpha_map(data: &Vec<u8>, textures: &mut ResMut<Assets<Image>>) -> Handle<Image> {
    // Multiply alphas by 17 to readjust the range from 0-15 to 0-255.
    let data: Vec<u8> = data.into_iter().map(|v| v * 17).collect();

    let mut tex = Image::new(
        Extent3d {
            width: 64,
            height: 64,
            ..default()
        },
        TextureDimension::D2,
        data,
        TextureFormat::R8Unorm,
    );

    // Wrap u and v values, to allow for easier tiling.
    tex.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });

    let texture_handle = textures.add(tex);

    texture_handle
}

fn render_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    mut textures: ResMut<Assets<Image>>,
    adts: Res<Vec<files::ADT>>,
    mut blp_lookup: ResMut<HashMap<(String, usize), Handle<Image>>>,
    mut adt_entities_lookup: ResMut<HashMap<(u32, u32), Vec<Entity>>>,
) {
    for adt in adts.iter() {
        // Skip ADTs we've already loaded.
        if adt_entities_lookup.get(&(adt.x, adt.y)).is_some() {
            continue;
        }

        // Load all BLPs.
        if let Some(mtex) = &adt.mtex {
            for (i, filename) in mtex.filenames.iter().enumerate() {
                let texture = process_blp(filename, &mut textures);
                blp_lookup.insert((adt.filename.clone(), i), texture);
            }
        }

        let mut adt_entities: Vec<Entity> = Vec::new();
        // Render chunks.
        for chunk in adt.mcnk.iter() {
            let mut layers: Vec<Option<Handle<Image>>> = vec![None, None, None, None];
            // The first layer never uses alpha.
            let mut alphas: Vec<Option<Handle<Image>>> = vec![
                Some(process_alpha_map(&vec![0 as u8; 64 * 64], &mut textures)),
                Some(process_alpha_map(&vec![0 as u8; 64 * 64], &mut textures)),
                Some(process_alpha_map(&vec![0 as u8; 64 * 64], &mut textures)),
            ];

            for (i, texture_layer) in chunk.mcly.layers.iter().enumerate() {
                let texture_id = texture_layer.texture_id as usize;
                layers[i] = blp_lookup
                    .get(&(adt.filename.clone(), texture_id))
                    .and_then(|t| Some(t.clone()));
            }

            for (i, alpha_layer) in chunk.mcal.layers.iter().enumerate() {
                let alpha_map = process_alpha_map(&alpha_layer.alpha_map, &mut textures);
                alphas[i] = Some(alpha_map);
            }

            let chunk_entities = create_chunk_heightmesh(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut water_materials,
                layers,
                alphas,
                chunk,
            );
            adt_entities.extend(chunk_entities);
        }

        adt_entities_lookup.insert((adt.x, adt.y), adt_entities);
    }
}

fn create_chunk_heightmesh(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<CustomMaterial>>,
    water_materials: &mut ResMut<Assets<WaterMaterial>>,
    layers: Vec<Option<Handle<Image>>>,
    alphas: Vec<Option<Handle<Image>>>,
    chunk: &chunks::adt::MCNK,
) -> Vec<Entity> {
    let mut chunk_entities: Vec<Entity> = Vec::new();

    // Render the ground mesh.
    let ground_id = create_ground_mesh(commands, meshes, materials, layers, alphas, chunk);
    chunk_entities.push(ground_id);

    // Render water if it exists in the chunk.
    if chunk.flags.lq_ocean { //|| chunk.flags.lq_magma || chunk.flags.lq_river {
        let water_id = create_water_mesh(commands, meshes, water_materials, chunk);
        chunk_entities.push(water_id);
    }

    chunk_entities
}

fn create_ground_mesh(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<CustomMaterial>>,
    layers: Vec<Option<Handle<Image>>>,
    alphas: Vec<Option<Handle<Image>>>,
    chunk: &chunks::adt::MCNK,
) -> Entity {
    let mut indices: Vec<u32> = Vec::new();
    for x in 0..8 {
        for y in 0..8 {
            let current_index = y * 17 + x;

            indices.push(current_index + 1);
            indices.push(current_index + 9);
            indices.push(current_index);

            indices.push(current_index + 9);
            indices.push(current_index + 17);
            indices.push(current_index);

            indices.push(current_index + 18);
            indices.push(current_index + 17);
            indices.push(current_index + 9);

            indices.push(current_index + 18);
            indices.push(current_index + 9);
            indices.push(current_index + 1);
        }
    }

    let indices = mesh::Indices::U32(indices);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for (i, position) in chunk.mcvt.heights.iter().enumerate() {
        let position = [position.x, position.z, position.y];
        let normal = [
            chunk.mcnr.normals[i].x as f32,
            chunk.mcnr.normals[i].z as f32,
            chunk.mcnr.normals[i].y as f32,
        ];

        positions.push(position);
        normals.push(normal);
        uvs.push([position[0], position[2]])
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let heightmesh = commands.spawn_bundle(MaterialMeshBundle {
        mesh: meshes.add(mesh),
        material: materials.add(CustomMaterial {
            base_positions: Vec2::new(chunk.position.x, chunk.position.y),
            layer_1: layers[0].clone(),
            layer_2: layers[1].clone(),
            alpha_2: alphas[0].clone(),
            layer_3: layers[2].clone(),
            alpha_3: alphas[1].clone(),
            layer_4: layers[3].clone(),
            alpha_4: alphas[2].clone(),
        }),
        ..default()
    });

    heightmesh.id()
}

fn create_water_mesh(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    water_materials: &mut ResMut<Assets<WaterMaterial>>,
    chunk: &chunks::adt::MCNK,
) -> Entity {
    let spread = CHUNK_SIZE / 8.;

    let chunk_position = [chunk.position.x, chunk.position.y];
    let max_water_height = chunk.mclq.height.max;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for x in 0..9 {
        for y in 0..9 {
            let position = [chunk_position[0] - ((x as f32) * spread), max_water_height, chunk_position[1] - ((y as f32) * spread)];
            positions.push(position);
            normals.push([1.0, 1.0, 1.0]);
        }
    }

    let mut indices: Vec<u32> = Vec::new();
    for x in 0..8 {
        for y in 0..8 {
            indices.push(x + 9 + (y * 9));
            indices.push(x + (y * 9));
            indices.push(x + 1 + (y * 9));

            indices.push(x + 9 + (y * 9));
            indices.push(x + 1 + (y * 9));
            indices.push(x + 10 + (y * 9));
        }
    }

    let indices = mesh::Indices::U32(indices);

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

    let watermesh = commands.spawn_bundle(MaterialMeshBundle {
        mesh: meshes.add(mesh),
        material: water_materials.add(WaterMaterial {}),
        ..default()
    });

    watermesh.id()
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct ChunkCoords {
    x: i32,
    y: i32,
}

static ADT_SIZE: f32 = 533.33333;
static CHUNK_SIZE: f32 = ADT_SIZE / 16.;

impl ChunkCoords {
    fn from_wow_pos(position: chunks::shared::C3Vector) -> Self {
        let x = if position.x >= 0.0 {
            ((position.x / CHUNK_SIZE).floor()) as i32
        } else {
            ((position.x / CHUNK_SIZE).ceil()) as i32
        };
        let y = if position.y >= 0.0 {
            ((position.y / CHUNK_SIZE).floor()) as i32
        } else {
            ((position.y / CHUNK_SIZE).ceil()) as i32
        };

        Self { x, y }
    }

    fn from_game_pos(position: Vec3) -> Self {
        let x = if position.x >= 0.0 {
            ((position.x / CHUNK_SIZE).floor()) as i32
        } else {
            ((position.x / CHUNK_SIZE).ceil()) as i32
        };
        let y = if position.z >= 0.0 {
            ((position.z / CHUNK_SIZE).floor()) as i32
        } else {
            ((position.z / CHUNK_SIZE).ceil()) as i32
        };

        Self { x, y }
    }
}


fn setup(
    mut commands: Commands,
) {
    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(-ADT_SIZE * 2., 100., -ADT_SIZE * 0.)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(FlyCam);
}

#[derive(Component)]
struct AdtParsingTask(Task<Option<files::ADT>>);


/// Spawn chunk loading tasks as the camera moves around.
fn chunk_queuer(
    mut commands: Commands,
    camera: Query<&mut Transform, With<FlyCam>>,
    wdt: Res<files::WDT>,
    adts: Res<Vec<files::ADT>>,
    mut adt_entities_lookup: ResMut<HashMap<(u32, u32), Vec<Entity>>>,
    chunk_tasks: Query<(Entity, &mut AdtParsingTask)>,
) {
    let pool = AsyncComputeTaskPool::get();
    let cam_pos: Vec3 = camera.single().translation;

    let x = ((17066.66656 - cam_pos.x) / ADT_SIZE).floor();
    let y = ((17066.66656 - cam_pos.z) / ADT_SIZE).floor();

    // Get a list of ADTs that we actually need loaded at this point in time.
    let adt_coords = get_adts_in_range((y as i32, x as i32), 2);

    // Skip this cycle if the ADTs are already loaded.
    let active_adts: Vec<(u32, u32)> = adts.iter().map(|a| (a.x, a.y)).collect();
    if adt_coords.iter().all(|a| active_adts.contains(a)) {
        return
    }

    // Skip this cycle if we've already queued chunks.
    let count = chunk_tasks.iter().count();
    if count > 0 {
        return
    }

    // Despawn any ADTs we have loaded already that are out of range.
    adt_entities_lookup.retain(|k, v| {
        if !adt_coords.contains(k) {
            println!("Despawning: {:?}", k);
            for e in v {
                commands.entity(*e).despawn();
            }
            false
        } else {
            true
        }
    });

    println!("Active ADTs: {:?}", active_adts);
    println!("Attempting to load adts: {:?}", adt_coords);

    // Add ADT load futures to the queue. 
    for c in adt_coords {
        let adt_name = format!("{}_{}_{}.adt", wdt.path.file_stem().and_then(|n| n.to_str()).expect("WDT should have a extension."), c.0, c.1);
        let adt_path = wdt.path
            .parent().expect("WDT file should be in a folder with the ADT files.")
            .join(adt_name);

        let mphd_flags = wdt.mphd.as_ref().and_then(|chunk| Some(chunk.flags.clone())).expect("WDT should have a valid MPHD chunk");

        let task = pool.spawn(async move {
            files::ADT::from_file(adt_path, &mphd_flags).ok()
        });

        commands.spawn().insert(AdtParsingTask(task));
    }
}

fn chunk_loader(
    mut commands: Commands,
    mut adts: ResMut<Vec<files::ADT>>,
    mut chunk_tasks: Query<(Entity, &mut AdtParsingTask)>,
) {
    for (entity, mut task) in &mut chunk_tasks {
        if let Some(task) = future::block_on(future::poll_once(&mut task.0)) {
            if let Some(adt) = task {
                println!("Adding ADT to render list: {} {}", adt.x, adt.y);
                adts.push(adt);
            }

            commands.entity(entity).remove::<AdtParsingTask>();
        }
    }
}

fn chunk_coordinates(
    adts: Res<Vec<files::ADT>>,
    mut chunk_lookup: ResMut<HashMap<ChunkCoords, (String, Option<chunks::adt::MTEX>, chunks::adt::MCNK)>>,
) {
    for adt in adts.iter() {
        let mtex = &adt.mtex;
        for chunk in adt.mcnk.iter() {
            let coords = ChunkCoords::from_wow_pos(chunk.position);
            if chunk_lookup.get(&coords).is_none() {
                chunk_lookup.insert(coords, (adt.filename.clone(), mtex.clone(), chunk.clone()));
            }
        }
    }
}

fn input(
    keys: Res<Input<KeyCode>>,
    adts: Res<Vec<files::ADT>>,
    mut query: Query<&mut Transform, With<FlyCam>>,
    mut wireframe_config: ResMut<WireframeConfig>,
) {
    let mut cam = query.single_mut();

    if keys.any_just_pressed([KeyCode::Equals]) {
        wireframe_config.global = !wireframe_config.global;
    }

    if keys.just_pressed(KeyCode::Return) {
        let center_adt = &adts[adts.len() / 2];
        let center_chunk = &center_adt.mcnk[center_adt.mcnk.len() / 2];
        let initial_position = Vec3::new(
            center_chunk.position.x,
            center_chunk.position.z,
            center_chunk.position.y,
        );

        cam.translation = initial_position;
    }

    if keys.just_pressed(KeyCode::C) && keys.pressed(KeyCode::LControl) {
        println!(".go xyz {} {} {}", cam.translation.x, cam.translation.z, cam.translation.y);
    }
}

fn ui(
    mut egui_context: ResMut<EguiContext>,
    query: Query<&mut Transform, With<FlyCam>>,
    chunk_lookup: ResMut<HashMap<ChunkCoords, (String, Option<chunks::adt::MTEX>, chunks::adt::MCNK)>>,
    chunk_tasks: Query<(Entity, &mut AdtParsingTask)>,
) {
    let cam_pos: Vec3 = query.single().translation;
    let coords = ChunkCoords::from_game_pos(cam_pos);
    let location = chunk_lookup.get(&coords);

    egui::Window::new("Chunk info")
        .min_width(450.0)
        .show(egui_context.ctx_mut(), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.colored_label(Color32::LIGHT_YELLOW, format!("Loading {} chunks", chunk_tasks.iter().count()));
                    ui.label(format!("Position: {:?}", cam_pos));

                    if let Some(location) = location {
                        let (adt, mtex, chunk) = location;
                        ui.label(format!(
                            "Chunk: ({}) ({}, {}) {:#?}",
                            adt, chunk.x, chunk.y, chunk.mcly.layers
                        ));
                        ui.label(format!("Textures: {:#?}", mtex.as_ref().unwrap()));
                        ui.label(format!("Water: {:#?}", chunk.mclq));
                    }
                });
        });
}
