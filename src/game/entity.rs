use std::{collections::HashSet, sync::Arc};

use glam::*;
use glow::HasContext;
use sdl2::{keyboard::Keycode, mouse::MouseButton};

use crate::{
    abs::{Mesh, ShaderProgram, Texture, Vertex},
    game::{Block, BlockType, RayHit, ResourceManager, World, cast_ray},
};

pub const PLACABLE_BLOCKS: [Block; 22] = [
    Block::Grass,
    Block::Dirt,
    Block::Planks,
    Block::PlanksSlabTop,
    Block::PlanksSlabBottom,
    Block::PlanksStairsN,
    Block::PlanksStairsS,
    Block::PlanksStairsE,
    Block::PlanksStairsW,
    Block::OakLog,
    Block::Leaves,
    Block::CobbleStone,
    Block::StoneSlabTop,
    Block::StoneSlabBottom,
    Block::StoneStairsN,
    Block::StoneStairsS,
    Block::StoneStairsE,
    Block::StoneStairsW,
    Block::Glass,
    Block::Brick,
    Block::Snow,
    Block::Glungus,
];

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EntityId {
    id: u32,
    entity_name: String,
}

impl EntityId {
    fn new<E: Entity>() -> EntityId {
        EntityId {
            id: rand::random(),
            entity_name: std::any::type_name::<E>()
                .rsplit("::")
                .next()
                .unwrap()
                .to_string(),
        }
    }
}

impl std::str::FromStr for EntityId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split_once("-").ok_or("EntityId: parsing error")?;
        let id = parts
            .0
            .parse::<u32>()
            .map_err(|e| format!("EntityId: {}", e))?;
        Ok(EntityId {
            id,
            entity_name: parts.1.to_string(),
        })
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.id, self.entity_name)
    }
}

pub trait Entity: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>().rsplit("::").next().unwrap()
    }
    fn id(&self) -> EntityId
    where
        Self: Sized,
    {
        EntityId::new::<Self>()
    }
    fn position(&self) -> Vec3;
    fn velocity(&self) -> Vec3;
    fn apply_velocity(&mut self, delta: Vec3);
    fn width(&self) -> f32;
    fn height(&self) -> f32;
    fn eye_height(&self) -> f32;
    fn requests_removal(&self) -> bool {
        false
    }
    fn update(&mut self, id: EntityId, world: &mut World, events: &[sdl2::event::Event], dt: f64);
    fn draw(&self, _gl: &Arc<glow::Context>, _world: &World, _resource_manager: &ResourceManager) {}
}

#[derive(Clone)]
pub struct Player {
    pub old_position: Vec3,
    pub position: Vec3,
    pub velocity: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub jumping: bool,
    pub keys_down: HashSet<Keycode>,
    pub mouse_down: HashSet<MouseButton>,
    pub break_place_cooldown: u32,
    pub selected_block: Option<RayHit>,
    pub current_block: usize,
    pub sneaking: bool,
    pub projection: Mat4,
    pub cloud_projection: Mat4,
    chat_open: bool,
}

impl Player {
    pub fn new(position: Vec3, window: &sdl2::video::Window) -> Self {
        Player {
            old_position: position,
            position,
            velocity: Vec3::ZERO,
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
            yaw: -90.0,
            pitch: 0.0,
            jumping: false,
            keys_down: HashSet::new(),
            mouse_down: HashSet::new(),
            break_place_cooldown: 0,
            selected_block: None,
            current_block: 0,
            sneaking: false,
            projection: Mat4::perspective_rh_gl(
                90f32.to_radians(),
                window.size().0 as f32 / window.size().1 as f32,
                0.1,
                200.0,
            ),
            cloud_projection: Mat4::perspective_rh_gl(
                90f32.to_radians(),
                window.size().0 as f32 / window.size().1 as f32,
                0.1,
                400.0,
            ),
            chat_open: false,
        }
    }

    pub fn camera_pos(&self) -> Vec3 {
        self.position.with_y(self.position.y + self.eye_height())
    }
}

impl Entity for Player {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn velocity(&self) -> Vec3 {
        self.velocity
    }

    fn apply_velocity(&mut self, delta: Vec3) {
        self.velocity += delta;
    }

    fn width(&self) -> f32 {
        0.6
    }

    fn height(&self) -> f32 {
        1.8
    }

    fn eye_height(&self) -> f32 {
        if self.sneaking { 1.55 } else { 1.7 }
    }

