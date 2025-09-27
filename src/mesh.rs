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

#[inline]
fn pack_uv(uv: UVec2) -> u64 {
    ((uv.x << 5) | uv.y) as u64
}

#[inline]
fn pack_chunk_local_pos(pos: UVec3) -> u64 {
    ((pos.x << 10) | (pos.y << 5) | pos.z) as u64
}

#[inline]
fn pack_color_rgb677(color: Vec3) -> u64 {
    let r = (color.x * 63.0).round() as u64; // 6 bits
    let g = (color.y * 127.0).round() as u64; // 7 bits
    let b = (color.z * 127.0).round() as u64; // 7 bits
    (r << 14) | (g << 7) | b
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct BlockVertex {
    pub hi: u32,
    pub lo: u32,
}

impl BlockVertex {
    pub fn new(
        position: UVec3,
        normal: u8,
        uv: UVec2,
        block_type: u16,
        foliage: Vec3,
    ) -> Self {
        let uv = pack_uv(uv);
        let pos = pack_chunk_local_pos(position);
        let foliage = pack_color_rgb677(foliage);
        let normal = normal as u64;
        let block_type = block_type as u64;
        let serialized = pos
            | (normal << 15)
            | (uv << 18)
            | (block_type << 28)
            | (foliage << 44);
        BlockVertex {
            hi: (serialized >> 32) as u32,
            lo: (serialized & 0xFFFFFFFF) as u32,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct UIVertex {
    pub position: Vec3,
    pub uv: Vec2,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct OutlineVertex {
    pub position: Vec3,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct CloudPlaneVertex {
    pub position: Vec2,
    pub uv: Vec2,
}

impl VertexFormat for BlockVertex {
    fn setup_attribs() {
        unsafe {
            gl::VertexAttribIPointer(
                0,
                1,
                gl::UNSIGNED_INT,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, hi) as *const _,
            );
            gl::VertexAttribIPointer(
                1,
                1,
                gl::UNSIGNED_INT,
                std::mem::size_of::<Self>() as i32,
                offset_of!(Self, lo) as *const _,
            );

            gl::EnableVertexAttribArray(0);
            gl::EnableVertexAttribArray(1);
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
                std::mem::size_of_val(vertices) as isize,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // Element Buffer
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                std::mem::size_of_val(indices) as isize,
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
