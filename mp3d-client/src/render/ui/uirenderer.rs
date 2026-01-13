//! The UI renderer for the voxel engine.

use std::sync::Arc;

use glam::{Mat4, Vec2, Vec4};

use crate::{
    abs::{Mesh, ShaderProgram, TextureHandle},
    render::ui::UIVertex,
};

/// The rendering mode for a UI element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UIRenderMode {
    Texture(TextureHandle, Vec4),
    Color(Vec4),
}

/// A draw command for rendering a UI element.
pub struct DrawCommand {
    pub rect: [Vec2; 2],
    pub uv_rect: [Vec2; 2],
    pub mode: UIRenderMode,
}

/// The UI renderer for rendering 2D elements on the screen.
pub struct UIRenderer {
    gl: Arc<glow::Context>,
    shader_program: ShaderProgram,
    pub projection_matrix: Mat4,
    last_command: Option<DrawCommand>,
    vertices: Vec<UIVertex>,
    indices: Vec<u32>,
}

impl UIRenderer {
    /// Creates a new UI renderer.
    pub fn new(
        gl: &Arc<glow::Context>,
        shader_program: ShaderProgram,
        projection_matrix: Mat4,
    ) -> Self {
        Self {
            gl: Arc::clone(gl),
            shader_program,
            projection_matrix,
            last_command: None,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Adds a draw command to the UI renderer.
    pub fn add_command(&mut self, command: DrawCommand) {
        // If the last command's mode is the same this command's mode, we can batch them together.
        if let Some(last_command) = &self.last_command {
            if last_command.mode == command.mode {
                self.append_command(&command);
                self.last_command = Some(command);
            } else {
                // It did not match, so we need to flush the current batch by building the mesh.
                self.finish();
                self.append_command(&command);
                self.last_command = Some(command);
            }
        } else {
            self.append_command(&command);
            self.last_command = Some(command);
        }
    }

    /// Finishes the current batch and builds the mesh.
    pub fn finish(&mut self) {
        self.draw_mesh();
        self.vertices.clear();
        self.indices.clear();
        self.last_command = None;
    }

    /// Builds and draws the current mesh.
    fn draw_mesh(&mut self) {
        if self.vertices.is_empty() || self.indices.is_empty() {
            return;
        }

        let mesh = Mesh::new(&self.gl, &self.vertices, &self.indices, glow::TRIANGLES);

        self.shader_program.use_program();

        // Set up rendering state based on the last command's mode
        if let Some(last_command) = &self.last_command {
            match last_command.mode {
                UIRenderMode::Texture(texture_handle, color) => {
                    // Bind texture and set color uniform
                    texture_handle.bind(&self.gl, 0);
                    self.shader_program
                        .set_uniform("u_projection", self.projection_matrix);
                    self.shader_program.set_uniform("u_tex", 0);
                    self.shader_program.set_uniform("u_color", color);
                    self.shader_program.set_uniform("u_solid", false);
                }
                UIRenderMode::Color(color) => {
                    // Set color uniform
                    self.shader_program
                        .set_uniform("u_projection", self.projection_matrix);
                    self.shader_program.set_uniform("u_color", color);
                    self.shader_program.set_uniform("u_solid", true);
                }
            }
        }

        mesh.draw();
    }

    /// Appends a draw command's vertices and indices to the current batch.
    fn append_command(&mut self, command: &DrawCommand) {
        let base_index = self.vertices.len() as u32;
        let [min, max] = command.rect;
        let [uv_min, uv_max] = command.uv_rect;

        self.vertices.push(UIVertex {
            position: Vec2::new(max.x, min.y),
            uv: Vec2::new(uv_max.x, uv_min.y),
        });
        self.vertices.push(UIVertex {
            position: Vec2::new(min.x, min.y),
            uv: Vec2::new(uv_min.x, uv_min.y),
        });
        self.vertices.push(UIVertex {
            position: Vec2::new(min.x, max.y),
            uv: Vec2::new(uv_min.x, uv_max.y),
        });
        self.vertices.push(UIVertex {
            position: Vec2::new(max.x, max.y),
            uv: Vec2::new(uv_max.x, uv_max.y),
        });
        self.indices.push(base_index);
        self.indices.push(base_index + 1);
        self.indices.push(base_index + 2);
        self.indices.push(base_index);
        self.indices.push(base_index + 2);
        self.indices.push(base_index + 3);
    }
}
