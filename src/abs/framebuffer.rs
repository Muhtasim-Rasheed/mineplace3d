use crate::Texture;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorFormat {
    UnsignedRGBA,
    FloatR,
}

pub struct Framebuffer {
    id: u32,
    texture: Texture,
    depth: Option<Texture>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32, depth: bool, color_format: ColorFormat) -> Self {
        let mut fbo = 0;
        let mut tex = 0;
        let mut depth_texture = None;
        unsafe {
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);

            gl::GenTextures(1, &mut tex);
            gl::BindTexture(gl::TEXTURE_2D, tex);
            match color_format {
                ColorFormat::UnsignedRGBA => {
                    gl::TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        gl::RGBA as i32,
                        width as i32,
                        height as i32,
                        0,
                        gl::RGBA,
                        gl::UNSIGNED_BYTE,
                        std::ptr::null(),
                    );
                }
                ColorFormat::FloatR => {
                    gl::TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        gl::R32F as i32,
                        width as i32,
                        height as i32,
                        0,
                        gl::RED,
                        gl::FLOAT,
                        std::ptr::null(),
                    );
                }
            }
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                tex,
                0,
            );

            if depth {
                let mut depth_tex = 0;
                gl::GenTextures(1, &mut depth_tex);
                gl::BindTexture(gl::TEXTURE_2D, depth_tex);
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::DEPTH_COMPONENT32F as i32, // 32-bit float depth
                    width as i32,
                    height as i32,
                    0,
                    gl::DEPTH_COMPONENT,
                    gl::FLOAT,
                    std::ptr::null(),
                );
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_ATTACHMENT,
                    gl::TEXTURE_2D,
                    depth_tex,
                    0,
                );
                depth_texture = Some(Texture { id: depth_tex });
            }

            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                panic!("Framebuffer is not complete!");
            }

            // Unbind the framebuffer
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        Framebuffer {
            id: fbo,
            texture: Texture { id: tex },
            depth: depth_texture,
        }
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn depth_texture(&self) -> Option<&Texture> {
        self.depth.as_ref()
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.id);
        }
    }

    pub fn unbind() {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.id);
        }
    }
}
