use std::path::PathBuf;

use bevy::{prelude::*, render::render_resource::{Extent3d, TextureDimension, TextureFormat}, utils::hashbrown::HashMap};
use bevy_egui::{egui, EguiContext, EguiPlugin};

use bevy::render::mesh::{self, PrimitiveTopology};
use bevy::render::{render_resource::SamplerDescriptor, texture::ImageSampler};
use bevy::window::PresentMode;

use bevy_flycam::{NoCameraPlayerPlugin, FlyCam, MovementSettings};

use materials::CustomMaterial;
use wgpu_types::{AddressMode, FilterMode};

use wow_chunky::parser;
use wow_chunky::types::chunks;

mod materials;

fn main() {
    let wdt = parser::wdt::WDT::from_file(PathBuf::from("./test_data/Azeroth/Azeroth.wdt"))
        .expect("WDT should parse correctly.");
    // TODO: Split into startup systems for ADT loading, BLP loading, etc.
    // Store in some kind of HashMap resource of X/Y -> ADT?
    // Should probably load a WDT instead and pick the four centre chunks to render.
    // Maybe use a smaller WDT to test.
    let adt = parser::adt::ADT::from_wdt(&wdt, 31, 40)
        .expect("ADT should parse correctly.");
    let adt2 = parser::adt::ADT::from_wdt(&wdt, 32, 40)
        .expect("ADT should parse correctly.");
    let adt3 = parser::adt::ADT::from_wdt(&wdt, 31, 41)
        .expect("ADT should parse correctly.");
    let adt4 = parser::adt::ADT::from_wdt(&wdt, 32, 41)
        .expect("ADT should parse correctly.");

    App::new()
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Immediate,
            ..default()
        })
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(vec![adt, adt2, adt3, adt4])
        .insert_resource(HashMap::<(String, usize), Handle<Image>>::new())
        .add_plugins(DefaultPlugins)
        .add_plugin(MaterialPlugin::<CustomMaterial>::default())
        .add_plugin(NoCameraPlayerPlugin)
        .add_plugin(EguiPlugin)
        .insert_resource(MovementSettings {
            sensitivity: 0.00010,
            speed: 30.0,
        })
        .add_startup_system(render_terrain)
        .add_startup_system(setup)
        .add_system(ui_example)
        .run();
}

fn generate_image_from_buffer(
    width: u32,
    height: u32,
    data: &Vec<u8>,
) -> Image {
    let mut tex = Image::new(
        Extent3d {width, height, ..default()}, 
        TextureDimension::D2, data.clone(), 
        TextureFormat::Rgba8Unorm
    );

    // Wrap u and v values, to allow for easier tiling.
    tex.sampler_descriptor = ImageSampler::Descriptor(
        SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            ..default()
        }
    );

    tex
}

fn process_blp(
    raw_filename: &String,
    textures: &mut ResMut<Assets<Image>>,
) -> Handle<Image> {
    let specular_filename = format!("./test_data/{}_s.blp", raw_filename.replace("\\", "/").replace(".blp", ""));
    let normal_filename = format!("./test_data/{}", raw_filename.replace("\\", "/"));

    let specular_path = PathBuf::from(&specular_filename);
    let normal_path = PathBuf::from(&normal_filename);

    let path = if specular_path.exists() {specular_path} else {normal_path};

    // TODO: Specular textures are being loaded, but probably not being used properly.
    // In-game textures look noticably less flat, even with constrast turned up. Look into improving the lighting quality or handling speculars properly?
    let blp = parser::parse_blp(&path)
        .expect(format!("BLPs should be valid: {:?}", &path).as_str());

    let texture = generate_image_from_buffer(blp.width, blp.height, &blp.mipmaps[0].decompressed);
    let texture_handle = textures.add(texture);

    texture_handle
}

fn process_alpha_map(
    data: &Vec<u8>,
    textures: &mut ResMut<Assets<Image>>,
) -> Handle<Image> {
    // Multiply alphas by 17 to readjust the range from 0-15 to 0-255.
    let data: Vec<u8> = data.into_iter().map(|v| v * 17).collect();

    let mut tex = Image::new(
        Extent3d {width: 64, height: 64, ..default()}, 
        TextureDimension::D2, data, 
        TextureFormat::R8Unorm,
    );

    // Wrap u and v values, to allow for easier tiling.
    tex.sampler_descriptor = ImageSampler::Descriptor(
        SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }
    );

    let texture_handle = textures.add(tex);

    texture_handle
}

