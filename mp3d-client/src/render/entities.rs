// TODO: move more entity rendering code here.

use std::sync::Arc;

use glam::{Mat4, Vec2, Vec3};
use glow::HasContext;
use mp3d_core::entity::entities;

use crate::{
    abs::{ShaderProgram, TextureHandle, Vertex},
    resource::entity::modelstore::EntityModelLoader,
};

#[repr(C)]
pub struct EntityVertex {
    pub position: Vec3,
    pub uv: Vec2,
    pub normal: Vec3,
}

impl Vertex for EntityVertex {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,
                3,
                glow::FLOAT,
                false,
                std::mem::size_of::<EntityVertex>() as i32,
                0,
            );
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                2,
                glow::FLOAT,
                false,
                std::mem::size_of::<EntityVertex>() as i32,
                std::mem::size_of::<Vec3>() as i32,
            );
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(
                2,
                3,
                glow::FLOAT,
                false,
                std::mem::size_of::<EntityVertex>() as i32,
                (std::mem::size_of::<Vec3>() + std::mem::size_of::<Vec2>()) as i32,
            );
        }
    }
}

pub struct EntityRenderers {
    pub player: PlayerRenderer,
    pub cat: CatRenderer,
    pub item: ItemRenderer,
}

impl EntityRenderers {
    pub fn new(gl: &Arc<glow::Context>) -> Self {
        Self {
            player: PlayerRenderer::new(gl),
            cat: CatRenderer::new(gl),
            item: ItemRenderer::new(gl),
        }
    }
}

pub struct PlayerRenderer {
    gl: Arc<glow::Context>,
}

impl PlayerRenderer {
    fn new(gl: &Arc<glow::Context>) -> Self {
        Self { gl: Arc::clone(gl) }
    }

    pub fn draw(
        &self,
        pos: Vec3,
        yaw: f32,
        tex: TextureHandle,
        models: &EntityModelLoader,
        entity_shader: &ShaderProgram,
    ) {
        let model_mat = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw.to_radians());
        let Some(model) = models.get(None) else {
            log::error!("Player model is not loaded!");
            return;
        };
        entity_shader.use_program();
        entity_shader.set_uniform("u_model", model_mat);
        tex.bind(&self.gl, 0);
        model.draw();
    }
}

pub struct CatRenderer {
    gl: Arc<glow::Context>,
}

impl CatRenderer {
    fn new(gl: &Arc<glow::Context>) -> Self {
        Self { gl: Arc::clone(gl) }
    }

    pub fn draw(
        &self,
        pos: Vec3,
        yaw: f32,
        tex: TextureHandle,
        models: &EntityModelLoader,
        entity_shader: &ShaderProgram,
    ) {
        let model_mat = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw.to_radians());
        let Some(model) = models.get(Some(*entities::CAT)) else {
            log::error!("Cat model is not loaded!");
            return;
        };
        entity_shader.use_program();
        entity_shader.set_uniform("u_model", model_mat);
        tex.bind(&self.gl, 0);
        model.draw();
    }
}

pub struct ItemRenderer {
    gl: Arc<glow::Context>,
}

impl ItemRenderer {
    fn new(gl: &Arc<glow::Context>) -> Self {
        Self { gl: Arc::clone(gl) }
    }

    pub fn draw(
        &self,
        pos: Vec3,
        yaw: f32,
        tex: TextureHandle,
        models: &EntityModelLoader,
        entity_shader: &ShaderProgram,
    ) {
        let model_mat = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw.to_radians());
        let Some(model) = models.get(Some(*entities::ITEM)) else {
            log::error!("Item model is not loaded!");
            return;
        };
        entity_shader.use_program();
        entity_shader.set_uniform("u_model", model_mat);
        tex.bind(&self.gl, 0);
        model.draw();
    }
}
