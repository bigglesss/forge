use bevy::{render::render_resource::{AsBindGroup, ShaderRef}, reflect::TypeUuid, prelude::{Handle, Image, Material, Vec4}};

#[derive(AsBindGroup, TypeUuid, Debug, Default, Clone)]
#[uuid = "f5ec49f1-1a2e-4c3e-9f6f-836e54b1a576"]
pub struct CustomMaterial {
    #[uniform(0)]
    pub base_positions: Vec4,

    #[texture(1)]
    #[sampler(2)]
    pub layer_1: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub alpha_1: Option<Handle<Image>>,

    #[texture(5)]
    #[sampler(6)]
    pub layer_2: Option<Handle<Image>>,
    #[texture(7)]
    #[sampler(8)]
    pub alpha_2: Option<Handle<Image>>,

    #[texture(9)]
    #[sampler[10]]
    pub layer_3: Option<Handle<Image>>,
    #[texture(11)]
    #[sampler(12)]
    pub alpha_3: Option<Handle<Image>>,

    #[texture(13)]
    #[sampler(14)]
    pub layer_4: Option<Handle<Image>>,
    #[texture(15)]
    #[sampler(16)]
    pub alpha_4: Option<Handle<Image>>,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/texture1.wgsl".into()
    }
}
