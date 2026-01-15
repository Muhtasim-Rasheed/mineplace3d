use std::rc::Rc;

use glam::{Mat4, Vec2, Vec3, Vec4};
use glow::HasContext;

use crate::{
    abs::*,
    render::ui::{uirenderer::UIRenderer, widgets::*},
};

mod abs;
mod render;

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

    let vert = Shader::new(
        &app.gl,
        glow::VERTEX_SHADER,
        include_str!("render/shaders/ui/vert.glsl"),
    )
    .unwrap();
    let frag = Shader::new(
        &app.gl,
        glow::FRAGMENT_SHADER,
        include_str!("render/shaders/ui/frag.glsl"),
    )
    .unwrap();
    let shader_program = ShaderProgram::new(&app.gl, &[&vert, &frag]).unwrap();

    let mut keyboard_state = KeyboardState::default();
    let mut mouse_state = MouseState::default();

    let mut ui_renderer = UIRenderer::new(
        &app.gl,
        shader_program,
        Mat4::orthographic_rh_gl(0.0, 1280.0, 720.0, 0.0, -1.0, 1.0),
    );

    let font = Rc::new(Font::new(
        Texture::new(
            &app.gl,
            &image::load_from_memory_with_format(
                include_bytes!("assets/font.png"),
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
            include_bytes!("assets/gui.png"),
            image::ImageFormat::Png,
        )
        .unwrap(),
    );

    let label = Label::new(
        "Mineplace3D".to_string(),
        72.0,
        Vec4::new(1.0, 1.0, 1.0, 1.0),
        &font,
    );

    let another_label = Label::new(
        "testingtestingtesting".to_string(),
        48.0,
        Vec4::new(0.0, 1.0, 0.0, 1.0),
        &font,
    );

    let button = Button::new(
        "gnitsetgnitsetgnitset",
        Vec4::new(1.0, 1.0, 1.0, 1.0),
        32.0,
        Vec2::new(500.0, 100.0),
        &font,
        gui_tex.handle(),
    );

    let mut container = Column::new(10.0, crate::render::ui::widgets::Alignment::Center, 10.0);
    container.add_widget(label);
    container.add_widget(another_label);
    container.add_widget(button);

    let mut last_frame_time = std::time::Instant::now();

    'running: loop {
        let now = std::time::Instant::now();
        let delta_time = now.duration_since(last_frame_time).as_secs_f32();
        last_frame_time = now;

        mouse_state.delta = Vec3::ZERO.truncate();
        mouse_state.scroll_delta = Vec3::ZERO.truncate();
        keyboard_state.pressed.clear();
        keyboard_state.released.clear();
        mouse_state.pressed.clear();
        mouse_state.released.clear();

        for event in app.event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit { .. } => break 'running,
                sdl2::event::Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(width, height),
                    ..
                } => {
                    unsafe {
                        app.gl.viewport(0, 0, width, height);
                    }
                    ui_renderer.projection_matrix =
                        Mat4::orthographic_rh_gl(0.0, width as f32, height as f32, 0.0, -1.0, 1.0);
                }
                sdl2::event::Event::MouseMotion {
                    x, y, xrel, yrel, ..
                } => {
                    mouse_state.position = Vec3::new(x as f32, y as f32, 0.0).truncate();
                    mouse_state.delta = Vec3::new(xrel as f32, yrel as f32, 0.0).truncate();
                }
                sdl2::event::Event::MouseWheel { x, y, .. } => {
                    mouse_state.scroll_delta = Vec3::new(x as f32, y as f32, 0.0).truncate();
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
                    repeat: false,
                    ..
                } => {
                    keyboard_state.down.insert(keycode);
                    keyboard_state.pressed.insert(keycode);
                }
                sdl2::event::Event::KeyUp {
                    keycode: Some(keycode),
                    repeat: false,
                    ..
                } => {
                    keyboard_state.down.remove(&keycode);
                    keyboard_state.released.insert(keycode);
                }
                _ => {}
            }
        }

        let update_ctx = UpdateContext::new(&keyboard_state, &mouse_state, delta_time);

        container.update(&update_ctx);

        if container
            .get_widget::<Button>(2)
            .map_or(false, |btn| btn.is_pressed())
        {
            println!("woah");
        }

        container.layout(&LayoutContext {
            max_size: Vec2::new(app.window.size().0 as f32, app.window.size().1 as f32),
            cursor: Vec3::ZERO.truncate(),
        });

        unsafe {
            app.gl.clear_color(0.1, 0.1, 0.2, 1.0);
            app.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            app.gl.disable(glow::DEPTH_TEST);
            container.draw(&mut ui_renderer);
            app.gl.enable(glow::DEPTH_TEST);
        }

        app.window.gl_swap_window();
    }
}

// fn main() {
//     let mut app = App::new("Mineplace3D", 1280, 720, false);

//     unsafe {
//         app.gl.enable(glow::DEPTH_TEST);
//         app.gl.enable(glow::CULL_FACE);
//         app.gl.cull_face(glow::BACK);
//         app.gl.front_face(glow::CCW);
//         app.gl.enable(glow::BLEND);
//         app.gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
//     }

//     let world = mp3d_core::world::World::new();

//     // Example meshes
//     let meshes = render::meshing::mesh_world(&app.gl, &world);

