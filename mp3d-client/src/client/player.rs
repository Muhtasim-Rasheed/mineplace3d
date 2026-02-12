use glam::{Mat4, Vec3};
use mp3d_core::protocol::MoveInstructions;

pub struct ClientPlayer {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub input: MoveInstructions,
}

impl ClientPlayer {
    pub fn view(&self) -> Mat4 {
        let pitch_rad = self.pitch.to_radians();
        let yaw_rad = self.yaw.to_radians();

        let forward = Vec3::new(
            yaw_rad.sin() * pitch_rad.cos(),
            -pitch_rad.sin(),
            yaw_rad.cos() * pitch_rad.cos(),
        )
        .normalize();

        Mat4::look_at_rh(self.position, self.position + forward, Vec3::Y)
    }

    pub fn projection(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov.to_radians(), aspect_ratio, 0.1, 1000.0)
    }

    pub fn optimistic(&mut self, tps: u8) {
        let yaw_rad = self.input.yaw.to_radians();
        let forward_vec = Vec3::new(yaw_rad.sin(), 0.0, yaw_rad.cos());
        let right_vec = Vec3::new(yaw_rad.cos(), 0.0, -yaw_rad.sin());
        let mut movement = Vec3::ZERO;
        movement += forward_vec * (self.input.forward as f32) * 7.5;
        movement += right_vec * (self.input.strafe as f32) * 7.5;
        if self.input.jump {
            movement.y += 6.0;
        }
        if self.input.sneak {
            movement.y -= 6.0;
        }
        self.velocity += movement * (1.0 / tps as f32) * 5.0;
        self.position += self.velocity * (1.0 / tps as f32);
        self.velocity *= 0.9_f32.powf(1.0 / tps as f32 * 48.0);
    }
}