    fn update(&mut self, _id: EntityId, world: &mut World, events: &[sdl2::event::Event], dt: f64) {
        for event in events {
            match event {
                sdl2::event::Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => {
                    self.current_block =
                        (self.current_block + PLACABLE_BLOCKS.len() - 1) % PLACABLE_BLOCKS.len();
                }
                sdl2::event::Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => {
                    self.current_block = (self.current_block + 1) % PLACABLE_BLOCKS.len();
                }
                sdl2::event::Event::KeyDown {
                    keycode: Some(key), ..
                } => match *key {
                    Keycode::T | Keycode::Slash => self.chat_open = true,
                    Keycode::Return | Keycode::Escape => self.chat_open = false,
                    _ => {
                        self.keys_down.insert(*key);
                    }
                },
                sdl2::event::Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    self.keys_down.remove(key);
                }
                sdl2::event::Event::MouseButtonDown { mouse_btn, .. } => {
                    self.mouse_down.insert(*mouse_btn);
                }
                sdl2::event::Event::MouseButtonUp { mouse_btn, .. } => {
                    self.mouse_down.remove(mouse_btn);
                }
                sdl2::event::Event::MouseWheel { y, .. } => {
                    if *y > 0 {
                        self.current_block = (self.current_block + PLACABLE_BLOCKS.len() - 1)
                            % PLACABLE_BLOCKS.len();
                    } else if *y < 0 {
                        self.current_block = (self.current_block + 1) % PLACABLE_BLOCKS.len();
                    }
                }
                _ => {}
            }
        }

        self.sneaking =
            self.keys_down.contains(&Keycode::LShift) || self.keys_down.contains(&Keycode::RShift);

        self.selected_block = cast_ray(world, self.camera_pos(), self.forward, 5.0);

        let player_accel = if self.sneaking
            || world
                .get_block(
                    self.position.x as i32,
                    (self.position.y as i32) - 4,
                    self.position.z as i32,
                )
                .block_type()
                == BlockType::Air
        {
            0.5
        } else {
            0.9
        };
        let jump_accel = 8.0;
        let sprint_player_accel = player_accel * if self.sneaking { 1.0 } else { 1.5 };
        if self.keys_down.contains(&Keycode::W) {
            self.velocity += vec3(self.forward.x, 0.0, self.forward.z).normalize()
                * if self.keys_down.contains(&Keycode::LCtrl)
                    || self.keys_down.contains(&Keycode::Q)
                {
                    sprint_player_accel
                } else {
                    player_accel
                };
        }
        if self.keys_down.contains(&Keycode::S) {
            self.velocity -= vec3(self.forward.x, 0.0, self.forward.z).normalize() * player_accel;
        }
        if self.keys_down.contains(&Keycode::A) {
            self.velocity -= self.forward.cross(self.up).normalize() * player_accel;
        }
        if self.keys_down.contains(&Keycode::D) {
            self.velocity += self.forward.cross(self.up).normalize() * player_accel;
        }
        if self.keys_down.contains(&Keycode::Space) && !self.jumping {
            self.velocity.y += jump_accel;
        }
        self.old_position = self.position;
        if self.break_place_cooldown > 0 {
            self.break_place_cooldown -= 1;
        }
        if self.mouse_down.contains(&MouseButton::Right) && self.break_place_cooldown == 0 {
            if let Some(ref hit) = self.selected_block {
                let block_pos = hit.block_pos;
                let hit_normal = hit.face_normal;
                let new_pos = block_pos + hit_normal;
                world.set_block(
                    new_pos.x,
                    new_pos.y,
                    new_pos.z,
                    PLACABLE_BLOCKS[self.current_block],
                );
                let (collide_x, collide_y, collide_z) =
                    world.player_collision_mask(self.old_position, self.position, 0.5, 1.8);
                if collide_x || collide_y || collide_z {
                    world.set_block(new_pos.x, new_pos.y, new_pos.z, Block::Air);
                }
                self.break_place_cooldown = 12;
            }
        }
        if self.mouse_down.contains(&MouseButton::Left) && self.break_place_cooldown == 0 {
            if let Some(ref hit) = self.selected_block {
                if !(world.get_block(hit.block_pos.x, hit.block_pos.y, hit.block_pos.z)
                    == Block::Bedrock)
                {
                    let block_pos = hit.block_pos;
                    world.break_block(block_pos);
                    self.break_place_cooldown = 12;
                }
            }
        }
        self.velocity.y -= 0.75 - 0.2 * self.velocity.y;
        self.position += self.velocity * dt as f32;
        if self.sneaking {
            self.velocity.x *= 0.5;
            self.velocity.z *= 0.5;
            self.velocity.y *= 0.85;
        } else {
            self.velocity *= 0.85;
        }

        let (collide_x, collide_y, collide_z) = world.player_collision_mask(
            self.old_position,
            self.position,
            self.width(),
            self.height(),
        );

        if collide_y {
            self.position.y = self.old_position.y;
            self.jumping = false;
            self.velocity.y = 0.0;
        } else {
            self.jumping = true;
        }

