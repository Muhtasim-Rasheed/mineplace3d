use std::collections::HashMap;

use glam::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderType {
    Vertex = gl::VERTEX_SHADER as isize,
    Fragment = gl::FRAGMENT_SHADER as isize,
}

pub struct Shader {
    id: u32,
}

impl Shader {
    pub fn new(shader_type: ShaderType, source: &str) -> Self {
        let id = unsafe {
            let shader = gl::CreateShader(shader_type as u32);
            let c_str = std::ffi::CString::new(source).unwrap();
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
            gl::CompileShader(shader);

            let mut success = 0;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut v = Vec::<u8>::with_capacity(1024);
                let mut log_len = 0;
                gl::GetShaderInfoLog(
                    shader,
                    v.capacity() as i32,
                    &mut log_len,
                    v.as_mut_ptr().cast(),
                );
                v.set_len(log_len as usize);
                panic!("Shader compilation failed: {}", String::from_utf8_lossy(&v));
            }
            shader
        };

        Shader { id }
    }
}

pub struct ShaderProgramBuilder {
    shaders: HashMap<ShaderType, Shader>,
}

impl ShaderProgramBuilder {
    pub fn new() -> Self {
        ShaderProgramBuilder {
            shaders: HashMap::new(),
        }
    }

    pub fn attach_shader(mut self, shader_type: ShaderType, source: &str) -> Self {
        let shader = Shader::new(shader_type, source);
        self.shaders.insert(shader_type, shader);
        self
    }

    pub fn build(self) -> ShaderProgram {
        let id = unsafe {
            let program = gl::CreateProgram();
            for shader in self.shaders.values() {
                gl::AttachShader(program, shader.id);
            }
            gl::LinkProgram(program);
            for shader in self.shaders.values() {
                gl::DetachShader(program, shader.id);
            }
            program
        };

        ShaderProgram { id }
    }
}

pub trait UniformValue {
    fn set_uniform(&self, program_id: u32, name: &str);
}

impl UniformValue for i32 {
    fn set_uniform(&self, program_id: u32, name: &str) {
        let c_str = std::ffi::CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(program_id, c_str.as_ptr());
            gl::Uniform1i(location, *self);
        }
    }
}

impl UniformValue for f32 {
    fn set_uniform(&self, program_id: u32, name: &str) {
        let c_str = std::ffi::CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(program_id, c_str.as_ptr());
            gl::Uniform1f(location, *self);
        }
    }
}

impl UniformValue for Vec2 {
    fn set_uniform(&self, program_id: u32, name: &str) {
        let c_str = std::ffi::CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(program_id, c_str.as_ptr());
            gl::Uniform2f(location, self.x, self.y);
        }
    }
}

impl UniformValue for Vec3 {
    fn set_uniform(&self, program_id: u32, name: &str) {
        let c_str = std::ffi::CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(program_id, c_str.as_ptr());
            gl::Uniform3f(location, self.x, self.y, self.z);
        }
    }
}

impl UniformValue for Vec4 {
    fn set_uniform(&self, program_id: u32, name: &str) {
        let c_str = std::ffi::CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(program_id, c_str.as_ptr());
            gl::Uniform4f(location, self.x, self.y, self.z, self.w);
        }
    }
}

impl UniformValue for Mat4 {
    fn set_uniform(&self, program_id: u32, name: &str) {
        let c_str = std::ffi::CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(program_id, c_str.as_ptr());
            gl::UniformMatrix4fv(location, 1, gl::FALSE, self.to_cols_array().as_ptr());
        }
    }
}

impl UniformValue for bool {
    fn set_uniform(&self, program_id: u32, name: &str) {
        let c_str = std::ffi::CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(program_id, c_str.as_ptr());
            gl::Uniform1i(location, if *self { gl::TRUE } else { gl::FALSE } as i32);
        }
    }
}

impl<const N: usize> UniformValue for [Vec3; N] {
    fn set_uniform(&self, program_id: u32, name: &str) {
        #[repr(C, packed)]
        struct Vec3Packed {
            x: f32,
            y: f32,
            z: f32,
        }

        impl From<Vec3> for Vec3Packed {
            fn from(v: Vec3) -> Self {
                Vec3Packed {
                    x: v.x,
                    y: v.y,
                    z: v.z,
                }
            }
        }

        let self_packed: [Vec3Packed; N] = self.map(|v| v.into());

        let c_str = std::ffi::CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(program_id, c_str.as_ptr());
            gl::Uniform3fv(location, N as i32, self_packed.as_ptr() as *const f32);
        }
    }
}

impl UniformValue for IVec3 {
    fn set_uniform(&self, program_id: u32, name: &str) {
        let c_str = std::ffi::CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(program_id, c_str.as_ptr());
            gl::Uniform3i(location, self.x, self.y, self.z);
        }
    }
}

impl<T: UniformValue> UniformValue for &T {
    fn set_uniform(&self, program_id: u32, name: &str) {
        (*self).set_uniform(program_id, name);
    }
}

pub struct ShaderProgram {
    id: u32,
}

impl ShaderProgram {
    pub fn set_uniform<T: UniformValue>(&self, name: &str, value: T) {
        value.set_uniform(self.id, name);
    }

    pub fn use_program(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}
