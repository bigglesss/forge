use std::path::PathBuf;

use bevy::{prelude::*, render::render_resource::{Extent3d, TextureDimension, TextureFormat}};

use bevy::render::mesh::{self, PrimitiveTopology};
use bevy::render::{render_resource::SamplerDescriptor, texture::ImageSampler};
use bevy::utils::HashMap;
use bevy::window::PresentMode;

use bevy_flycam::{NoCameraPlayerPlugin, FlyCam, MovementSettings};

use wgpu_types::{AddressMode, FilterMode};

use wow_bin::parser;
use wow_bin::types::chunks;


fn main() {
    // TODO: Split into startup systems for ADT loading, BLP loading, etc.
    // Store in some kind of HashMap resource of X/Y -> ADT?
    // Should probably load a WDT instead and pick the four centre chunks to render.
    // Maybe use a smaller WDT to test.
    let adt = parser::parse_adt(PathBuf::from("./test_data/Azeroth/Azeroth_31_58.adt"), chunks::MPHDFlags {has_height_texturing: false})
        .expect("ADT should parse correctly:");
    let adt2 = parser::parse_adt(PathBuf::from("./test_data/Azeroth/Azeroth_32_58.adt"), chunks::MPHDFlags {has_height_texturing: false})
        .expect("ADT should parse correctly:");
    let adt3 = parser::parse_adt(PathBuf::from("./test_data/Azeroth/Azeroth_31_59.adt"), chunks::MPHDFlags {has_height_texturing: false})
        .expect("ADT should parse correctly:");
    let adt4 = parser::parse_adt(PathBuf::from("./test_data/Azeroth/Azeroth_32_59.adt"), chunks::MPHDFlags {has_height_texturing: false})
        .expect("ADT should parse correctly:");

    App::new()
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Immediate,
            ..default()
        })
        .insert_resource(vec![adt, adt2, adt3, adt4])
        .insert_resource(HashMap::<(String, usize), Handle<StandardMaterial>>::new())
        .add_plugins(DefaultPlugins)
        .add_plugin(NoCameraPlayerPlugin)
        .insert_resource(MovementSettings {
            sensitivity: 0.00010,
            speed: 20.0,
        })
        .add_startup_system(render_terrain)
        .add_startup_system(setup)
        .run();
}

fn create_material_from_blp(
    raw_filename: &String,
    textures: &mut ResMut<Assets<Image>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Handle<StandardMaterial> {
    let filename = format!("./test_data/{}", raw_filename.replace("\\", "/"));

    let blp = parser::parse_blp(PathBuf::from(&filename))
        .expect(format!("BLPs should be valid: {}", &filename).as_str());

    let mut tex = Image::new(
        Extent3d {width: blp.width as u32, height: blp.height as u32, ..default()}, 
        TextureDimension::D2, blp.mipmaps[0].decompressed.clone(), 
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

    let texture_handle = textures.add(tex);

    let material = materials.add(StandardMaterial {
        base_color_texture: Some(texture_handle.clone()),
        ..default()
    });

    material
}

fn render_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut textures: ResMut<Assets<Image>>,
    adts: Res<Vec<parser::ADT>>,
    mut blp_lookup: ResMut<HashMap<(String, usize), Handle<StandardMaterial>>>,
) {
    for adt in adts.iter() {
        // Load all BLPs.
        if let Some(mtex) = &adt.mtex {
            for (i, filename) in mtex.filenames.iter().enumerate() {
                let material = create_material_from_blp(filename, &mut textures, &mut materials);
                blp_lookup.insert((adt.key.clone(), i), material);
            }
        }

        for chunk in adt.mcnk.iter() {
            let base_texture = chunk.mcly.layers[0].texture_id;
            let base_material = &blp_lookup.get(&(adt.key.clone(), base_texture as usize)).expect("Missing BLP");
            create_chunk_heightmesh(&mut commands, &mut meshes, &base_material, chunk);
        }
    }
}

fn create_chunk_heightmesh(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: &Handle<StandardMaterial>,
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
    
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(mesh),
        // material: materials.add(Color::rgb(0.2, 0.2, 0.2).into()),
        material: material.clone(),
        ..default()
    });
}

fn setup(
    mut commands: Commands,
    adts: Res<Vec<parser::ADT>>,
    ) {
    let random_chunk_heights = &adts[0].mcnk.last().unwrap()
        .mcvt.heights;

    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(-14188.0, 186.0, 185.0),
            rotation: Quat::from_array([-0.20046994, 0.44985244, 0.10437336, 0.86403173]),
            ..default()
        },
        ..default()
    });

    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(random_chunk_heights[10].x, random_chunk_heights[10].z, random_chunk_heights[10].y).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    }).insert(FlyCam);
}

