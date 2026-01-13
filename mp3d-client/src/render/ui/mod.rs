//! All UI related utilities.
//!
//! This module contains functions and structures for rendering user interfaces. It includes
//! components such as text rendering, buttons, and other interactive elements. It additionally
//! includes a UI renderer for rendering 2D elements on the screen.

use glam::Vec2;
use glow::HasContext;

use crate::abs::Vertex;

pub struct UIVertex {
    pub position: Vec2,
    pub uv: Vec2,
}

impl Vertex for UIVertex {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            let stride = std::mem::size_of::<UIVertex>() as i32;
            // Position attribute
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            // UV attribute
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                2,
                glow::FLOAT,
                false,
                stride,
                2 * std::mem::size_of::<f32>() as i32,
            );
        }
    }
}

pub mod uirenderer;
pub mod widgets;
