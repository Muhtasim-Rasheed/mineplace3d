use glam::*;
use glfw::{Action, Context, Key, MouseButton};
use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use crate::game::*;
use crate::shader::ShaderProgramBuilder;
use crate::texture::Texture;
use crate::ui::*;

mod game;
mod mesh;
mod shader;
mod texture;
mod ui;

macro_rules! shader {
    ($folder:literal -> $vert:ident & $frag:ident -> $program:ident) => {
        const $vert: &str = include_str!(concat!("shaders/", $folder, "vertex_shader.glsl"));
        const $frag: &str = include_str!(concat!("shaders/", $folder, "fragment_shader.glsl"));
        let $program = ShaderProgramBuilder::new()
            .attach_shader(shader::ShaderType::Vertex, $vert)
            .attach_shader(shader::ShaderType::Fragment, $frag)
            .build();
    };
}

const WINDOW_WIDTH: u32 = 1600;
const WINDOW_HEIGHT: u32 = 900;

const CHUNK_RADIUS: i32 = game::RENDER_DISTANCE - 1;

const PLACABLE_BLOCKS: [Block; 5] = [
    Block::Grass,
    Block::Dirt,
    Block::Planks,
    Block::Stone,
    Block::Glungus,
];

fn request_chunks_around_player(
    player_pos: Vec3,
    world: &mut World,
    task_sender: &mpsc::Sender<ChunkTask>,
    queued_chunks: &mut HashSet<IVec3>,
) {
    let player_chunk = (player_pos / CHUNK_SIZE as f32).floor().as_ivec3();

    for x in -CHUNK_RADIUS..=CHUNK_RADIUS {
        for y in -CHUNK_RADIUS..=CHUNK_RADIUS {
            for z in -CHUNK_RADIUS..=CHUNK_RADIUS {
                let offset = ivec3(x, y, z);
                let chunk_pos = player_chunk + offset;

                if offset.length_squared() > (CHUNK_RADIUS * CHUNK_RADIUS) {
                    continue;
                }

                if !world.chunk_exists(chunk_pos.x, chunk_pos.y, chunk_pos.z)
                    && !queued_chunks.contains(&chunk_pos)
                {
                    task_sender
                        .send(ChunkTask::Generate {
                            cx: chunk_pos.x,
                            cy: chunk_pos.y,
                            cz: chunk_pos.z,
                            noise: world.noise(),
                            cave_noise: world.cave_noise(),
                            biome_noise: world.biome_noise(),
                        })
                        .unwrap();
                    queued_chunks.insert(chunk_pos);
                }
            }
        }
    }
}

