// TODO: move more entity rendering code here.

use std::sync::Arc;

use glam::{Vec2, Vec3, vec2, vec3};
use glow::HasContext;
use mp3d_core::entity::Entity;

use crate::abs::{Mesh, Vertex};

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

pub fn player_model(gl: &Arc<glow::Context>) -> Mesh {
    let width = mp3d_core::entity::PlayerEntity::width();
    let height = mp3d_core::entity::PlayerEntity::height();
    let hw = width / 2.0;

    let (y0, y1) = (0.0, height);

    #[rustfmt::skip]
    let vertices: Vec<EntityVertex> = vec![
        // North
        EntityVertex { position: vec3(-hw, y1, -hw), uv: vec2(0.0, 1.0), normal: vec3(0.0, 0.0, -1.0) },
        EntityVertex { position: vec3( hw, y1, -hw), uv: vec2(1.0, 1.0), normal: vec3(0.0, 0.0, -1.0) },
        EntityVertex { position: vec3( hw, y0, -hw), uv: vec2(1.0, 0.0), normal: vec3(0.0, 0.0, -1.0) },
        EntityVertex { position: vec3(-hw, y0, -hw), uv: vec2(0.0, 0.0), normal: vec3(0.0, 0.0, -1.0) },
        // South
        EntityVertex { position: vec3( hw, y1,  hw), uv: vec2(0.0, 1.0), normal: vec3(0.0, 0.0, 1.0) },  
        EntityVertex { position: vec3(-hw, y1,  hw), uv: vec2(1.0, 1.0), normal: vec3(0.0, 0.0, 1.0) },  
        EntityVertex { position: vec3(-hw, y0,  hw), uv: vec2(1.0, 0.0), normal: vec3(0.0, 0.0, 1.0) },  
        EntityVertex { position: vec3( hw, y0,  hw), uv: vec2(0.0, 0.0), normal: vec3(0.0, 0.0, 1.0) },  
        // East    
        EntityVertex { position: vec3( hw, y1, -hw), uv: vec2(0.0, 1.0), normal: vec3(1.0, 0.0, 0.0) },
        EntityVertex { position: vec3( hw, y1,  hw), uv: vec2(1.0, 1.0), normal: vec3(1.0, 0.0, 0.0) },
        EntityVertex { position: vec3( hw, y0,  hw), uv: vec2(1.0, 0.0), normal: vec3(1.0, 0.0, 0.0) },
        EntityVertex { position: vec3( hw, y0, -hw), uv: vec2(0.0, 0.0), normal: vec3(1.0, 0.0, 0.0) },
        // West
        EntityVertex { position: vec3(-hw, y1,  hw), uv: vec2(0.0, 1.0), normal: vec3(-1.0, 0.0, 0.0) },
        EntityVertex { position: vec3(-hw, y1, -hw), uv: vec2(1.0, 1.0), normal: vec3(-1.0, 0.0, 0.0) },
        EntityVertex { position: vec3(-hw, y0, -hw), uv: vec2(1.0, 0.0), normal: vec3(-1.0, 0.0, 0.0) },
        EntityVertex { position: vec3(-hw, y0,  hw), uv: vec2(0.0, 0.0), normal: vec3(-1.0, 0.0, 0.0) },
        // Up
        EntityVertex { position: vec3(-hw, y0, -hw), uv: vec2(0.0, 1.0), normal: vec3(0.0, 1.0, 0.0) },
        EntityVertex { position: vec3( hw, y0, -hw), uv: vec2(1.0, 1.0), normal: vec3(0.0, 1.0, 0.0) },
        EntityVertex { position: vec3( hw, y0,  hw), uv: vec2(1.0, 0.0), normal: vec3(0.0, 1.0, 0.0) },
        EntityVertex { position: vec3(-hw, y0,  hw), uv: vec2(0.0, 0.0), normal: vec3(0.0, 1.0, 0.0) },
        // Down
        EntityVertex { position: vec3(-hw, y1,  hw), uv: vec2(0.0, 1.0), normal: vec3(0.0, -1.0, 0.0) },
        EntityVertex { position: vec3( hw, y1,  hw), uv: vec2(1.0, 1.0), normal: vec3(0.0, -1.0, 0.0) },
        EntityVertex { position: vec3( hw, y1, -hw), uv: vec2(1.0, 0.0), normal: vec3(0.0, -1.0, 0.0) },
        EntityVertex { position: vec3(-hw, y1, -hw), uv: vec2(0.0, 0.0), normal: vec3(0.0, -1.0, 0.0) },
    ];

    let mut indices: Vec<u32> = Vec::new();

    for i in 0..6 {
        let base = i * 4;
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    Mesh::new(gl, &vertices, &indices, glow::TRIANGLES)
}
