use crate::{events, geom::*};
use serde::{Deserialize, Serialize};
use winit;
use winit::event::VirtualKeyCode as KeyCode;

pub struct GameCamera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl GameCamera {
    pub fn build_view_projection_matrix(&self) -> (cgmath::Matrix4<f32>, cgmath::Matrix4<f32>) {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        (view, proj)
    }
}

pub trait Camera {
    fn new(player_pos: Pos3) -> Self;
    fn update(&mut self, _events: &events::Events, player_pos: Pos3) {}
    // fn render(&self, _rules: &GameData, _igs: &mut InstanceGroups) {}
    fn update_camera(&self, _cam: &mut GameCamera) {}
    fn integrate(&mut self) {}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrbitCamera {
    pub pitch: f32,
    pub yaw: f32,
    pub distance: f32,
    #[serde(with = "Pos3Def")]
    player_pos: Pos3,
    #[serde(with = "QuatDef")]
    player_rot: Quat,
}

impl Camera for OrbitCamera {
    fn new(player_pos: Pos3) -> Self {
        Self {
            pitch: 0.0,
            yaw: 0.0,
            distance: 10.0,
            player_pos,
            player_rot: Quat::new(1.0, 0.0, 0.0, 0.0),
        }
    }

    fn update(&mut self, events: &events::Events, player_pos: Pos3) {
        let (dx, dy) = events.mouse_delta();
        self.pitch += dy / 100.0;
        self.pitch = self.pitch.clamp(-PI / 4.0, PI / 4.0);
        self.yaw += dx / 100.0;
        self.yaw = self.yaw.clamp(-PI / 4.0, PI / 4.0);
        if events.key_pressed(KeyCode::Up) {
            self.distance -= 0.5;
        }
        if events.key_pressed(KeyCode::Down) {
            self.distance += 0.5;
        }
        self.player_pos = player_pos;
    }

    fn update_camera(&self, c: &mut GameCamera) {
        // The camera should point at the player
        c.target = self.player_pos;
        // And rotated around the player's position and offset backwards
        c.eye = self.player_pos * 0.8
            + (self.player_rot
                * Quat::from(cgmath::Euler::new(
                    cgmath::Rad(self.pitch),
                    cgmath::Rad(self.yaw),
                    cgmath::Rad(0.0),
                ))
                * Vec3::new(0.0, 0.0, -self.distance));
        // To be fancy, we'd want to make the camera's eye to be an object in the world and whose rotation is locked to point towards the player, and whose distance from the player is locked, and so on---so we'd have player OR camera movements apply accelerations to the camera which could be "beaten" by collision.
    }
}
