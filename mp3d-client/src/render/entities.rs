// TODO: move more entity rendering code here.

use std::sync::Arc;

use glam::{Mat4, Vec2, Vec3};
use glow::HasContext;
use mp3d_core::entity::entities;

use crate::{
    abs::{ShaderProgram, Vertex},
    scenes::Assets,
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
    pub fn new() -> Self {
        Self {
            player: PlayerRenderer,
            cat: CatRenderer,
            item: ItemRenderer,
        }
    }
}

pub struct PlayerRenderer;

impl PlayerRenderer {
    pub fn draw(&self, pos: Vec3, yaw: f32, assets: &Arc<Assets>, entity_shader: &ShaderProgram) {
        let model_mat = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw.to_radians());
        let Some(model) = assets.entity_model_loader.get(None) else {
            log::error!("Player model is not loaded!");
            return;
        };
        entity_shader.use_program();
        entity_shader.set_uniform("u_model", model_mat);
        model.tex.as_ref().unwrap_or(&assets.missing).bind(0);
        model.mesh.draw();
    }
}

pub struct CatRenderer;

impl CatRenderer {
    pub fn draw(&self, pos: Vec3, yaw: f32, assets: &Arc<Assets>, entity_shader: &ShaderProgram) {
        let model_mat = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw.to_radians());
        let Some(model) = assets.entity_model_loader.get(Some(*entities::CAT)) else {
            log::error!("Cat model is not loaded!");
            return;
        };
        entity_shader.use_program();
        entity_shader.set_uniform("u_model", model_mat);
        model.tex.as_ref().unwrap_or(&assets.missing).bind(0);
        model.mesh.draw();
    }
}

pub struct ItemRenderer;

impl ItemRenderer {
    pub fn draw(&self, pos: Vec3, yaw: f32, assets: &Arc<Assets>, entity_shader: &ShaderProgram) {
        let model_mat = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw.to_radians());
        let Some(model) = assets.entity_model_loader.get(Some(*entities::ITEM)) else {
            log::error!("Item model is not loaded!");
            return;
        };
        entity_shader.use_program();
        entity_shader.set_uniform("u_model", model_mat);
        assets.missing.bind(0);
        model.mesh.draw();
    }
}
