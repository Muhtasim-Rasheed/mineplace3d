#![windows_subsystem = "windows"]

use std::{path::PathBuf, rc::Rc, sync::OnceLock};

use glam::{Mat4, Vec2};
use glow::HasContext;

use crate::{
    abs::*,
    render::ui::{uirenderer::UIRenderer, widgets::*},
};

mod abs;
mod client;
mod other;
mod render;
mod resource;
mod scenes;

#[macro_export]
macro_rules! shader_program {
    ($name:ident, $gl:expr, $path_prefix:literal) => {{
        let vert = $crate::abs::Shader::new(
            &$gl,
            glow::VERTEX_SHADER,
            include_str!(concat!(
                $path_prefix,
                "/render/shaders/",
                stringify!($name),
                "/vert.glsl"
            )),
        )
        .unwrap();
        let frag = $crate::abs::Shader::new(
            &$gl,
            glow::FRAGMENT_SHADER,
            include_str!(concat!(
                $path_prefix,
                "/render/shaders/",
                stringify!($name),
                "/frag.glsl"
            )),
        )
        .unwrap();
        ShaderProgram::new(&$gl, &[&vert, &frag]).unwrap()
    }};
}

pub static ASSETS: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/src/assets");

static GAME_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn get_game_dir() -> &'static PathBuf {
    GAME_DIR.get_or_init(|| {
        let dir = std::env::var_os("MINEPLACE3D_GAME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::data_dir()
                    .unwrap_or_else(|| std::env::current_dir().unwrap())
                    .join("mineplace3d")
            });
        if !dir.exists() {
            std::fs::create_dir_all(&dir).expect("Failed to create game directory");
        }
        dir
    })
}

pub fn get_saves_dir() -> PathBuf {
    let saves_dir = get_game_dir().join("saves");
    if !saves_dir.exists() {
        std::fs::create_dir_all(&saves_dir).expect("Failed to create saves directory");
    }
    saves_dir
}

pub fn get_config_path() -> PathBuf {
    get_game_dir().join("config.json")
}

fn main() {
    let mut app = App::new("Mineplace3D", 1280, 720, false);

    unsafe {
        app.gl.enable(glow::DEPTH_TEST);
        app.gl.enable(glow::CULL_FACE);
        app.gl.cull_face(glow::BACK);
        app.gl.front_face(glow::CCW);
        app.gl.enable(glow::BLEND);
        app.gl
            .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
    }

    let shader_program = shader_program!(ui, app.gl, ".");

    let mut keyboard_state = other::KeyboardState::default();
    let mut mouse_state = other::MouseState::default();

    let mut ui_renderer = UIRenderer::new(
        &app.gl,
        shader_program,
        Mat4::orthographic_rh_gl(0.0, 1280.0, 720.0, 0.0, -20.0, 20.0),
    );

    let font = Rc::new(Font::new(
        Texture::new(
            &app.gl,
            &image::load_from_memory_with_format(
                ASSETS.get_file("font.png").unwrap().contents(),
                image::ImageFormat::Png,
            )
            .unwrap(),
        ),
        glam::Vec2::new(7.0, 12.0),
        ' ',
    ));

    let gui_tex = Texture::new(
        &app.gl,
        &image::load_from_memory_with_format(
            ASSETS.get_file("gui.png").unwrap().contents(),
            image::ImageFormat::Png,
        )
        .unwrap(),
    );

    let assets = scenes::Assets::load(&app.gl).unwrap_or_else(|e| {
        panic!("Failed to load assets: {}", e);
    });

    let config = scenes::options::ClientConfig::load();

    let mut scene_manager = scenes::SceneManager::new(
        Box::new(scenes::titlescreen::TitleScreen::new(
            &font,
            gui_tex.handle(),
            (1280, 720),
        )),
        assets,
        config,
    );

    let mut last_frame_time = std::time::Instant::now();

    'running: loop {
        let now = std::time::Instant::now();
        let delta_time = now.duration_since(last_frame_time).as_secs_f32();
        last_frame_time = now;

        mouse_state.delta = Vec2::ZERO;
        mouse_state.scroll_delta = Vec2::ZERO;
        keyboard_state.repeated.clear();
        keyboard_state.pressed.clear();
        keyboard_state.released.clear();
        keyboard_state.text_input.clear();
        mouse_state.pressed.clear();
        mouse_state.released.clear();

        for event in app.event_pump.poll_iter() {
            scene_manager.handle_event(&app.gl, &event);
            match event {
                sdl2::event::Event::Quit { .. } => break 'running,
                sdl2::event::Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(width, height),
                    ..
                } => {
                    unsafe {
                        app.gl.viewport(0, 0, width, height);
                    }
                    ui_renderer.projection_matrix = Mat4::orthographic_rh_gl(
                        0.0,
                        width as f32,
                        height as f32,
                        0.0,
                        -20.0,
                        20.0,
                    );
                }
                sdl2::event::Event::MouseMotion {
                    x, y, xrel, yrel, ..
                } => {
                    mouse_state.position = Vec2::new(x as f32, y as f32);
                    mouse_state.delta = Vec2::new(xrel as f32, yrel as f32);
                }
                sdl2::event::Event::MouseWheel { x, y, .. } => {
                    mouse_state.scroll_delta = Vec2::new(x as f32, y as f32);
                }
                sdl2::event::Event::MouseButtonDown { mouse_btn, .. } => {
                    mouse_state.down.insert(mouse_btn);
                    mouse_state.pressed.insert(mouse_btn);
                }
                sdl2::event::Event::MouseButtonUp { mouse_btn, .. } => {
                    mouse_state.down.remove(&mouse_btn);
                    mouse_state.released.insert(mouse_btn);
                }
                sdl2::event::Event::KeyDown {
                    keycode: Some(keycode),
                    repeat,
                    ..
                } => {
                    if repeat {
                        keyboard_state.repeated.insert(keycode);
                    } else {
                        keyboard_state.repeated.insert(keycode);
                        keyboard_state.down.insert(keycode);
                    }
                    keyboard_state.pressed.insert(keycode);
                }
                sdl2::event::Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => {
                    keyboard_state.down.remove(&keycode);
                    keyboard_state.released.insert(keycode);
                }
                sdl2::event::Event::TextInput { text, .. } => {
                    keyboard_state.text_input = text;
                }
                _ => {}
            }
        }

        let update_ctx = other::UpdateContext::new(&keyboard_state, &mouse_state, delta_time);
        if !scene_manager.update(&app.gl, &update_ctx, &mut app.window, &app.sdl) {
            break 'running;
        }

        scene_manager.render(&app.gl, &mut ui_renderer);
        app.window.gl_swap_window();
    }
}