fn render_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut textures: ResMut<Assets<Image>>,
    adts: Res<Vec<parser::adt::ADT>>,
    mut blp_lookup: ResMut<HashMap<(String, usize), Handle<Image>>>,
) {
    for adt in adts.iter() {
        // Load all BLPs.
        if let Some(mtex) = &adt.mtex {
            for (i, filename) in mtex.filenames.iter().enumerate() {
                let texture = process_blp(filename, &mut textures);
                blp_lookup.insert((adt.filename.clone(), i), texture);
            }
        }

        // Render chunks.
        for chunk in adt.mcnk.iter() {
            let mut layers: Vec<Option<Handle<Image>>> = vec![None, None, None, None];
            // The first layer never uses alpha.
            let mut alphas: Vec<Option<Handle<Image>>> = vec![
                Some(process_alpha_map(&vec![0 as u8; 64*64], &mut textures)),
                Some(process_alpha_map(&vec![0 as u8; 64*64], &mut textures)),
                Some(process_alpha_map(&vec![0 as u8; 64*64], &mut textures)),
            ];

            for (i, texture_layer) in chunk.mcly.layers.iter().enumerate() {
                let texture_id = texture_layer.texture_id as usize;
                layers[i] = blp_lookup.get(&(adt.filename.clone(), texture_id)).and_then(|t| Some(t.clone()));
            }

            for (i, alpha_layer) in chunk.mcal.layers.iter().enumerate() {
                let alpha_map = process_alpha_map(&alpha_layer.alpha_map, &mut textures);
                alphas[i] = Some(alpha_map);
            }

            create_chunk_heightmesh(&mut commands, &mut meshes, &mut materials, layers, alphas, chunk);
        }
    }
}

fn create_chunk_heightmesh(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<CustomMaterial>>,
    layers: Vec<Option<Handle<Image>>>,
    alphas: Vec<Option<Handle<Image>>>,
    chunk: &chunks::MCNK,
) {
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
        let normal = [chunk.mcnr.normals[i].x as f32, chunk.mcnr.normals[i].z as f32, chunk.mcnr.normals[i].y as f32];

        positions.push(position);
        normals.push(normal);
        uvs.push([position[0], position[2]])
    }

    let temp_positions = positions.clone();

    let min_position = temp_positions.iter().reduce(|acc, p| {
        if acc[0] < p[0] || acc[2] < p[2] {
            p
        } else {
            acc
        }
    }).unwrap();

    let max_position = temp_positions.iter().reduce(|acc, p| {
        if acc[0] > p[0] || acc[2] > p[2] {
            p
        } else {
            acc
        }
    }).unwrap();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    commands.spawn_bundle(MaterialMeshBundle {
        mesh: meshes.add(mesh),
        // material: materials.add(Color::rgb(0.2, 0.2, 0.2).into()),
        material: materials.add(CustomMaterial {
            base_positions: Vec4::new(chunk.position.x, chunk.position.y, max_position[0] - min_position[0], max_position[2] - min_position[2]),
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
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct ChunkCoords {
    x: i32,
    y: i32,
}

fn setup(
    mut commands: Commands,
    adts: Res<Vec<parser::adt::ADT>>,
    ) {
    let mut chunk_lookup: HashMap<ChunkCoords, (parser::adt::ADT, chunks::MCNK)> = HashMap::new();
    for adt in adts.iter() {
        for chunk in adt.mcnk.iter() {
            let c = ChunkCoords {
                x: (chunk.position.x / 33.334).floor() as i32,
                y: (chunk.position.y / 33.334).floor() as i32,
            };
            chunk_lookup.insert(c, (adt.clone(), chunk.clone()));
        }
    }

    commands.insert_resource(chunk_lookup);

    let _initial_position = Vec3::new(adts[0].mcnk[0].position.x, adts[0].mcnk[0].position.y, adts[0].mcnk[0].position.z);

    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-4337.3545, 13.121, -143.148).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    }).insert(FlyCam);

    println!("{:?}", (adts[0].mcnk[0].position.x, adts[0].mcnk[0].position.y, adts[0].mcnk[0].position.z));
}

fn ui_example(
    mut egui_context: ResMut<EguiContext>,
    query: Query<&mut Transform, With<FlyCam>>,
    chunk_lookup: Res<HashMap<ChunkCoords, (parser::adt::ADT, chunks::MCNK)>>,
) {
    let cam_pos: Vec3 = query.single().translation;

    let chunk_coords = ChunkCoords {
        x: ((cam_pos.x / 33.334) + 1.0).floor() as i32,
        y: ((cam_pos.z / 33.334) + 1.0).floor() as i32,
    };

    let location = chunk_lookup.get(&chunk_coords);

    egui::SidePanel::left("Info panel")
    .min_width(450.0)
    .show(egui_context.ctx_mut(), |ui| {
        ui.label(format!("Position: {:?}", cam_pos));
        ui.label(format!("Chunk coords: {:?}", &chunk_coords));

        if let Some(location) = location {
            let (adt, chunk) = location;
            ui.label(format!("Chunk: ({}) ({}, {}) {:#?}", adt.filename, chunk.x, chunk.y, chunk.mcly.layers));
            ui.label(format!("Textures: {:#?}", adt.mtex.as_ref().unwrap()));
        }
    });
}