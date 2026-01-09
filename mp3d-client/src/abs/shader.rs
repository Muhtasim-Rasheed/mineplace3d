//! OpenGL Shaders
//!
//! This module defines the [`Shader`] and [`ShaderProgram`] structs for managing OpenGL shaders.
//! This module also provides the [`Uniform`] trait for setting uniform variables in shader
//! programs.

use std::sync::Arc;

use glam::{IVec3, Mat4, Vec2, Vec3, Vec4};
use glow::HasContext;

/// Represents an individual OpenGL shader.
pub struct Shader {
    gl: Arc<glow::Context>,
    id: glow::Shader,
    _shader_type: u32,
}

impl Shader {
    /// Compiles a new shader from the given source code.
    pub fn new(gl: &Arc<glow::Context>, shader_type: u32, source: &str) -> Result<Self, String> {
        unsafe {
            let shader = gl.create_shader(shader_type).map_err(|e| e.to_string())?;
            gl.shader_source(shader, source);
            gl.compile_shader(shader);

            if !gl.get_shader_compile_status(shader) {
                let log = gl.get_shader_info_log(shader);
                gl.delete_shader(shader);
                return Err(log);
            }

            Ok(Self {
                gl: Arc::clone(gl),
                id: shader,
                _shader_type: shader_type,
            })
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_shader(self.id);
        }
    }
}

/// Represents a uniform variable in a shader program.
pub trait Uniform {
    /// Sets the value of the uniform variable in the given shader program.
    fn set_uniform(&self, gl: &glow::Context, program: glow::Program, name: &str);
}

impl Uniform for bool {
    fn set_uniform(&self, gl: &glow::Context, program: glow::Program, name: &str) {
        unsafe {
            let location = gl.get_uniform_location(program, name);
            if let Some(loc) = location {
                gl.uniform_1_i32(Some(&loc), *self as i32);
            }
        }
    }
}

impl Uniform for f32 {
    fn set_uniform(&self, gl: &glow::Context, program: glow::Program, name: &str) {
        unsafe {
            let location = gl.get_uniform_location(program, name);
            if let Some(loc) = location {
                gl.uniform_1_f32(Some(&loc), *self);
            }
        }
    }
}

impl Uniform for i32 {
    fn set_uniform(&self, gl: &glow::Context, program: glow::Program, name: &str) {
        unsafe {
            let location = gl.get_uniform_location(program, name);
            if let Some(loc) = location {
                gl.uniform_1_i32(Some(&loc), *self);
            }
        }
    }
}

impl Uniform for Vec2 {
    fn set_uniform(&self, gl: &glow::Context, program: glow::Program, name: &str) {
        unsafe {
            let location = gl.get_uniform_location(program, name);
            if let Some(loc) = location {
                gl.uniform_2_f32(Some(&loc), self.x, self.y);
            }
        }
    }
}

impl Uniform for Vec3 {
    fn set_uniform(&self, gl: &glow::Context, program: glow::Program, name: &str) {
        unsafe {
            let location = gl.get_uniform_location(program, name);
            if let Some(loc) = location {
                gl.uniform_3_f32(Some(&loc), self.x, self.y, self.z);
            }
        }
    }
}

impl Uniform for IVec3 {
    fn set_uniform(&self, gl: &glow::Context, program: glow::Program, name: &str) {
        unsafe {
            let location = gl.get_uniform_location(program, name);
            if let Some(loc) = location {
                gl.uniform_3_i32(Some(&loc), self.x, self.y, self.z);
            }
        }
    }
}

impl Uniform for Vec4 {
    fn set_uniform(&self, gl: &glow::Context, program: glow::Program, name: &str) {
        unsafe {
            let location = gl.get_uniform_location(program, name);
            if let Some(loc) = location {
                gl.uniform_4_f32(Some(&loc), self.x, self.y, self.z, self.w);
            }
        }
    }
}

impl Uniform for Mat4 {
    fn set_uniform(&self, gl: &glow::Context, program: glow::Program, name: &str) {
        unsafe {
            let location = gl.get_uniform_location(program, name);
            if let Some(loc) = location {
                gl.uniform_matrix_4_f32_slice(Some(&loc), false, self.as_ref());
            }
        }
    }
}

impl<const N: usize> Uniform for [Vec3; N] {
    fn set_uniform(&self, gl: &glow::Context, program: glow::Program, name: &str) {
        unsafe {
            let location = gl.get_uniform_location(program, name);
            if let Some(loc) = location {
                let mut data = Vec::with_capacity(N * 3);
                for vec in self.iter() {
                    data.push(vec.x);
                    data.push(vec.y);
                    data.push(vec.z);
                }
                gl.uniform_3_f32_slice(Some(&loc), &data);
            }
        }
    }
}

impl<T: Uniform> Uniform for &T {
    fn set_uniform(&self, gl: &glow::Context, program: glow::Program, name: &str) {
        (*self).set_uniform(gl, program, name);
    }
}

/// Represents an OpenGL shader program composed of multiple shaders.
pub struct ShaderProgram {
    gl: Arc<glow::Context>,
    id: glow::Program,
}

impl ShaderProgram {
    /// Links a new shader program from the given shaders.
    pub fn new(gl: &Arc<glow::Context>, shaders: &[&Shader]) -> Result<Self, String> {
        unsafe {
            let program = gl.create_program().map_err(|e| e.to_string())?;

            for shader in shaders {
                gl.attach_shader(program, shader.id);
            }

            gl.link_program(program);

            if !gl.get_program_link_status(program) {
                let log = gl.get_program_info_log(program);
                gl.delete_program(program);
                return Err(log);
            }

            for shader in shaders {
                gl.detach_shader(program, shader.id);
            }

            Ok(Self {
                gl: Arc::clone(gl),
                id: program,
            })
        }
    }

    /// Binds the shader program for use.
    pub fn use_program(&self) {
        unsafe {
            self.gl.use_program(Some(self.id));
        }
    }

    /// Sets a uniform variable in the shader program.
    pub fn set_uniform<T: Uniform>(&self, name: &str, value: T) {
        value.set_uniform(&self.gl, self.id, name);
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.id);
        }
    }
}
