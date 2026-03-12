//! Mesh management module.
//!
//! This module defines the [`Mesh`] struct for managing mesh data on the GPU side.
//! Vertices should implement the [`Vertex`] trait.

use std::sync::Arc;

use glow::HasContext;

/// Trait that defines the necessary methods for a vertex.
pub trait Vertex {
    /// Sets up the vertex attribute pointers for the vertex.
    fn vertex_attribs(gl: &glow::Context);
}

impl Vertex for glam::Vec3 {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,
                3,
                glow::FLOAT,
                false,
                3 * std::mem::size_of::<f32>() as i32,
                0,
            );
        }
    }
}

/// Represents a mesh stored on the GPU side.
pub struct Mesh {
    gl: Arc<glow::Context>,
    draw_mode: u32,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    index_count: usize,
}

impl Mesh {
    /// Creates a new mesh from the given vertex and index data.
    pub fn new<V: Vertex>(
        gl: &Arc<glow::Context>,
        vertices: &[V],
        indices: &[u32],
        draw_mode: u32,
    ) -> Self {
        unsafe {
            let vao = gl.create_vertex_array().unwrap();
            let vbo = gl.create_buffer().unwrap();
            let ebo = gl.create_buffer().unwrap();

            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    vertices.as_ptr() as *const u8,
                    std::mem::size_of_val(vertices),
                ),
                glow::DYNAMIC_DRAW,
            );

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    indices.as_ptr() as *const u8,
                    std::mem::size_of_val(indices),
                ),
                glow::DYNAMIC_DRAW,
            );

            V::vertex_attribs(gl);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);

            Self {
                gl: Arc::clone(gl),
                draw_mode,
                vao,
                vbo,
                ebo,
                index_count: indices.len(),
            }
        }
    }

    /// Updates the mesh.
    pub fn update<V: Vertex>(&mut self, vertices: &[V], indices: &[u32]) {
        unsafe {
            self.index_count = indices.len();

            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            self.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    vertices.as_ptr() as *const u8,
                    std::mem::size_of_val(vertices),
                ),
                glow::DYNAMIC_DRAW,
            );

            self.gl
                .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ebo));
            self.gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    indices.as_ptr() as *const u8,
                    std::mem::size_of_val(indices),
                ),
                glow::DYNAMIC_DRAW,
            );

            self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
            self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        }
    }

    /// Draws the mesh.
    pub fn draw(&self) {
        unsafe {
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.draw_elements(
                self.draw_mode,
                self.index_count as i32,
                glow::UNSIGNED_INT,
                0,
            );
            self.gl.bind_vertex_array(None);
        }
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_buffer(self.vbo);
            self.gl.delete_buffer(self.ebo);
            self.gl.delete_vertex_array(self.vao);
        }
    }
}