        if collide_x || collide_z {
            let mut stepped_pos = self.old_position;
            stepped_pos.y += 0.55;
            stepped_pos.x = self.position.x;
            stepped_pos.z = self.position.z;

            if !world.is_player_colliding(stepped_pos, self.width(), self.height()) && !self.jumping
            {
                self.position = stepped_pos;
            } else {
                if collide_x {
                    self.position.x = self.old_position.x;
                    self.velocity.x = 0.0;
                }
                if collide_z {
                    self.position.z = self.old_position.z;
                    self.velocity.z = 0.0;
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum BillboardType {
    Explosion = 0,
}

impl BillboardType {
    pub fn uvs(&self) -> [Vec2; 2] {
        let tile_index = *self as u32;
        let tile_x = tile_index % 12;
        let tile_y = tile_index / 12;

        let uv_unit = 1.0 / 12.0;
        let uv_row_unit = 1.0 / 12.0;

        [
            vec2(tile_x as f32 * uv_unit, tile_y as f32 * uv_row_unit),
            vec2(
                (tile_x + 1) as f32 * uv_unit,
                (tile_y + 1) as f32 * uv_row_unit,
            ),
        ]
    }

    pub fn spherical_billboard(&self) -> bool {
        match self {
            BillboardType::Explosion => true,
        }
    }

    pub fn knockback_mult(&self) -> f32 {
        match self {
            BillboardType::Explosion => 2.0,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct BillboardVertex {
    pub corner: Vec2,
    pub uv: Vec2,
}

impl Vertex for BillboardVertex {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            let stride = std::mem::size_of::<BillboardVertex>() as i32;

            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(0);

            gl.vertex_attrib_pointer_f32(
                1,
                2,
                glow::FLOAT,
                false,
                stride,
                std::mem::size_of::<Vec2>() as i32,
            );
            gl.enable_vertex_attrib_array(1);
        }
    }
}

#[derive(Clone)]
pub struct Billboard {
    pub position: Vec3,
    pub size: f32,
    pub life: u32,
    pub kind: BillboardType,
    start_size: f32,
    shader_key: String,
    atlas_key: String,
}

impl Billboard {
    pub fn new(
        position: Vec3,
        size: f32,
        life: u32,
        kind: BillboardType,
        shader_key: &str,
        atlas_key: &str,
    ) -> Self {
        Billboard {
            position,
            size,
            life,
            kind,
            start_size: size,
            shader_key: shader_key.to_string(),
            atlas_key: atlas_key.to_string(),
        }
    }
}

impl Entity for Billboard {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn velocity(&self) -> Vec3 {
        Vec3::ZERO
    }

    fn apply_velocity(&mut self, _delta: Vec3) {
        // Billboards do not move
    }

    fn width(&self) -> f32 {
        self.size
    }

    fn height(&self) -> f32 {
        self.size
    }

    fn eye_height(&self) -> f32 {
        self.size / 2.0
    }

    fn requests_removal(&self) -> bool {
        self.life == 0
    }

    fn update(
        &mut self,
        id: EntityId,
        world: &mut World,
        _events: &[sdl2::event::Event],
        _dt: f64,
    ) {
        if self.life > 0 {
            self.life -= 1;
        }
        if self.life > 0 {
            self.size = self.start_size * (self.life as f32 / 30.0);
        }

        let effect_radius = 3.0;

        for (other_id, entity) in &world.entities {
            if *other_id == id || entity.borrow().name() == self.name() {
                continue;
            }
            let to_entity = entity.borrow().position() - self.position;
            let distance = to_entity.length_squared();
            if distance < effect_radius * effect_radius && distance > 0.0 {
                let force = to_entity.normalize()
                    * (effect_radius - distance.sqrt())
                    * self.kind.knockback_mult();
                entity.borrow_mut().apply_velocity(force);
            }
        }
    }

    fn draw(&self, gl: &Arc<glow::Context>, world: &World, resource_manager: &ResourceManager) {
        if self.life == 0 {
            return;
        }
        let shader = resource_manager
            .get::<ShaderProgram>(&self.shader_key)
            .unwrap();
        let atlas = resource_manager.get::<Texture>(&self.atlas_key).unwrap();
        let uvs = self.kind.uvs();

        let view = Mat4::look_at_rh(
            world.get_player().camera_pos(),
            world.get_player().camera_pos() + world.get_player().forward,
            world.get_player().up,
        );
        let projection = world.get_player().projection;

        let vertices = vec![
            BillboardVertex {
                corner: vec2(-1.0, -1.0),
                uv: vec2(uvs[0].x, uvs[1].y),
            },
            BillboardVertex {
                corner: vec2(1.0, -1.0),
                uv: vec2(uvs[1].x, uvs[1].y),
            },
            BillboardVertex {
                corner: vec2(1.0, 1.0),
                uv: vec2(uvs[1].x, uvs[0].y),
            },
            BillboardVertex {
                corner: vec2(-1.0, 1.0),
                uv: vec2(uvs[0].x, uvs[0].y),
            },
        ];
        let indices = [0, 1, 2, 0, 2, 3];
        let mesh = Mesh::new(gl, &vertices, &indices, glow::TRIANGLES);

        shader.use_program();
        shader.set_uniform("view", view);
        shader.set_uniform("projection", projection);
        shader.set_uniform("center", self.position);
        shader.set_uniform("size", self.size);
        shader.set_uniform("spherical", self.kind.spherical_billboard());
        atlas.bind_to_unit(0);
        shader.set_uniform("texture_sampler", 0);
        mesh.draw();
    }
}
