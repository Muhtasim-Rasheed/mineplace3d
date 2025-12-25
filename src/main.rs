use glam::*;
use glow::HasContext;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use crate::abs::*;
use crate::asset::ResourceManager;
use crate::game::*;
use crate::shader::{Shader, ShaderProgram};
use crate::texture::Texture;
use crate::ui::*;

mod abs;
mod game;

macro_rules! shader {
    ($gl:expr, $folder:literal -> $vert:ident & $frag:ident -> $program:ident) => {
        let $vert: Shader = Shader::new(
            &$gl,
            glow::VERTEX_SHADER,
            include_str!(concat!("shaders/", $folder, "vertex_shader.glsl")),
        )
        .unwrap();
        let $frag: Shader = Shader::new(
            &$gl,
            glow::FRAGMENT_SHADER,
            include_str!(concat!("shaders/", $folder, "fragment_shader.glsl")),
        )
        .unwrap();
        let $program =
            ShaderProgram::new(&$gl, &[&$vert, &$frag]).expect("Failed to create shader program");
    };
}

const TRANSLATIONS_JSON: &str = include_str!("assets/translations.json");
const MODEL_DEF_JSON: &str = include_str!("assets/models.json");

fn mid<T>(v: &[T]) -> usize {
    if v.len().is_multiple_of(2) {
        v.len() / 2
    } else {
        v.len() / 2 - 1
    }
}

fn shift_vec<T: Clone>(v: &[T], index: usize) -> Vec<T> {
    let shift = (v.len() + index - mid(v)) % v.len();
    v.iter()
        .cycle()
        .skip(shift)
        .take(v.len())
        .cloned()
        .collect()
}

fn key_to_char(key: Keycode) -> Option<char> {
    match key {
        Keycode::A => Some('a'),
        Keycode::B => Some('b'),
        Keycode::C => Some('c'),
        Keycode::D => Some('d'),
        Keycode::E => Some('e'),
        Keycode::F => Some('f'),
        Keycode::G => Some('g'),
        Keycode::H => Some('h'),
        Keycode::I => Some('i'),
        Keycode::J => Some('j'),
        Keycode::K => Some('k'),
        Keycode::L => Some('l'),
        Keycode::M => Some('m'),
        Keycode::N => Some('n'),
        Keycode::O => Some('o'),
        Keycode::P => Some('p'),
        Keycode::Q => Some('q'),
        Keycode::R => Some('r'),
        Keycode::S => Some('s'),
        Keycode::T => Some('t'),
        Keycode::U => Some('u'),
        Keycode::V => Some('v'),
        Keycode::W => Some('w'),
        Keycode::X => Some('x'),
        Keycode::Y => Some('y'),
        Keycode::Z => Some('z'),
        Keycode::Space => Some(' '),
        Keycode::Quote => Some('\''),
        Keycode::Comma => Some(','),
        Keycode::Minus => Some('-'),
        Keycode::Period => Some('.'),
        Keycode::Slash => Some('/'),
        Keycode::Semicolon => Some(';'),
        Keycode::Equals => Some('='),
        Keycode::LeftBracket => Some('['),
        Keycode::Backslash => Some('\\'),
        Keycode::RightBracket => Some(']'),
        Keycode::Num0 => Some('0'),
        Keycode::Num1 => Some('1'),
        Keycode::Num2 => Some('2'),
        Keycode::Num3 => Some('3'),
        Keycode::Num4 => Some('4'),
        Keycode::Num5 => Some('5'),
        Keycode::Num6 => Some('6'),
        Keycode::Num7 => Some('7'),
        Keycode::Num8 => Some('8'),
        Keycode::Num9 => Some('9'),
        _ => None,
    }
}

