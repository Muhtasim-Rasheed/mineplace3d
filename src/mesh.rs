use std::marker::PhantomData;

use glam::*;

macro_rules! offset_of {
    ($ty:ty, $field:ident) => {{
        let base = std::ptr::null::<$ty>();
        let field = std::ptr::addr_of!((*base).$field);
        field as usize - base as usize
    }};
}

pub trait VertexFormat {
    fn setup_attribs();
}

#[derive(Clone, Copy)]
pub struct BlockVertex {
    pub position: IVec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub block_type: u32,
    pub foliage: Vec3,
}

#[derive(Clone, Copy)]
pub struct UIVertex {
    pub position: Vec3,
    pub uv: Vec2,
}

#[derive(Clone, Copy)]
pub struct OutlineVertex {
    pub position: Vec3,
}

#[derive(Clone, Copy)]
pub struct CloudPlaneVertex {
    pub position: Vec2,
    pub uv: Vec2,
}

impl VertexFormat for BlockVertex {
    fn setup_attribs() {
        unsafe {
            gl::VertexAttribIPointer(
                0,
                3,
                gl::INT,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, position) as *const _,
            );
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, normal) as *const _,
            );
            gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, uv) as *const _,
            );
            gl::VertexAttribIPointer(
                3,
                1,
                gl::UNSIGNED_INT,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, block_type) as *const _,
            );
            gl::VertexAttribPointer(
                4,
                3,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, foliage) as *const _,
            );

            gl::EnableVertexAttribArray(0);
            gl::EnableVertexAttribArray(1);
            gl::EnableVertexAttribArray(2);
            gl::EnableVertexAttribArray(3);
            gl::EnableVertexAttribArray(4);
        }
    }
}

impl VertexFormat for UIVertex {
    fn setup_attribs() {
        unsafe {
            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, position) as *const _,
            );
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, uv) as *const _,
            );

            gl::EnableVertexAttribArray(0);
            gl::EnableVertexAttribArray(1);
        }
    }
}

impl VertexFormat for OutlineVertex {
    fn setup_attribs() {
        unsafe {
            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, position) as *const _,
            );

            gl::EnableVertexAttribArray(0);
        }
    }
}

impl VertexFormat for CloudPlaneVertex {
    fn setup_attribs() {
        unsafe {
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, position) as *const _,
            );
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, uv) as *const _,
            );

            gl::EnableVertexAttribArray(0);
            gl::EnableVertexAttribArray(1);
        }
    }
}

pub enum DrawMode {
    Triangles = gl::TRIANGLES as isize,
    Lines = gl::LINES as isize,
}

pub struct Mesh<T: VertexFormat> {
    vao: u32,
    vbo: u32,
    ebo: u32,
    draw_mode: u32,
    vertex_count: usize,
    _marker: PhantomData<T>,
}

impl<T: VertexFormat> Mesh<T> {
    pub fn new(vertices: &[T], indices: &[u32], draw_mode: DrawMode) -> Self {
        let mut vao = 0;
        let mut vbo = 0;
        let mut ebo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);

            gl::BindVertexArray(vao);

            // Vertex Buffer
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<T>()) as isize,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // Element Buffer
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * std::mem::size_of::<u32>()) as isize,
                indices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // Setup vertex attributes
            T::setup_attribs();

            // Unbind VAO
            gl::BindVertexArray(0);
        }

        Mesh::<T> {
            vao,
            vbo,
            ebo,
            draw_mode: draw_mode as u32,
            vertex_count: indices.len(),
            _marker: PhantomData,
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawElements(
                self.draw_mode,
                self.vertex_count as i32,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );
            gl::BindVertexArray(0);
        }
    }
}

impl<T: VertexFormat> Drop for Mesh<T> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteBuffers(1, &self.ebo);
        }
    }
}

pub fn outline_mesh() -> Mesh<OutlineVertex> {
    let vertices: [OutlineVertex; 8] = [
        OutlineVertex {
            position: vec3(0.0, 0.0, 0.0),
        },
        OutlineVertex {
            position: vec3(1.0, 0.0, 0.0),
        },
        OutlineVertex {
            position: vec3(1.0, 1.0, 0.0),
        },
        OutlineVertex {
            position: vec3(0.0, 1.0, 0.0),
        },
        OutlineVertex {
            position: vec3(0.0, 0.0, 1.0),
        },
        OutlineVertex {
            position: vec3(1.0, 0.0, 1.0),
        },
        OutlineVertex {
            position: vec3(1.0, 1.0, 1.0),
        },
        OutlineVertex {
            position: vec3(0.0, 1.0, 1.0),
        },
    ];

    let indices: [u32; 24] = [
        0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7,
    ];

    Mesh::new(&vertices, &indices, DrawMode::Lines)
}

pub fn quad_mesh() -> Mesh<UIVertex> {
    let vertices: [UIVertex; 4] = [
        UIVertex {
            position: vec3(-1.0, -1.0, 0.0),
            uv: vec2(0.0, 0.0),
        },
        UIVertex {
            position: vec3(1.0, -1.0, 0.0),
            uv: vec2(1.0, 0.0),
        },
        UIVertex {
            position: vec3(1.0, 1.0, 0.0),
            uv: vec2(1.0, 1.0),
        },
        UIVertex {
            position: vec3(-1.0, 1.0, 0.0),
            uv: vec2(0.0, 1.0),
        },
    ];
    let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
    Mesh::new(&vertices, &indices, DrawMode::Triangles)
}
