//! All utilities related to rendering clouds.

use glam::Vec2;
use glow::HasContext;

use crate::abs::{Mesh, ShaderProgram, Texture, Vertex};

#[repr(C)]
pub struct CloudVertex(pub Vec2);

impl Vertex for CloudVertex {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,
                2,
                glow::FLOAT,
                false,
                size_of::<CloudVertex>() as i32,
                0,
            );
        }
    }
}

pub struct CloudRenderer {
    pub texture: Texture,
    pub mesh: Mesh,
    pub shader: ShaderProgram,
}

impl CloudRenderer {
    pub fn new(gl: &std::sync::Arc<glow::Context>) -> Self {
        let seed = rand::random::<i32>();
        log::info!("Generating cloud texture with seed: {}", seed);
        let mut noise = fastnoise_lite::FastNoiseLite::new();
        noise.set_noise_type(Some(fastnoise_lite::NoiseType::Perlin));
        noise.set_fractal_type(Some(fastnoise_lite::FractalType::FBm));
        noise.set_fractal_octaves(Some(4));
        noise.set_fractal_gain(Some(0.5));
        noise.set_fractal_lacunarity(Some(2.0));
        noise.set_seed(Some(seed));
        let mut data = Vec::new();
        let width = 256;
        let height = width;
        for z in 0..width {
            for x in 0..height {
                let value = noise.get_noise_2d(x as f32 * 15.0, z as f32 * 10.0);
                let alpha = ((value + 1.0) / 2.0 * 255.0) as u8;
                data.push(255);
                data.push(255);
                data.push(255);
                data.push(if alpha > 128 { 255 } else { 0 });
            }
        }
        let texture = Texture::new_bytes(gl, width, height, data);

        let vertices = [
            CloudVertex(Vec2::new(-1.0, -1.0)),
            CloudVertex(Vec2::new(1.0, -1.0)),
            CloudVertex(Vec2::new(1.0, 1.0)),
            CloudVertex(Vec2::new(-1.0, 1.0)),
        ];

        let indices = [0u32, 1, 2, 2, 3, 0];

        let mesh = Mesh::new(gl, &vertices, &indices, glow::TRIANGLES);

        let shader = crate::shader_program!(cloud, gl, "..");

        Self {
            texture,
            mesh,
            shader,
        }
    }
}