fn main() {
    let mut app = App::new("Mineplace3D", 1280, 720, true);

    let font_image =
        image::load_from_memory(include_bytes!("assets/font.png")).expect("Failed to load texture");

    let font = BitmapFont::new(
        font_image, ' ', // first character
        12,  // characters per row
        7,   // character width
        12,  // character height
    );

    game(rand::random(), &mut app, &font);
}

fn game(seed: i32, app: &mut App, font: &BitmapFont) {
    unsafe {
        app.gl.enable(glow::DEPTH_TEST);
        app.gl.enable(glow::CULL_FACE);
        app.gl.cull_face(glow::BACK);
        app.gl.front_face(glow::CCW);
        app.gl.enable(glow::BLEND);
        app.gl
            .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
    }

    let atlas_image = image::load_from_memory(include_bytes!("assets/atlas.png"))
        .expect("Failed to load texture");
    let billboard_atlas_image =
        image::load_from_memory(include_bytes!("assets/billboard_atlas.png"))
            .expect("Failed to load texture");

    let mut view;

    shader!(app.gl, "block/" -> vert_shader & frag_shader -> shader_program);
    shader!(app.gl, "outline/" -> outline_vert_shader & outline_frag_shader -> outline_shader_program);
    shader!(app.gl, "billboard/" -> billboard_vert_shader & billboard_frag_shader -> billboard_shader_program);
    shader!(app.gl, "cloud/" -> cloud_vert_shader & cloud_frag_shader -> cloud_shader_program);
    shader!(app.gl, "ssao/" -> ssao_vert_shader & ssao_frag_shader -> ssao_shader_program);
    shader!(app.gl, "postprocessing/" -> postprocessing_vert_shader & postprocessing_frag_shader -> postprocessing_shader_program);
    shader!(app.gl, "ui/" -> ui_vert_shader & ui_frag_shader -> ui_shader_program);

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
                    let (chunk, outside_blocks) =
                        game::Chunk::new(cx, cy, cz, &noise, &cave_noise, &biome_noise);
                    let result = ChunkResult::Generated {
                        cx,
                        cy,
                        cz,
                        chunk,
                        outside_blocks,
                    };
                    result_sender.send(result).unwrap();
                }
            }
        }
    });
    let mut queued_chunks: HashSet<IVec3> = HashSet::new();

    let atlas_image = atlas_image.to_rgba8();
    let atlas_texture = Texture::new(&app.gl, &atlas_image.into());

    let billboard_atlas_image = billboard_atlas_image.to_rgba8();
    let billboard_atlas_texture = Texture::new(&app.gl, &billboard_atlas_image.into());

    let mut debug_mesh;
    let mut chat_mesh = font.build(&app.gl, "", 50.0, app.window.size().1 as f32 - 150.0, 24.0);
    let mut chat_hist_mesh;
    let mut cursor = font.build(
        &app.gl,
        "*",
        app.window.size().0 as f32 / 2.0 - 10.0,
        app.window.size().1 as f32 / 2.0 - 10.0,
        36.0,
    );
    let outline_mesh = outline_mesh(&app.gl);
    let mut ui_projection = Mat4::orthographic_rh_gl(
        0.0,
        app.window.size().0 as f32,
        app.window.size().1 as f32,
        0.0,
        -3.0,
        3.0,
    );

    let mut keys_down: HashSet<Keycode> = HashSet::new();
    let mut mouse_down: HashSet<MouseButton> = HashSet::new();

    let mut last_time = Instant::now();
    let mut duration = Instant::now();
    let mut fps = 1.0 / 0.016;
    let mut grab: bool = false;

    let mut time = 0.0;

    let framebuffer = Framebuffer::new(
        &app.gl,
        app.window.size().0 as i32,
        app.window.size().1 as i32,
        true,
        ColorUsage::All,
    );
    framebuffer.bind();
    unsafe {
        app.gl
            .viewport(0, 0, app.window.size().0 as i32, app.window.size().1 as i32);
    }
    Framebuffer::unbind(&app.gl);
    let ssao_framebuffer = Framebuffer::new(
        &app.gl,
        app.window.size().0 as i32,
        app.window.size().1 as i32,
        false,
        ColorUsage::RedFloat,
    );
    ssao_framebuffer.bind();
    unsafe {
        app.gl
            .viewport(0, 0, app.window.size().0 as i32, app.window.size().1 as i32);
    }
    Framebuffer::unbind(&app.gl);
    let mut ssao_samples = [vec3(0.0, 0.0, 0.0); 32];
    for (i, sample) in ssao_samples.iter_mut().enumerate() {
        let scale = i as f32 / 32.0;
        let mut sample_ = vec3(
            rand::random::<f32>() * 2.0 - 1.0,
            rand::random::<f32>() * 2.0 - 1.0,
            rand::random::<f32>(),
        );
        sample_ = sample_.normalize() * rand::random::<f32>();
        let lerp = 0.1 + 0.9 * scale * scale;
        *sample = sample_ * lerp;
    }
    let mut ssao_noise_data = [0u8; 4 * 16];
    for i in 0..16 {
        let x = rand::random::<f32>() * 2.0 - 1.0;
        let y = rand::random::<f32>() * 2.0 - 1.0;
        ssao_noise_data[i * 4] = ((x * 0.5 + 0.5) * 255.0) as u8;
        ssao_noise_data[i * 4 + 1] = ((y * 0.5 + 0.5) * 255.0) as u8;
        ssao_noise_data[i * 4 + 2] = 0;
        ssao_noise_data[i * 4 + 3] = 0;
    }
    let ssao_noise_texture = Texture::new_from_data(&app.gl, 4, 4, ssao_noise_data.as_slice());

    let cloud_plane = game::make_cloud_plane(&app.gl);
    let cloud_texture = game::cloud_texture_gen(&app.gl, UVec2::splat(144), seed);

    let mut window_events = Vec::new();

    let mut command: Option<String> = None;
    let mut chat_hist: Vec<String> = vec![
        "Welcome to Mineplace3D!".to_string(),
        "Type /help for a list of commands.".to_string(),
    ];
    let mut chat_open = false;
    let mut show_ui = true;

    let mut vsync = true;

    let translations =
        asset::Translations::new(TRANSLATIONS_JSON).expect("Failed to load translations");

    let model_defs =
        asset::ModelDefs::new(MODEL_DEF_JSON).expect("Failed to load model definitions");

    let resource_mgr = ResourceManager::new()
        .add("atlas", atlas_texture)
        .add("font", Texture::new(&app.gl, &font.atlas))
        .add("cloud", cloud_texture)
        .add("billboard_atlas", billboard_atlas_texture)
        .add("block_shader", shader_program)
        .add("outline_shader", outline_shader_program)
        .add("billboard_shader", billboard_shader_program)
        .add("cloud_shader", cloud_shader_program)
        .add("ssao_shader", ssao_shader_program)
        .add("postprocessing_shader", postprocessing_shader_program)
        .add("ui_shader", ui_shader_program)
        .add("translations", translations)
        .add("model_defs", model_defs);

    let mut world = World::new(seed, resource_mgr, &app.window);

    'running: loop {
        if vsync {
            app.video_subsystem
                .gl_set_swap_interval(sdl2::video::SwapInterval::VSync)
                .unwrap();
        } else {
            app.video_subsystem
                .gl_set_swap_interval(sdl2::video::SwapInterval::Immediate)
                .unwrap();
        }

        for event in app.event_pump.poll_iter() {
            if matches!(event, sdl2::event::Event::Quit { .. }) {
                break 'running;
            }
            window_events.push(event);
        }

        for event in &window_events {
            match event {
                sdl2::event::Event::KeyDown {
                    keycode: Some(Keycode::F1),
                    ..
                } => {
                    show_ui = !show_ui;
                }
                sdl2::event::Event::KeyDown {
                    keycode: Some(Keycode::F2),
                    ..
                } => {
                    chat_hist.clear();
                }
                sdl2::event::Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    if !chat_open {
                        grab = !grab;
                    } else {
                        chat_open = false;
                        grab = true;
                    }
                }
                sdl2::event::Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if !chat_open {
                        keys_down.insert(*key);
                    }
                    if *key == Keycode::Slash && !chat_open {
                        chat_open = true;
                        command = Some("/".to_string());
                        grab = false;
                    } else if *key == Keycode::T && !chat_open {
                        chat_open = true;
                        command = Some("".to_string());
                        grab = false;
                    } else if *key == Keycode::Return && chat_open {
                        if let Some(cmd) = command.take()
                            && cmd.starts_with('/')
                        {
                            let parts: Vec<&str> = cmd[1..].split_whitespace().collect();
                            match parts.first().copied() {
                                Some("help") => {
                                    chat_hist.push("Available commands.".to_string());
                                    chat_hist.push("/help - Show this message.".to_string());
                                    chat_hist.push("/seed - Show the world seed.".to_string());
                                    chat_hist.push(
                                        "/tp <x> <y> <z> - Teleport to coordinates.".to_string(),
                                    );
                                    chat_hist.push("/vsync <on|off> - Toggle VSync.".to_string());
                                    chat_hist.push(
                                        "/fov <degrees> - Set the field of view.".to_string(),
                                    );
                                }
                                Some("seed") => {
                                    chat_hist.push(format!("Current world seed: {}", world.seed()));
                                }
                                Some("tp") => {
                                    if parts.len() != 4 {
                                        chat_hist.push("Usage: /tp <x> <y> <z>".to_string());
                                    } else {
                                        let x = parts[1].parse::<f32>();
                                        let y = parts[2].parse::<f32>();
                                        let z = parts[3].parse::<f32>();
                                        if x.is_err() || y.is_err() || z.is_err() {
                                            chat_hist.push("Invalid coordinates.".to_string());
                                        } else {
                                            world.get_player_mut().position = vec3(
                                                x.clone().unwrap(),
                                                y.clone().unwrap(),
                                                z.clone().unwrap(),
                                            );
                                            world.get_player_mut().velocity = vec3(0.0, 0.0, 0.0);
                                            chat_hist.push(format!(
                                                "Teleported to: {:.2} {:.2} {:.2}",
                                                x.unwrap(),
                                                y.unwrap(),
                                                z.unwrap()
                                            ));
                                        }
                                    }
                                }
                                Some("vsync") => {
                                    if parts.len() != 2 {
                                        chat_hist.push("Usage: /vsync <on|off>".to_string());
                                    } else if parts[1] == "on" {
                                        vsync = true;
                                        chat_hist.push("VSync enabled.".to_string());
                                    } else if parts[1] == "off" {
                                        vsync = false;
                                        chat_hist.push("VSync disabled.".to_string());
                                    } else {
                                        chat_hist.push("Usage: /vsync <on|off>".to_string());
                                    }
                                }
                                Some("fov") => {
                                    if parts.len() != 2 {
                                        chat_hist.push("Usage: /fov <degrees>".to_string());
                                    } else {
                                        let fov = parts[1].parse::<f32>();
                                        if let Ok(mut fov) = fov {
                                            if !(30.0..=120.0).contains(&fov) {
                                                chat_hist.push("FOV must be between 30 and 120 degrees. It has been clamped.".to_string());
                                                fov = fov.clamp(30.0, 120.0);
                                            }
                                            world.get_player_mut().set_fov(fov, app.window.size());
                                            chat_hist.push(format!("FOV set to {:.2}", fov));
                                        } else {
                                            chat_hist.push("Invalid FOV value.".to_string());
                                        }
                                    }
                                }
                                Some(cmd) => {
                                    chat_hist.push(format!("Unknown command: {}", cmd));
                                }
                                None => {}
                            }
                        }
                        chat_open = false;
                        grab = true;
                    } else if *key == Keycode::Backspace && chat_open {
                        if let Some(ref mut cmd) = command {
                            cmd.pop();
                        }
                    } else if chat_open
                        && let Some(ref mut cmd) = command
                        && let Some(c) = key_to_char(*key)
                        && !c.is_control()
                    {
                        cmd.push(c);
                    }
                }
                sdl2::event::Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    keys_down.remove(key);
                }
                sdl2::event::Event::MouseButtonDown {
                    mouse_btn: button, ..
                } => {
                    mouse_down.insert(*button);
                }
                sdl2::event::Event::MouseMotion { xrel, yrel, .. } => {
                    if app.window.grab() {
                        let sensitivity = 0.175;
                        world.get_player_mut().yaw += (*xrel as f32) * sensitivity;
                        world.get_player_mut().pitch -= (*yrel as f32) * sensitivity;
                        if world.get_player().pitch > 89.0 {
                            world.get_player_mut().pitch = 89.0;
                        }
                        if world.get_player().pitch < -89.0 {
                            world.get_player_mut().pitch = -89.0;
                        }

                        // Update camera front vector
                        let yaw_rad = world.get_player().yaw.to_radians();
                        let pitch_rad = world.get_player().pitch.to_radians();
                        world.get_player_mut().forward = vec3(
                            yaw_rad.cos() * pitch_rad.cos(),
                            pitch_rad.sin(),
                            yaw_rad.sin() * pitch_rad.cos(),
                        )
                        .normalize();
                    }
                }
                sdl2::event::Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(w, h),
                    ..
                } => {
                    framebuffer.resize(*w, *h);
                    ssao_framebuffer.resize(*w, *h);
                    unsafe {
                        app.gl.viewport(0, 0, *w, *h);
                    }
                    ui_projection =
                        Mat4::orthographic_rh_gl(0.0, *w as f32, *h as f32, 0.0, -3.0, 3.0);
                    cursor = font.build(
                        &app.gl,
                        "*",
                        *w as f32 / 2.0 - 10.0,
                        *h as f32 / 2.0 - 10.0,
                        36.0,
                    );
                }
                _ => {}
            }
        }

        let player = world.get_player().clone();
        let dt = (Instant::now() - last_time).as_secs_f64().min(2.0);
        last_time = Instant::now();

        if grab {
            app.sdl.mouse().set_relative_mouse_mode(true);
            app.window.set_grab(true);
        } else {
            app.sdl.mouse().set_relative_mouse_mode(false);
            app.window.set_grab(false);
        }

        if duration.elapsed().as_secs_f32() >= 0.5 {
            fps = 1.0 / dt.max(f64::MIN_POSITIVE);
            duration = Instant::now();
        }
        let text = format!(
            r#"Mineplace3D v{}
FPS: {:.2}
DT: {:.4}
XYZ: {:.2} {:.2} {:.2}
SEED: {}
FACING: {}
INDICES: {}



Current Block: {}"#,
            env!("CARGO_PKG_VERSION"),
            fps,
            dt,
            player.position.x,
            player.position.y,
            player.position.z,
            world.seed(),
            if player.forward.x.abs() > player.forward.z.abs() {
                if player.forward.x > 0.0 {
                    "+X / E"
                } else {
                    "-X / W"
                }
            } else if player.forward.z > 0.0 {
                "+Z / S"
            } else {
                "-Z / N"
            },
            world
                .meshes
                .values()
                .map(|m| m.index_count())
                .sum::<usize>(),
            world
                .resource_mgr
                .get::<asset::Translations>("translations")
                .unwrap()
                .get({
                    let block = PLACABLE_BLOCKS[player.current_block];
                    block.into()
                })
                .unwrap_or(&"Unknown".to_string()),
        );
        debug_mesh = font.build(&app.gl, &text, 50.0, 50.0, 24.0);
        if let Some(ref cmd) = command {
            chat_mesh = font.build(
                &app.gl,
                &cmd.to_string(),
                50.0,
                app.window.size().1 as f32 - 150.0 - 24.0,
                24.0,
            );
        }
        let chat_hist_text = chat_hist
            .join("\n")
            .lines()
            .rev()
            .take(20)
            .collect::<Vec<&str>>()
            .into_iter()
            .rev()
            .collect::<Vec<&str>>()
            .join("\n");
        chat_hist_mesh = font.build(
            &app.gl,
            &chat_hist_text,
            50.0,
            app.window.size().1 as f32 - font.text_metrics(&chat_hist_text, 24.0).1 - 150.0 - 24.0,
            24.0,
        );
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
        while let Ok(ChunkResult::Generated {
            cx,
            cy,
            cz,
            chunk,
            outside_blocks,
        }) = result_receiver.try_recv()
        {
            world.add_chunk(cx, cy, cz, chunk, outside_blocks);
        }
        world.update(window_events.as_slice(), dt);
        let vp = player.projection * view;
        world.update_mesh_visibility(vp);
        world.generate_meshes(&app.gl);

        let blocks = shift_vec(&PLACABLE_BLOCKS, player.current_block)
            [mid(&PLACABLE_BLOCKS) - 3..=mid(&PLACABLE_BLOCKS) + 3]
            .to_vec();
        let block_meshes = blocks
            .iter()
            .enumerate()
            .map(|(i, block)| {
                let size = vec2(60.0, -60.0);
                let x = 100.0 + i as f32 * (size.x * 5.0 / 3.0);
                let y = app.window.size().1 as f32 - 50.0;
                let position = vec2(x, y);

                block.ui_mesh(
                    &app.gl,
                    position,
                    position + size,
                    Mat4::from_rotation_x(30f32.to_radians())
                        * Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_4),
                    world
                        .resource_mgr
                        .get::<asset::ModelDefs>("model_defs")
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let block_mesh_multiply_colors = blocks
            .iter()
            .enumerate()
            .map(|(i, block)| {
                let alpha = if i == 3 { 1.0 } else { 0.75 };
                if matches!(block, Block::Grass) {
                    vec4(0.5, 1.0, 0.6, alpha)
                } else if matches!(block, Block::Leaves) {
                    vec4(0.45, 1.3, 0.54, alpha)
                } else {
                    vec4(1.0, 1.0, 1.0, alpha)
                }
            })
            .collect::<Vec<_>>();

        let shader = world
            .resource_mgr
            .get::<ShaderProgram>("block_shader")
            .unwrap();
        let outline_shader = world
            .resource_mgr
            .get::<ShaderProgram>("outline_shader")
            .unwrap();
        let cloud_shader = world
            .resource_mgr
            .get::<ShaderProgram>("cloud_shader")
            .unwrap();
        let ssao_shader = world
            .resource_mgr
            .get::<ShaderProgram>("ssao_shader")
            .unwrap();
        let postprocessing_shader = world
            .resource_mgr
            .get::<ShaderProgram>("postprocessing_shader")
            .unwrap();
        let ui_shader = world
            .resource_mgr
            .get::<ShaderProgram>("ui_shader")
            .unwrap();
        unsafe {
            framebuffer.bind();

            app.gl.enable(glow::DEPTH_TEST);
            app.gl.clear_color(0.6, 0.6, 0.9, 1.0);
            app.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            shader.use_program();
            world
                .resource_mgr
                .get::<Texture>("atlas")
                .unwrap()
                .bind_to_unit(0);
            shader.set_uniform("view", view);
            shader.set_uniform("projection", player.projection);
            shader.set_uniform("texture_sampler", 0);
            shader.set_uniform("textures_per_row", 12);
            shader.set_uniform("texture_row_count", 12);
            shader.set_uniform("chunk_side_length", CHUNK_SIZE as f32);
            shader.set_uniform("time", time);
            for (pos, mesh) in &world.meshes {
                if !world.mesh_visible.contains(pos) {
                    continue;
                }
                shader.set_uniform("chunk_pos", pos);
                mesh.draw();
            }

            if let Some(ref hit) = player.selected_block {
                outline_shader.use_program();
                outline_shader.set_uniform(
                    "model",
                    Mat4::from_translation(hit.block_pos.as_vec3())
                        * Mat4::from_scale(vec3(1.005, 1.005, 1.005)),
                );
                outline_shader.set_uniform("view", view);
                outline_shader.set_uniform("projection", player.projection);
                outline_shader.set_uniform("color", vec3(1.0, 1.0, 1.0));
                outline_mesh.draw();
            }

            app.gl.disable(glow::CULL_FACE);
            app.gl.depth_mask(false);
            cloud_shader.use_program();
            cloud_shader.set_uniform("view", view);
            cloud_shader.set_uniform("projection", player.cloud_projection);
            cloud_shader.set_uniform("time", time);
            world
                .resource_mgr
                .get::<Texture>("cloud")
                .unwrap()
                .bind_to_unit(0);
            cloud_shader.set_uniform("cloud_texture", 0);
            cloud_plane.draw();
            app.gl.enable(glow::CULL_FACE);
            app.gl.depth_mask(true);

            world.draw_entities(&app.gl);

            Framebuffer::unbind(&app.gl);

            ssao_framebuffer.bind();

            app.gl.disable(glow::DEPTH_TEST);
            app.gl.clear_buffer_f32_slice(glow::COLOR, 0, &[1.0]);
            app.gl.clear(glow::COLOR_BUFFER_BIT);
            ssao_shader.use_program();
            framebuffer.depth_texture().unwrap().bind_to_unit(0);
            ssao_shader.set_uniform("depth_texture", 0);
            ssao_noise_texture.bind_to_unit(1);
            ssao_shader.set_uniform("noise_texture", 1);
            ssao_shader.set_uniform("samples", ssao_samples);
            ssao_shader.set_uniform("projection", player.projection);
            ssao_shader.set_uniform("inverse_projection", player.projection.inverse());
            ssao_shader.set_uniform(
                "screen_size",
                vec2(app.window.size().0 as f32, app.window.size().1 as f32),
            );
            quad_mesh(&app.gl).draw();

            Framebuffer::unbind(&app.gl);

            app.gl.disable(glow::DEPTH_TEST);
            app.gl.clear(glow::COLOR_BUFFER_BIT);
            postprocessing_shader.use_program();
            framebuffer.texture().bind_to_unit(0);
            postprocessing_shader.set_uniform("texture_sampler", 0);
            ssao_framebuffer.texture().bind_to_unit(1);
            postprocessing_shader.set_uniform("ssao_texture", 1);
            quad_mesh(&app.gl).draw();

            if show_ui {
                ui_shader.use_program();
                world
                    .resource_mgr
                    .get::<Texture>("font")
                    .unwrap()
                    .bind_to_unit(0);
                ui_shader.set_uniform("projection", ui_projection);
                ui_shader.set_uniform("ui_color", vec4(1.0, 1.0, 1.0, 1.0));
                debug_mesh.draw();
                if grab {
                    cursor.draw();
                }
                if chat_open {
                    chat_mesh.draw();
                }
                chat_hist_mesh.draw();
                world
                    .resource_mgr
                    .get::<Texture>("atlas")
                    .unwrap()
                    .bind_to_unit(0);
                for (block_mesh, color) in
                    block_meshes.iter().zip(block_mesh_multiply_colors.iter())
                {
                    ui_shader.set_uniform("ui_color", color);
                    block_mesh.draw();
                }
            }
        }

        app.window.gl_swap_window();

        window_events.clear();
        time += dt as f32;
    }
}
