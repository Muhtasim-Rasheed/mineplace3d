//! SDL2 and OpenGL application management.
//!
//! This module defines the [`App`] struct which encapsulates the SDL2
//! and OpenGL context necessary for creating a windowed application.

use std::sync::Arc;

/// The [`App`] struct encapsulates the SDL2 and OpenGL context.
pub struct App {
    pub sdl: sdl2::Sdl,
    pub video_subsystem: sdl2::VideoSubsystem,
    pub window: sdl2::video::Window,
    pub gl_context: sdl2::video::GLContext,
    pub gl: Arc<glow::Context>,
    pub event_pump: sdl2::EventPump,
}

impl App {
    /// Creates a new [`App`] instance with the specified title, width, and height.
    /// The width and height options are ignored if `fullscreen` is set to `true`.
    pub fn new(title: &str, width: u32, height: u32, fullscreen: bool) -> Self {
        let sdl = sdl2::init().unwrap();
        let video_subsystem = sdl.video().unwrap();
        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(3, 3);
        let display_mode = video_subsystem.current_display_mode(0).unwrap();
        let desktop_width = display_mode.w as u32;
        let desktop_height = display_mode.h as u32;
        let (width, height) = if fullscreen {
            (desktop_width, desktop_height)
        } else {
            (width, height)
        };
        let mut window = video_subsystem
            .window(title, width, height)
            .opengl()
            .resizable()
            .build()
            .unwrap();
        window
            .set_fullscreen(if fullscreen {
                sdl2::video::FullscreenType::Desktop
            } else {
                sdl2::video::FullscreenType::Off
            })
            .unwrap();
        let gl_context = window.gl_create_context().unwrap();
        window.gl_make_current(&gl_context).unwrap();
        let gl = unsafe {
            glow::Context::from_loader_function(|s| {
                video_subsystem.gl_get_proc_address(s) as *const _
            })
        };
        let event_pump = sdl.event_pump().unwrap();
        let gl = Arc::new(gl);

        Self {
            sdl,
            video_subsystem,
            window,
            gl_context,
            gl,
            event_pump,
        }
    }
}