//     // Example shader program
//     let vert = Shader::new(
//         &app.gl,
//         glow::VERTEX_SHADER,
//         include_str!("render/shaders/chunk/vert.glsl"),
//     )
//     .unwrap();
//     let frag = Shader::new(
//         &app.gl,
//         glow::FRAGMENT_SHADER,
//         include_str!("render/shaders/chunk/frag.glsl"),
//     )
//     .unwrap();
//     let shader_program = ShaderProgram::new(&app.gl, &[&vert, &frag]).unwrap();

//     // Projection & view matrices
//     let mut projection = Mat4::perspective_rh_gl(90.0f32.to_radians(), 1280.0 / 720.0, 0.1, 400.0);
//     let mut view;

//     let mut yaw: f32 = 0.0;
//     let mut pitch: f32 = 0.0;
//     let mut vel = Vec3::new(0.0, 0.0, 0.0);
//     let mut position = Vec3::new(0.0, 0.0, 0.0);
//     let mut front = Vec3::new(0.0, 0.0, -1.0);

//     let mut grabbed = true;

//     let mut keys_pressed = std::collections::HashSet::new();

//     // Main loop
//     'running: loop {
//         // Grab or ungrab mouse
//         app.sdl.mouse().set_relative_mouse_mode(grabbed);

//         // Handle events
//         for event in app.event_pump.poll_iter() {
//             match event {
//                 sdl2::event::Event::Quit { .. } => break 'running,
//                 sdl2::event::Event::Window {
//                     win_event: sdl2::event::WindowEvent::Resized(width, height),
//                     ..
//                 } => {
//                     unsafe {
//                         app.gl.viewport(0, 0, width, height);
//                     }
//                     projection = glam::Mat4::perspective_rh_gl(
//                         90.0f32.to_radians(),
//                         width as f32 / height as f32,
//                         0.1,
//                         400.0,
//                     );
//                 }
//                 sdl2::event::Event::MouseMotion { xrel, yrel, .. } => {
//                     if grabbed {
//                         let sensitivity = 0.1;
//                         yaw += (xrel as f32) * sensitivity;
//                         pitch -= (yrel as f32) * sensitivity;

//                         if pitch > 89.0 {
//                             pitch = 89.0;
//                         }
//                         if pitch < -89.0 {
//                             pitch = -89.0;
//                         }

//                         let yaw_radians = yaw.to_radians();
//                         let pitch_radians = pitch.to_radians();
//                         front.x = yaw_radians.cos() * pitch_radians.cos();
//                         front.y = pitch_radians.sin();
//                         front.z = yaw_radians.sin() * pitch_radians.cos();
//                         front = front.normalize();
//                     }
//                 }
//                 sdl2::event::Event::KeyDown {
//                     keycode: Some(sdl2::keyboard::Keycode::Escape),
//                     ..
//                 } => {
//                     grabbed = !grabbed;
//                 }
//                 sdl2::event::Event::KeyDown {
//                     keycode: Some(keycode),
//                     ..
//                 } => {
//                     keys_pressed.insert(keycode);
//                 }
//                 sdl2::event::Event::KeyUp {
//                     keycode: Some(keycode),
//                     ..
//                 } => {
//                     keys_pressed.remove(&keycode);
//                 }
//                 _ => {}
//             }
//         }

//         if keys_pressed.contains(&sdl2::keyboard::Keycode::W) {
//             vel += front.with_y(0.0).normalize()
//                 * if keys_pressed.contains(&sdl2::keyboard::Keycode::LCtrl)
//                     || keys_pressed.contains(&sdl2::keyboard::Keycode::Q)
//                 {
//                     0.12
//                 } else {
//                     0.06
//                 };
//         }
//         if keys_pressed.contains(&sdl2::keyboard::Keycode::S) {
//             vel -= front.with_y(0.0).normalize() * 0.06;
//         }
//         if keys_pressed.contains(&sdl2::keyboard::Keycode::A) {
//             let right = front.cross(glam::Vec3::new(0.0, 1.0, 0.0)).normalize();
//             vel -= right * 0.06;
//         }
//         if keys_pressed.contains(&sdl2::keyboard::Keycode::D) {
//             let right = front.cross(glam::Vec3::new(0.0, 1.0, 0.0)).normalize();
//             vel += right * 0.06;
//         }
//         if keys_pressed.contains(&sdl2::keyboard::Keycode::Space) {
//             vel += glam::Vec3::new(0.0, 0.06, 0.0);
//         }
//         if keys_pressed.contains(&sdl2::keyboard::Keycode::LShift) {
//             vel -= glam::Vec3::new(0.0, 0.06, 0.0);
//         }
//         position += vel;
//         vel *= 0.8;

//         // Update view matrix with new position
//         view = glam::Mat4::look_at_rh(position, position + front, glam::Vec3::new(0.0, 1.0, 0.0));

//         unsafe {
//             // Clear screen
//             app.gl.clear_color(0.1, 0.1, 0.2, 1.0);
//             app.gl
//                 .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

//             // Render mesh
//             shader_program.use_program();
//             shader_program.set_uniform("u_projection", projection);
//             shader_program.set_uniform("u_view", view);
//             for mesh in meshes.values() {
//                 mesh.draw();
//             }
//         }

//         // Swap window buffers
//         app.window.gl_swap_window();
//     }
// }
