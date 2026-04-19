//! All utilities related to rendering particles

use glam::{FloatExt, IVec3, Mat4, Vec2, Vec3};
use glow::HasContext;
use mp3d_core::block::{Block, BlockState};

use crate::{
    abs::{InstanceData, Mesh, ShaderProgram},
    scenes::Assets,
    shader_program,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticleSprite {
    Block {
        block: &'static str,
        state: &'static str,
    },
    #[allow(dead_code)]
    Texture { texture: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub lifetime: f32,
    pub age: f32,
    pub size: f32,
    pub has_gravity: bool,
    pub sprite: ParticleSprite,
}

impl Particle {
    pub fn update(&mut self, delta_time: f32) {
        self.position += self.velocity * delta_time;
        if self.has_gravity {
            self.velocity.y -= 12.0 * delta_time;
        }
        self.age += delta_time;
    }

    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime
    }
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
    particle_instances: Vec<ParticleInstance>,
    mesh: Option<Mesh>,
    shader: ShaderProgram,
}

impl ParticleSystem {
    pub fn new(gl: &std::sync::Arc<glow::Context>) -> Self {
        let particle_shader = shader_program!(particle, gl, "..");

        Self {
            particles: Vec::new(),
            particle_instances: Vec::new(),
            mesh: None,
            shader: particle_shader,
        }
    }

    pub fn emit(&mut self, particle: Particle) {
        self.particles.push(particle);
    }

    pub fn block_break(&mut self, position: IVec3, block: &Block, block_state: &BlockState) {
        if let Some(state_ident) = block_state.to_ident() {
            for _ in 0..64 {
                let position = position.as_vec3()
                    + Vec3::new(
                        rand::random::<f32>(),
                        rand::random::<f32>(),
                        rand::random::<f32>(),
                    );
                let velocity = Vec3::new(
                    rand::random::<f32>() * 2.0 - 1.0,
                    rand::random::<f32>() * 2.0,
                    rand::random::<f32>() * 2.0 - 1.0,
                );
                let lifetime = rand::random::<f32>() + 0.5;
                let size = 0.1;
                self.emit(Particle {
                    position,
                    velocity,
                    lifetime,
                    age: 0.0,
                    size,
                    has_gravity: true,
                    sprite: ParticleSprite::Block {
                        block: block.ident,
                        state: state_ident,
                    },
                });
            }
        }
    }

    pub fn update(&mut self, delta_time: f32, assets: &Assets) {
        for particle in &mut self.particles {
            particle.update(delta_time);
        }
        self.particles.retain(|p| p.is_alive());
        self.particle_instances = self
            .particles
            .iter()
            .filter_map(|p| ParticleInstance::from(p, assets))
            .collect();
    }

    fn mesh(&mut self, gl: &std::sync::Arc<glow::Context>) -> &Mesh {
        if self.mesh.is_none() {
            let vertices = [
                Vec2::new(-0.5, -0.5),
                Vec2::new(0.5, -0.5),
                Vec2::new(0.5, 0.5),
                Vec2::new(-0.5, 0.5),
            ];
            let indices = [0u32, 1, 2, 2, 3, 0];
            self.mesh = Some(Mesh::new_instanced(
                gl,
                &vertices,
                &indices,
                &self.particle_instances,
                glow::TRIANGLES,
            ));
        }

        self.mesh
            .as_mut()
            .unwrap()
            .update_instances(&self.particle_instances);
        self.mesh.as_ref().unwrap()
    }

    pub fn render(
        &mut self,
        gl: &std::sync::Arc<glow::Context>,
        assets: &Assets,
        view: Mat4,
        projection: Mat4,
    ) {
        self.shader.use_program();
        self.shader.set_uniform("u_view", view);
        self.shader.set_uniform("u_proj", projection);
        self.shader.set_uniform("u_block_atlas", 0);
        assets.block_textures.upload(gl).bind(0);
        self.mesh(gl).draw_instanced();
    }
}

#[repr(C)]
pub struct ParticleInstance {
    pub position: Vec3,
    pub size: f32,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub sprite_type: u32, // 0 for block, 1 for texture
}

impl ParticleInstance {
    pub fn from(particle: &Particle, assets: &Assets) -> Option<Self> {
        match particle.sprite {
            ParticleSprite::Block { block, state } => {
                let Some([uv_min, uv_max]) = assets
                    .block_models
                    .get(&(block, state))
                    .and_then(|m| m.particle.as_ref())
                    .and_then(|p| assets.block_textures.get_uv(p, [Vec2::ZERO, Vec2::ONE]))
                else {
                    log::warn!(
                        "Failed to get UV coordinates for block '{}', state '{}'",
                        block,
                        state
                    );
                    return None;
                };
                Some(Self {
                    position: particle.position,
                    size: particle.size,
                    uv_min,
                    uv_max,
                    sprite_type: 0,
                })
            }
            ParticleSprite::Texture { texture: _ } => {
                log::error!("TODO: separate particle texture atlas");
                None
            }
        }
    }
}

impl InstanceData for ParticleInstance {
    fn instance_attribs(gl: &glow::Context) {
        unsafe {
            let mut offset = 0;
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                3,
                glow::FLOAT,
                false,
                size_of::<ParticleInstance>() as i32,
                offset as i32,
            );
            gl.vertex_attrib_divisor(1, 1);
            offset += size_of::<Vec3>();
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(
                2,
                1,
                glow::FLOAT,
                false,
                size_of::<ParticleInstance>() as i32,
                offset as i32,
            );
            gl.vertex_attrib_divisor(2, 1);
            offset += size_of::<f32>();
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(
                3,
                2,
                glow::FLOAT,
                false,
                size_of::<ParticleInstance>() as i32,
                offset as i32,
            );
            gl.vertex_attrib_divisor(3, 1);
            offset += size_of::<Vec2>();
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_f32(
                4,
                2,
                glow::FLOAT,
                false,
                size_of::<ParticleInstance>() as i32,
                offset as i32,
            );
            gl.vertex_attrib_divisor(4, 1);
            offset += size_of::<Vec2>();
            gl.enable_vertex_attrib_array(5);
            gl.vertex_attrib_pointer_i32(
                5,
                1,
                glow::INT,
                size_of::<ParticleInstance>() as i32,
                offset as i32,
            );
            gl.vertex_attrib_divisor(5, 1);
        }
    }
}
