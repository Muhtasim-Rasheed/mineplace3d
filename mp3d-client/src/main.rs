use glam::{Mat4, Vec3};
use glow::HasContext;

use crate::abs::*;

mod abs;
mod render;

struct ExampleVertex {
    position: Vec3,
    color: Vec3,
}

impl ExampleVertex {
    fn new(position: Vec3, color: Vec3) -> Self {
        Self { position, color }
    }
}

impl Vertex for ExampleVertex {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            let stride = std::mem::size_of::<ExampleVertex>() as i32;

            // Position attribute
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,
                3,
                glow::FLOAT,
                false,
                stride,
                0,
            );

            // Color attribute
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                3,
                glow::FLOAT,
                false,
                stride,
                3 * std::mem::size_of::<f32>() as i32,
            );
        }
    }
}

fn main() {
    let mut app = App::new("Mineplace3D", 1280, 720, false);

    // Example mesh
    let vertices = [
        ExampleVertex::new(Vec3::new(-1.0, -1.0, -3.0), Vec3::new(1.0, 0.0, 0.0)),
        ExampleVertex::new(Vec3::new(1.0, -1.0, -3.0), Vec3::new(0.0, 1.0, 0.0)),
        ExampleVertex::new(Vec3::new(0.0, 1.0, -3.0), Vec3::new(0.0, 0.0, 1.0)),
    ];
    let indices = [0u32, 1, 2];
    let mesh = Mesh::new(&app.gl, &vertices, &indices, glow::TRIANGLES);

    // Example shader program
    let vert = Shader::new(&app.gl, glow::VERTEX_SHADER, include_str!("render/shaders/vert.glsl")).unwrap();
    let frag = Shader::new(&app.gl, glow::FRAGMENT_SHADER, include_str!("render/shaders/frag.glsl")).unwrap();
    let shader_program = ShaderProgram::new(&app.gl, &[&vert, &frag]).unwrap();

    // Projection & view matrices
    let mut projection = Mat4::perspective_rh_gl(
        45.0f32.to_radians(),
        1280.0 / 720.0,
        0.1,
        100.0,
    );
    let mut view = Mat4::look_at_rh(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    let mut yaw: f32 = 0.0;
    let mut pitch: f32 = 0.0;
    let mut vel = Vec3::new(0.0, 0.0, 0.0);
    let mut position = Vec3::new(0.0, 0.0, 0.0);
    let mut front = Vec3::new(0.0, 0.0, -1.0);
    
    let mut grabbed = true;

    let mut keys_pressed = std::collections::HashSet::new();

    // Main loop
    'running: loop {
        // Grab or ungrab mouse
        app.sdl.mouse().set_relative_mouse_mode(grabbed);

        // Handle events
        for event in app.event_pump.poll_iter() {
            match event {
                // Does user want to quit?
                sdl2::event::Event::Quit { .. } => break 'running,
                // Did user resize the window?
                sdl2::event::Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(width, height),
                    ..
                } => {
                    unsafe {
                        app.gl.viewport(0, 0, width, height);
                    }
                    projection = glam::Mat4::perspective_rh_gl(
                        45.0f32.to_radians(),
                        width as f32 / height as f32,
                        0.1,
                        100.0,
                    );
                }
                // Did user move?
                sdl2::event::Event::MouseMotion { xrel, yrel, .. } => if grabbed {
                    let sensitivity = 0.1;
                    yaw += (xrel as f32) * sensitivity;
                    pitch -= (yrel as f32) * sensitivity;

                    if pitch > 89.0 {
                        pitch = 89.0;
                    }
                    if pitch < -89.0 {
                        pitch = -89.0;
                    }

                    let yaw_radians = yaw.to_radians();
                    let pitch_radians = pitch.to_radians();
                    front.x = yaw_radians.cos() * pitch_radians.cos();
                    front.y = pitch_radians.sin();
                    front.z = yaw_radians.sin() * pitch_radians.cos();
                    front = front.normalize();
                }
                sdl2::event::Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Escape), .. } => {
                    grabbed = !grabbed;
                }
                sdl2::event::Event::KeyDown { keycode: Some(keycode), .. } => {
                    keys_pressed.insert(keycode);
                }
                sdl2::event::Event::KeyUp { keycode: Some(keycode), .. } => {
                    keys_pressed.remove(&keycode);
                }
                _ => {}
            }
        }

        if keys_pressed.contains(&sdl2::keyboard::Keycode::W) {
            vel += front.with_y(0.0).normalize() * 0.025;
        }
        if keys_pressed.contains(&sdl2::keyboard::Keycode::S) {
            vel -= front.with_y(0.0).normalize() * 0.025;
        }
        if keys_pressed.contains(&sdl2::keyboard::Keycode::A) {
            let right = front.cross(glam::Vec3::new(0.0, 1.0, 0.0)).normalize();
            vel -= right * 0.025;
        }
        if keys_pressed.contains(&sdl2::keyboard::Keycode::D) {
            let right = front.cross(glam::Vec3::new(0.0, 1.0, 0.0)).normalize();
            vel += right * 0.025;
        }
        if keys_pressed.contains(&sdl2::keyboard::Keycode::Space) {
            vel += glam::Vec3::new(0.0, 0.025, 0.0);
        }
        if keys_pressed.contains(&sdl2::keyboard::Keycode::LShift) {
            vel -= glam::Vec3::new(0.0, 0.025, 0.0);
        }
        position += vel;
        vel *= 0.8;

        // Update view matrix with new position
        view = glam::Mat4::look_at_rh(
            position,
            position + front,
            glam::Vec3::new(0.0, 1.0, 0.0),
        );

        unsafe {
            // Clear screen
            app.gl.clear_color(0.1, 0.1, 0.2, 1.0);
            app.gl.clear(glow::COLOR_BUFFER_BIT);

            // Render mesh
            shader_program.use_program();
            shader_program.set_uniform("u_projection", &projection);
            shader_program.set_uniform("u_view", &view);
            mesh.draw();
        }

        // Swap window buffers
        app.window.gl_swap_window();
    }
}