fn main() {
    use glfw::fail_on_errors;
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    let (mut window, events) = glfw
        .create_window(
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            "mineplace3D",
            glfw::WindowMode::Windowed,
        )
        .expect("Failed to create GLFW window.");

    gl::load_with(|symbol| window.get_proc_address(symbol));
    glfw.set_swap_interval(glfw::SwapInterval::Sync(1));
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
        gl::FrontFace(gl::CCW);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    }

    let atlas_image = image::load_from_memory(include_bytes!("assets/atlas.png"))
        .expect("Failed to load texture");

    let mut world = World::new(rand::random::<u32>());

    let mut view;
    let projection = Mat4::perspective_rh(
        90f32.to_radians(),
        WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32,
        0.1,
        200.0,
    );
    let cloud_projection = Mat4::perspective_rh(
        90f32.to_radians(),
        WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32,
        0.1,
        400.0,
    );

    shader!("block/" -> VERT_SHADER & FRAG_SHADER -> shader_program);
    shader!("outline/" -> OUTLINE_VERT_SHADER & OUTLINE_FRAG_SHADER -> outline_shader_program);
    shader!("cloud/" -> CLOUD_VERT_SHADER & CLOUD_FRAG_SHADER -> cloud_shader_program);
    shader!("ui/" -> UI_VERT_SHADER & UI_FRAG_SHADER -> ui_shader_program);

    let (task_sender, task_receiver) = mpsc::channel::<ChunkTask>();
    let (result_sender, result_receiver) = mpsc::channel::<ChunkResult>();
    thread::spawn(move || {
        while let Ok(task) = task_receiver.recv() {
            match task {
                ChunkTask::Generate {
                    cx,
                    cy,
                    cz,
                    noise,
                    cave_noise,
                    biome_noise,
                } => {
                    let chunk = game::Chunk::new(cx, cy, cz, &noise, &cave_noise, &biome_noise);
                    let result = ChunkResult::Generated { cx, cy, cz, chunk };
                    result_sender.send(result).unwrap();
                }
            }
        }
    });
    let mut queued_chunks: HashSet<IVec3> = HashSet::new();

    let font_image =
        image::load_from_memory(include_bytes!("assets/font.png")).expect("Failed to load texture");

    let font = BitmapFont::new(
        font_image.clone(),
        ' ', // first character
        12,  // characters per row
        7,   // character width
        12,  // character height
    );

    let font_image = font_image.to_rgba8();
    let (width, height) = font_image.dimensions();
    let font_texture = Texture::new(width, height, &font_image);

    let atlas_image = atlas_image.to_rgba8();
    let (atlas_width, atlas_height) = atlas_image.dimensions();
    let atlas_texture = Texture::new(atlas_width, atlas_height, &atlas_image);

    window.make_current();
    window.set_key_polling(true);
    window.set_mouse_button_polling(true);
    window.set_scroll_polling(true);

    let mut debug_mesh;
    let cursor = font.build(
        "*",
        WINDOW_WIDTH as f32 / 2.0 - 10.0,
        WINDOW_HEIGHT as f32 / 2.0 - 10.0,
        36.0,
    );
    let outline_mesh = mesh::outline_mesh();
    let ui_projection = Mat4::orthographic_rh_gl(
        0.0,
        WINDOW_WIDTH as f32,
        WINDOW_HEIGHT as f32,
        0.0,
        -3.0,
        3.0,
    );

    let mut keys_down: HashSet<Key> = HashSet::new();
    let mut mouse_down: HashSet<MouseButton> = HashSet::new();

    let mut last_mouse_pos = window.get_cursor_pos();

    let mut last_time = Instant::now();
    let mut duration = Instant::now();
    let mut fps = 1.0 / 0.016;
    let mut grab: bool = false;

    let mut time = 0.0;

    let cloud_plane = game::make_cloud_plane();
    let cloud_texture = game::cloud_texture_gen(UVec2::splat(144), world.seed());

    let mut window_events = Vec::new();

    while !window.should_close() {
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            window_events.push(event);
        }

        for event in &window_events {
            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    grab = !grab;
                }
                glfw::WindowEvent::Key(key, _, Action::Press, _) => {
                    keys_down.insert(*key);
                }
                glfw::WindowEvent::Key(key, _, Action::Release, _) => {
                    keys_down.remove(key);
                }
                glfw::WindowEvent::MouseButton(button, Action::Press, _) => {
                    mouse_down.insert(*button);
                }
                _ => {}
            }
        }

        let player = world.get_player().clone();
        let dt = (Instant::now() - last_time).as_secs_f64().min(0.05);
        last_time = Instant::now();

        if grab {
            window.set_cursor_mode(glfw::CursorMode::Disabled);
        } else {
            window.set_cursor_mode(glfw::CursorMode::Normal);
        }

        if duration.elapsed().as_secs_f32() >= 0.5 {
            fps = 1.0 / dt.max(f64::MIN_POSITIVE);
            duration = Instant::now();
        }
        let text = format!("FPS: {:.2} DT: {:.4}", fps, dt);
        debug_mesh = font.build(&text, 50.0, 50.0, 36.0);
        view = Mat4::look_at_rh(
            player.camera_pos(),
            player.camera_pos() + player.forward,
            player.up,
        );

        request_chunks_around_player(
            player.position,
            &mut world,
            &task_sender,
            &mut queued_chunks,
        );
        queued_chunks
            .retain(|&chunk_pos| !world.chunk_exists(chunk_pos.x, chunk_pos.y, chunk_pos.z));
        while let Ok(ChunkResult::Generated { cx, cy, cz, chunk }) = result_receiver.try_recv() {
            world.add_chunk(cx, cy, cz, chunk);
        }
        world.update(window_events.as_slice(), dt);
        let vp = projection * view;
        world.generate_meshes(vp);

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::DepthMask(gl::TRUE);
            gl::ClearColor(0.6, 0.6, 0.9, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            shader_program.use_program();
            atlas_texture.bind_to_unit(0);
            shader_program.set_uniform("view", view);
            shader_program.set_uniform("projection", projection);
            shader_program.set_uniform("texture_sampler", 0);
            shader_program.set_uniform("textures_per_row", 12);
            shader_program.set_uniform("texture_row_count", 12);
            shader_program.set_uniform("time", time);
            for mesh in &world.meshes {
                mesh.draw();
            }

            if let Some(ref hit) = player.selected_block {
                outline_shader_program.use_program();
                outline_shader_program.set_uniform(
                    "model",
                    Mat4::from_translation(hit.block_pos.as_vec3())
                        * Mat4::from_scale(vec3(1.005, 1.005, 1.005)),
                );
                outline_shader_program.set_uniform("view", view);
                outline_shader_program.set_uniform("projection", projection);
                outline_shader_program.set_uniform("color", vec3(1.0, 1.0, 1.0));
                outline_mesh.draw();
            }

            gl::Disable(gl::CULL_FACE);
            gl::DepthMask(gl::FALSE);
            cloud_shader_program.use_program();
            cloud_shader_program.set_uniform("view", view);
            cloud_shader_program.set_uniform("projection", cloud_projection);
            cloud_shader_program.set_uniform("time", time);
            cloud_texture.bind_to_unit(0);
            cloud_shader_program.set_uniform("cloud_texture", 0);
            cloud_plane.draw();

            gl::Disable(gl::DEPTH_TEST);

            ui_shader_program.use_program();
            font_texture.bind_to_unit(0);
            ui_shader_program.set_uniform("projection", ui_projection);
            ui_shader_program.set_uniform("ui_color", vec4(1.0, 1.0, 1.0, 1.0));
            debug_mesh.draw();
            if grab {
                cursor.draw();
            }
            atlas_texture.bind_to_unit(0);
        }

        window.swap_buffers();

        let dx = window.get_cursor_pos().0 - last_mouse_pos.0;
        let dy = window.get_cursor_pos().1 - last_mouse_pos.1;
        if window.get_cursor_mode() == glfw::CursorMode::Disabled {
            let sensitivity = 0.1;
            world.get_player_mut().yaw += (dx as f32) * sensitivity;
            world.get_player_mut().pitch -= (dy as f32) * sensitivity;
            if world.get_player().pitch > 89.0 {
                world.get_player_mut().pitch = 89.0;
            }
            if world.get_player().pitch < -89.0 {
                world.get_player_mut().pitch = -89.0;
            }

            // Update camera front vector
            let yaw_rad = player.yaw.to_radians();
            let pitch_rad = player.pitch.to_radians();
            world.get_player_mut().forward = vec3(
                yaw_rad.cos() * pitch_rad.cos(),
                pitch_rad.sin(),
                yaw_rad.sin() * pitch_rad.cos(),
            )
            .normalize();
        }

        last_mouse_pos = window.get_cursor_pos();

        window_events.clear();
        time += dt as f32;
    }
}
