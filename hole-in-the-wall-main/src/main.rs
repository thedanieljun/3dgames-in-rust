use ambisonic::Ambisonic;
use ambisonic::SoundController;
use ambisonic::{rodio, AmbisonicBuilder};
use cgmath::Matrix3;
use engine3d::{
    camera::*,
    collision,
    geom::*,
    render::{InstanceGroups, InstanceRaw},
    run, Engine, DT,
};
use rand;
use rand::Rng;
use rodio::Source;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;
use winit;
use winit::event::VirtualKeyCode as KeyCode;

const G: f32 = 5.0;
const MIN_VEL: f32 = 0.1; // if absolute velocity is below this value, consider the object to be stationary
const MBHS: f32 = 0.5; // menu box half size
const WBHS: f32 = 1.0; // wall box half size
const PBHS: f32 = 0.5; // player box half size
const WH: i8 = 3; // wall height in boxes
const WW: i8 = 6; // wall width in boxes
const WIV: Vec3 = Vec3::new(0.0, 0.0, -2.0); // initial velocity of wall
const WIZ: f32 = 20.0; // initial z position of wall
const WVSF: f32 = 0.5; // wall velocity scaling factor

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
enum Mode {
    Menu,
    GamePlay,
    EndScreen,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MenuObject {
    pub body: Box,
}

impl MenuObject {
    fn render(&self, rules: &GameData, igs: &mut InstanceGroups) {
        igs.render(
            rules.menu_object_model,
            InstanceRaw {
                model: (Mat4::from_translation(self.body.c.to_vec())
                    * Mat4::from_nonuniform_scale(
                        self.body.half_sizes.x,
                        self.body.half_sizes.y,
                        self.body.half_sizes.z,
                    ))
                .into(),
            },
        );
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct StartObject {
    pub body: Box,
}

impl StartObject {
    fn render(&self, rules: &GameData, igs: &mut InstanceGroups) {
        igs.render(
            rules.start_model,
            InstanceRaw {
                model: (Mat4::from_translation(self.body.c.to_vec())
                    * Mat4::from_nonuniform_scale(
                        self.body.half_sizes.x,
                        self.body.half_sizes.y,
                        self.body.half_sizes.z,
                    ))
                .into(),
            },
        );
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct LoadObject {
    pub body: Box,
}

impl LoadObject {
    fn render(&self, rules: &GameData, igs: &mut InstanceGroups) {
        igs.render(
            rules.load_model,
            InstanceRaw {
                model: (Mat4::from_translation(self.body.c.to_vec())
                    * Mat4::from_nonuniform_scale(
                        self.body.half_sizes.x,
                        self.body.half_sizes.y,
                        self.body.half_sizes.z,
                    ))
                .into(),
            },
        );
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ScoreObject {
    pub body: Box,
}

impl ScoreObject {
    fn render(&self, rules: &GameData, igs: &mut InstanceGroups, score: i8) {
        igs.render(
            rules.score_models[score as usize],
            InstanceRaw {
                model: (Mat4::from_translation(self.body.c.to_vec())
                    * Mat4::from_nonuniform_scale(
                        self.body.half_sizes.x,
                        self.body.half_sizes.y,
                        self.body.half_sizes.z,
                    ))
                .into(),
            },
        );
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum WallType {
    Diamond,
    Glass,
}

// #[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[derive(Clone, PartialEq, Debug)]
pub struct Wall {
    pub wall_type: WallType,
    pub body: Vec<Box>,
    pub vels: Vec<Vec3>,
    pub rots: Vec<Quat>,
    pub omegas: Vec<Vec3>,
    pub missing_x: i8,
    pub missing_y: i8,
    control: (i8, i8),
}

impl Wall {
    pub fn generate_components(
        wall_z: f32,
        axes: Mat3,
        missing: Option<(i8, i8)>,
    ) -> (Vec<Box>, i8, i8) {
        let missing_x = if missing.is_some() {
            missing.unwrap().0
        } else {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..WW)
        };
        let missing_y = if missing.is_some() {
            missing.unwrap().1
        } else {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..WH)
        };

        let mut boxes = vec![];
        let half_sizes = Vec3::new(WBHS, WBHS, WBHS);
        for x in 0..WW {
            for y in 0..WH {
                if x != missing_x || y != missing_y {
                    let c = Pos3::new(
                        x as f32 * 2.0 * WBHS + WBHS - WW as f32 * WBHS,
                        y as f32 * 2.0 * WBHS + WBHS,
                        wall_z,
                    );
                    boxes.push(Box {
                        c,
                        axes,
                        half_sizes,
                    })
                }
            }
        }
        (boxes, missing_x, missing_y)
    }

    fn reset(&mut self, score: i8) {
        let mut rng = rand::thread_rng();
        let wall_type = if rng.gen_range(0..1) == 0 {
            WallType::Diamond
        } else {
            WallType::Glass
        };
        self.wall_type = wall_type;
        let (boxes, missing_x, missing_y) = Wall::generate_components(WIZ, Mat3::one(), None);
        self.body = boxes;
        self.missing_x = missing_x;
        self.missing_y = missing_y;
        let n_boxes = self.body.len();
        self.vels = vec![WIV * (score + 1) as f32 * WVSF; n_boxes];
        self.rots = vec![Quat::new(1.0, 0.0, 0.0, 0.0); n_boxes];
        self.omegas = vec![Vec3::zero(); n_boxes];
        self.control = (0, 0);
    }

    fn render(&self, rules: &GameData, igs: &mut InstanceGroups) {
        let model = match self.wall_type {
            WallType::Diamond => rules.diamond_wall_model,
            WallType::Glass => rules.glass_wall_model,
        };
        for (i, b) in self.body.iter().enumerate() {
            igs.render(
                model,
                InstanceRaw {
                    model: (Mat4::from_translation(b.c.to_vec())
                        * Mat4::from_nonuniform_scale(
                            b.half_sizes.x,
                            b.half_sizes.y,
                            b.half_sizes.z,
                        )
                        * Mat4::from(self.rots[i]))
                    .into(),
                },
            );
        }
    }

    fn input(&mut self, events: &engine3d::events::Events) {
        self.control.0 = if events.key_held(KeyCode::A) {
            -1
        } else if events.key_held(KeyCode::D) {
            1
        } else {
            0
        };
        self.control.1 = if events.key_held(KeyCode::W) {
            -1
        } else if events.key_held(KeyCode::S) {
            1
        } else {
            0
        };
    }

    fn integrate(&mut self) {
        for (b, v) in &mut self.body.iter_mut().zip(self.vels.iter()) {
            b.c += v * DT;
        }

        for i in 0..self.body.len() {
            let drot = 0.5
                * DT
                * Quat::new(0.0, self.omegas[i].x, self.omegas[i].y, self.omegas[i].z)
                * self.rots[i];
            self.rots[i] += drot;
            self.body[i].axes = self.body[i].axes * Matrix3::from(drot);
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Platform {
    #[serde(with = "Plane")]
    pub body: Plane,
    control: (i8, i8),
}

impl Platform {
    pub fn generate_bounds(wall_height: i8, wall_width: i8) -> Vec<Platform> {
        let mut bounds = vec![];
        let btn = Vec3::new(0.0, 1.0, 0.0); // bottom & top normal vector
        let lrn = Vec3::new(1.0, 0.0, 0.0); // left & right normal vector

        let top_dist = wall_height as f32 * WBHS * 2.0;
        let left_dist = wall_width as f32 * WBHS * 2.0;
        let right_dist = -1.0 * left_dist;

        let b = Platform {
            body: Plane { n: btn, d: 0.0 },
            control: (0, 0),
        };
        let t = Platform {
            body: Plane {
                n: btn,
                d: top_dist,
            },
            control: (0, 0),
        };
        let l = Platform {
            body: Plane {
                n: lrn,
                d: left_dist,
            },
            control: (0, 0),
        };
        let r = Platform {
            body: Plane {
                n: lrn,
                d: right_dist,
            },
            control: (0, 0),
        };

        bounds.push(b);
        bounds.push(t);
        bounds.push(l);
        bounds.push(r);
        bounds
    }

    fn render(&self, rules: &GameData, igs: &mut InstanceGroups) {
        igs.render(
            rules.platform_model,
            engine3d::render::InstanceRaw {
                model: (Mat4::from(cgmath::Quaternion::between_vectors(
                    Vec3::new(0.0, 1.0, 0.0),
                    self.body.n,
                )) * Mat4::from_translation(self.body.n * self.body.d)
                    * Mat4::from_translation(Vec3::new(0.0, -0.025, 0.0))
                    * Mat4::from_nonuniform_scale(0.5, 0.05, 0.5))
                .into(),
            },
        );
    }

    fn input(&mut self, events: &engine3d::events::Events) {
        self.control.0 = if events.key_held(KeyCode::A) {
            -1
        } else if events.key_held(KeyCode::D) {
            1
        } else {
            0
        };
        self.control.1 = if events.key_held(KeyCode::W) {
            -1
        } else if events.key_held(KeyCode::S) {
            1
        } else {
            0
        };
    }

    fn integrate(&mut self) {
        self.body.n += Vec3::new(
            self.control.0 as f32 * 0.4 * DT,
            0.0,
            self.control.1 as f32 * 0.4 * DT,
        );
        self.body.n = self.body.n.normalize();
    }
}

pub struct Audio {
    scene: Ambisonic,
    sound1: Option<SoundController>,
    sound2: Option<SoundController>,
    sound3: Option<SoundController>,
    sound4: Option<SoundController>,
}

// #[derive(Serialize, Deserialize, Debug)]
// #[derive(Debug)]
struct Game<Cam: Camera> {
    start: StartObject,
    scores: ScoreObject,
    play_again: MenuObject,
    load_save: LoadObject,
    wall: Wall,
    floor: Platform,
    // bounds: Vec<Platform>,
    player: Player,
    camera: Cam,
    ps: Vec<collision::Contact<usize>>,
    ww: Vec<collision::Contact<usize>>,
    pw: Vec<collision::Contact<usize>>,
    fw: Vec<collision::Contact<usize>>,
    pf: Vec<collision::Contact<usize>>,
    pl: Vec<collision::Contact<usize>>,
    mode: Mode,
    score: i8,
    high_score: i8,
    audio: Audio,
    state: GameState,
}

#[derive(Serialize, Deserialize, Debug)]
struct GameState {
    wall_z: f32,
    missing_x: i8,
    missing_y: i8,
    wall_type: WallType,
    #[serde(with = "Pos3Def")]
    player_posn: Pos3,
    score: i8,
}

struct GameData {
    diamond_wall_model: engine3d::assets::ModelRef,
    glass_wall_model: engine3d::assets::ModelRef,
    platform_model: engine3d::assets::ModelRef,
    player_model: engine3d::assets::ModelRef,
    camera_model: engine3d::assets::ModelRef,
    menu_object_model: engine3d::assets::ModelRef,
    start_model: engine3d::assets::ModelRef,
    load_model: engine3d::assets::ModelRef,
    score_models: Vec<engine3d::assets::ModelRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub body: Box,
    #[serde(with = "Vec3Def")]
    pub velocity: Vec3,
    #[serde(with = "Vec3Def")]
    pub acc: Vec3,
    #[serde(with = "QuatDef")]
    pub rot: Quat,
    #[serde(with = "Vec3Def")]
    pub omega: Vec3,
}

impl Player {
    const MAX_SPEED: f32 = 3.0;
    fn render(&self, rules: &GameData, igs: &mut InstanceGroups) {
        igs.render(
            rules.player_model,
            InstanceRaw {
                model: (Mat4::from_translation(self.body.c.to_vec())
                    * Mat4::from_nonuniform_scale(
                        self.body.half_sizes.x,
                        self.body.half_sizes.y,
                        self.body.half_sizes.z,
                    )
                    * Mat4::from(self.rot))
                .into(),
            },
        );
    }
    fn integrate(&mut self) {
        self.velocity += self.rot * self.acc;
        // println!("inte {:?}", self.velocity);
        if self.velocity.magnitude() > Self::MAX_SPEED {
            self.velocity = self.velocity.normalize_to(Self::MAX_SPEED);
        }
        if self.velocity.magnitude() >= MIN_VEL {
            self.body.c += self.velocity * DT;
        }
        let drot = 0.5 * DT * Quat::new(0.0, self.omega.x, self.omega.y, self.omega.z) * self.rot;
        self.rot += drot;
        self.body.axes = self.body.axes * Matrix3::from(drot);
    }
}

impl<C: Camera> engine3d::Game for Game<C> {
    type StaticData = GameData;
    fn start(engine: &mut Engine) -> (Self, Self::StaticData) {
        // create menu objects
        let menu_object_half_sizes = Vec3::new(MBHS, MBHS, MBHS);
        let start = StartObject {
            body: Box {
                c: Pos3::new(3.0, MBHS, 0.0),
                axes: Matrix3::one(),
                half_sizes: menu_object_half_sizes,
            },
        };
        let scores = ScoreObject {
            body: Box {
                c: Pos3::new(-3.0, MBHS, 0.0),
                axes: Matrix3::one(),
                half_sizes: menu_object_half_sizes,
            },
        };
        let play_again = MenuObject {
            body: Box {
                c: Pos3::new(3.0, MBHS, 0.0),
                axes: Matrix3::one(),
                half_sizes: menu_object_half_sizes,
            },
        };
        let load_save = LoadObject {
            body: Box {
                c: Pos3::new(0.0, MBHS, 3.0),
                axes: Matrix3::one(),
                half_sizes: menu_object_half_sizes,
            },
        };

        // create wall
        // generate wall components
        // let boxes = Wall::generate_components(Matrix3::one());
        let (boxes, missing_x, missing_y) = Wall::generate_components(WIZ, Matrix3::one(), None);
        let n_boxes = boxes.len();
        let wall = Wall {
            wall_type: WallType::Glass,
            body: boxes,
            missing_x,
            missing_y,
            vels: vec![WIV; n_boxes],
            rots: vec![Quat::new(1.0, 0.0, 0.0, 0.0); n_boxes],
            omegas: vec![Vec3::zero(); n_boxes],
            control: (0, 0),
        };

        // create platform
        let floor = Platform {
            body: Plane {
                n: Vec3::new(0.0, 1.0, 0.0),
                d: 0.0,
            },
            control: (0, 0),
        };

        // let bounds = Platform::generate_bounds(wall_height, wall_width);

        // create player
        let player = Player {
            body: Box {
                c: Pos3::new(0.0, PBHS, 0.0),
                axes: Matrix3::one(),
                half_sizes: Vec3::new(PBHS, PBHS, PBHS),
            },
            velocity: Vec3::zero(),
            acc: Vec3::zero(),
            omega: Vec3::zero(),
            rot: Quat::new(1.0, 0.0, 0.0, 0.0),
        };

        // create camera
        let camera = C::new(player.body.c);

        // models
        // TODO: update .obj and .mtl files
        let menu_object_model = engine.load_model("box.obj");
        let diamond_wall_model = engine.load_model("wall.obj");
        let glass_wall_model = engine.load_model("glass-box.obj");
        let floor_model = engine.load_model("floor.obj");
        let player_model = engine.load_model("cube.obj");
        let camera_model = engine.load_model("sphere.obj");
        let start_model = engine.load_model("start.obj");
        let load_model = engine.load_model("load.obj");
        let score_models = vec![
            engine.load_model("score0.obj"),
            engine.load_model("score1.obj"),
            engine.load_model("score2.obj"),
            engine.load_model("score3.obj"),
            engine.load_model("score4.obj"),
            engine.load_model("score5.obj"),
            engine.load_model("score6.obj"),
            engine.load_model("score7.obj"),
            engine.load_model("score8.obj"),
            engine.load_model("score9.obj"),
        ];

        let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let scene = AmbisonicBuilder::default().build();

        // let source1 = source1.repeat_infinite();
        // let audio_paths = vec![
        //     "content/boxMovement.wav",
        //     "content/wallBreakSound.wav",
        //     "content/wallBreakSoundGlass.mp3",
        //     "content/wallTrainSound.mp3"
        // ];
        // let playing_action = vec![
        //     AlreadyPlayingAction::Nothing,
        //     AlreadyPlayingAction::Nothing,
        //     AlreadyPlayingAction::Nothing,
        //     AlreadyPlayingAction::Retrigger,
        // ];
        // let audio = Audio::new(audio_paths, playing_action);

        let audio = Audio {
            scene,
            sound1: None,
            sound2: None,
            sound3: None,
            sound4: None,
        };

        let state = GameState {
            wall_z: wall.body[0].c.z,
            missing_x,
            missing_y,
            wall_type: WallType::Glass,
            player_posn: player.body.c,
            score: 0,
        };

        // create game
        (
            Self {
                start,
                scores,
                play_again,
                load_save,
                wall,
                floor,
                player,
                camera,
                ps: vec![],
                ww: vec![],
                fw: vec![],
                pw: vec![],
                pf: vec![],
                pl: vec![],
                mode: Mode::Menu,
                score: 0,
                high_score: 0,
                audio,
                state
                // sources: vec![source1],
                // sources: vec![source1, source2, source3, source4],
            },
            GameData {
                menu_object_model,
                diamond_wall_model,
                glass_wall_model,
                platform_model: floor_model,
                player_model,
                camera_model,
                start_model,
                load_model,
                score_models,
            },
        )
    }

    fn render(&self, rules: &Self::StaticData, igs: &mut InstanceGroups) {
        // always render player and floor
        self.player.render(rules, igs);
        self.floor.render(rules, igs);

        match self.mode {
            Mode::Menu => {
                self.start.render(rules, igs);
                self.scores.render(rules, igs, self.score);
                self.load_save.render(rules, igs);
            }
            Mode::GamePlay => {
                self.wall.render(rules, igs);
            }
            Mode::EndScreen => {
                self.wall.render(rules, igs);
                self.play_again.render(rules, igs);
                self.scores.render(rules, igs, self.score);
                self.load_save.render(rules, igs);
            }
        }
    }

    fn handle_collision(&mut self) {
        self.pf.clear();
        self.pw.clear();
        let mut pb = [self.player.body];
        let mut pv = [self.player.velocity];

        // always check and restitute player - floor
        collision::gather_contacts_ab(&pb, &[self.floor.body], &mut self.pf);
        collision::restitute_dyn_stat(&mut pb, &mut pv, &[self.floor.body], &mut self.pf, false);
        // always check and restitute player - wall
        collision::gather_contacts_ab(&pb, &self.wall.body, &mut self.pw);
        collision::restitute_dyn_dyn(
            &mut pb,
            &mut pv,
            &mut self.wall.body,
            &mut self.wall.vels,
            &mut self.pw,
        );

        match self.mode {
            Mode::Menu => {
                self.ps.clear();
                self.pl.clear();
                // collision between player and start object
                collision::gather_contacts_ab(&pb, &[self.start.body], &mut self.ps);
                // collision between player and load save object
                collision::gather_contacts_ab(&pb, &[self.load_save.body], &mut self.pl);
            }
            Mode::GamePlay => {
                self.ww.clear();
                self.fw.clear();

                /*
                // wall - wall
                collision::gather_contacts_aa(&self.wall.body, &mut self.ww);
                collision::restitute_dyns(&mut self.wall.body, &mut self.wall.vels, &mut self.ww);

                // wall - floor
                collision::gather_contacts_ab(&self.wall.body, &[self.floor.body], &mut self.fw);
                collision::restitute_dyn_stat(
                    &mut self.wall.body,
                    &mut self.wall.vels,
                    &[self.floor.body],
                    &mut self.pf,
                    false,
                );
                */
            }
            Mode::EndScreen => {
                self.ps.clear();
                self.pl.clear();
                self.ww.clear();
                self.fw.clear();

                // player - play again menu object
                collision::gather_contacts_ab(&pb, &[self.play_again.body], &mut self.ps);
                // collision between player and load save object
                collision::gather_contacts_ab(&pb, &[self.load_save.body], &mut self.pl);

                // wall - wall
                collision::gather_contacts_aa(&self.wall.body, &mut self.ww);
                collision::restitute_dyns(&mut self.wall.body, &mut self.wall.vels, &mut self.ww);

                // floor - wall
                collision::gather_contacts_ab(&self.wall.body, &[self.floor.body], &mut self.fw);
                collision::restitute_dyn_stat(
                    &mut self.wall.body,
                    &mut self.wall.vels,
                    &[self.floor.body],
                    &mut self.fw,
                    true,
                );
            }
        }
        self.player.body = pb[0];
        self.player.velocity = pv[0];
        // self.player.body.c += self.player.velocity * DT;
    }

    fn update(&mut self, _rules: &Self::StaticData, engine: &mut Engine) {
        self.player.acc = Vec3::zero();

        // how much the player velocity changes per button click
        let h_disp = Vec3::new(0.05, 0.0, 0.0);
        let v_disp = Vec3::new(0.0, 0.30, 0.0);
        let z_disp = Vec3::new(0.0, 0.0, 0.05);
        let g_disp = Vec3::new(0.0, -G, 0.0);

        // player should not go past these bounds
        let top_bound = WH as f32 * WBHS * 2.0;
        let left_bound = WW as f32 * WBHS - 2.0;
        let right_bound = -left_bound + WBHS - 1.0;
        let front_bound = WIZ;
        let back_bound = 0.0;

        // apply gravity here instead of integrate() so handle_collision can deal with gravity smoothly
        self.player.velocity += g_disp * DT;
        if self.mode == Mode::EndScreen {
            for v in self.wall.vels.iter_mut() {
                *v += g_disp * DT;
            }
        }

        self.handle_collision();

        // move player
        let psn = self.player.body.c;
        if engine.events.key_held(KeyCode::A) && psn.x + PBHS + h_disp.x <= left_bound {
            self.player.acc += h_disp;
        } else if engine.events.key_held(KeyCode::D) && psn.x + PBHS - h_disp.x >= right_bound {
            self.player.acc -= h_disp;
        }
        if engine.events.key_held(KeyCode::W) && psn.z + PBHS + z_disp.x <= front_bound {
            self.player.acc += z_disp;
        } else if engine.events.key_held(KeyCode::S) && psn.z + PBHS - z_disp.x >= back_bound {
            self.player.acc -= z_disp;
        }
        if engine.events.key_held(KeyCode::Space) && psn.y + PBHS + v_disp.y <= top_bound {
            self.player.acc += v_disp;
        }

        if self.player.acc.magnitude2() > 1.0 {
            self.player.acc = self.player.acc.normalize();
        }

        // rotate player
        if engine.events.key_held(KeyCode::Q) {
            self.player.omega = Vec3::unit_y();
        } else if engine.events.key_held(KeyCode::E) {
            self.player.omega = -Vec3::unit_y();
        } else {
            self.player.omega = Vec3::zero();
        }

        // save game state
        if self.mode == Mode::GamePlay && engine.events.key_pressed(KeyCode::Return) {
            let serialized = serde_json::to_string(&self.state).unwrap();
            let mut file = File::create("savefile.txt").unwrap();
            file.write_all(&serialized.as_bytes()).unwrap();
        }
        // update game state
        self.state.wall_z = self.wall.body[0].c.z;
        self.state.missing_x = self.wall.missing_x;
        self.state.missing_y = self.wall.missing_y;
        self.state.player_posn = self.player.body.c;
        self.state.score = self.score;

        // orbit camera
        self.camera.update(&engine.events, self.player.body.c);

        if self.mode != Mode::Menu {
            self.wall.integrate();
            // update wall audio
            let wall_z = self.wall.body[0].c.z;
            // let source = &rules.audio.source4;
            self.audio
                .sound4
                .as_mut()
                .unwrap()
                .adjust_position([0.0, 0.0, wall_z]);
        }
        self.floor.integrate();
        self.player.integrate();
        self.camera.integrate();
        for collision::Contact { a: pa, .. } in self.pf.iter() {
            // apply "friction" to players on the ground
            assert_eq!(*pa, 0);
            self.player.velocity *= 0.98;
        }

        if (self.player.velocity.x.abs() <= 0.1
            // if player is not moving, or player is not on the ground, remove sound
            && self.player.velocity.z.abs() <= 0.1
            && self.player.acc.x.abs() <= 0.01
            && self.player.acc.z.abs() <= 0.01)
            || self.pf.is_empty()
        {
            if self.audio.sound1.is_some() {
                self.audio.sound1.as_mut().unwrap().stop();
            }
            self.audio.sound1 = None;
        } else {
            // if player is moving, play player movement sound
            let player_posn = [
                self.player.body.c.x,
                self.player.body.c.y,
                self.player.body.c.z,
            ];
            match &mut self.audio.sound1 {
                // if sound is already playing, adjust posn
                Some(sound) => {
                    sound.adjust_position(player_posn);
                }
                // if sound is not playing, play
                None => {
                    let file = std::fs::File::open("content/boxMovement.wav").unwrap();
                    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                    let source = source.amplify(0.25).repeat_infinite();
                    let sound = self
                        .audio
                        .scene
                        .play_at(source.convert_samples(), player_posn);
                    self.audio.sound1 = Some(sound);
                }
            }
        }

        // handle game transitions
        match self.mode {
            Mode::Menu => {
                // if player hits start menu object, start game
                if !self.ps.is_empty() {
                    self.mode = Mode::GamePlay;
                    // reset player position and score
                    self.player.body.c = Pos3::new(0.0, PBHS, 0.0);
                    self.score = 0;
                    // start playing wall sound
                    let file = std::fs::File::open("content/wallTrainSound.mp3").unwrap();
                    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                    let source = source.amplify(3.0).repeat_infinite();
                    let sound = self
                        .audio
                        .scene
                        .play_at(source.convert_samples(), [0.0, 0.0, WIZ]);
                    self.audio.sound4 = Some(sound);
                }
                // if player hits load save object, load save
                if !self.pl.is_empty() {
                    self.mode = Mode::GamePlay;
                    self.load_game();
                    // start playing wall sound
                    let file = std::fs::File::open("content/wallTrainSound.mp3").unwrap();
                    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                    let source = source.amplify(3.0).repeat_infinite();
                    let sound = self
                        .audio
                        .scene
                        .play_at(source.convert_samples(), [0.0, 0.0, WIZ]);
                    self.audio.sound4 = Some(sound);
                }
            }
            Mode::GamePlay => {
                // if player hits wall, end game
                if !self.pw.is_empty() {
                    self.mode = Mode::EndScreen;
                    // stop playing wall sound
                    self.audio.sound4.as_mut().unwrap().stop();
                    // Explode wall, away from player and toward the back
                    for pos in 0..self.wall.body.len() {
                        // self.wall.vels[pos] +=
                        //     (self.wall.body[pos].c - self.player.body.c - WIV * 3.0)
                        //         .normalize_to(rand::random::<f32>());

                        self.wall.omegas[pos] = Vec3::new(
                            rand::random::<f32>(),
                            rand::random::<f32>(),
                            rand::random::<f32>(),
                        )
                        .normalize();
                    }
                    // play wall break sound
                    let wall_c = self.wall.body[self.pw[0].b].c;
                    let wall_posn = [wall_c.x, wall_c.y, wall_c.z];
                    match self.wall.wall_type {
                        WallType::Diamond => {
                            let file = std::fs::File::open("content/wallBreakSound.wav").unwrap();
                            let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                            let source = source.amplify(1.5);
                            let sound = self
                                .audio
                                .scene
                                .play_at(source.convert_samples(), wall_posn);
                            self.audio.sound2 = Some(sound);
                        }
                        WallType::Glass => {
                            let file =
                                std::fs::File::open("content/wallBreakSoundGlass.mp3").unwrap();
                            let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                            let source = source.amplify(1.5);
                            let sound = self
                                .audio
                                .scene
                                .play_at(source.convert_samples(), wall_posn);
                            self.audio.sound3 = Some(sound);
                        }
                    }
                    // TODO: record and write score to file
                    // reset score and player position
                    // self.score = 0;
                    self.player.body.c = Pos3::new(0.0, PBHS, 0.0);
                } else if self.wall.body[0].c.z + WBHS < self.player.body.c.z - 2.0 * WBHS {
                    // if wall passes camera, increment score and reset wall
                    self.score += 1;
                    if self.score > self.high_score {
                        self.high_score = self.score;
                    }
                    self.wall.reset(self.score);
                    // reset wall sound
                    self.audio.sound4.as_mut().unwrap().stop();
                    let file = std::fs::File::open("content/wallTrainSound.mp3").unwrap();
                    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                    let source = source.amplify(3.0);
                    let sound = self
                        .audio
                        .scene
                        .play_at(source.convert_samples(), [0.0, 0.0, WIZ]);
                    self.audio.sound4 = Some(sound);
                }
            }
            Mode::EndScreen => {
                // if player hits play again menu object, start game
                if !self.ps.is_empty() {
                    self.mode = Mode::GamePlay;
                    // reset wall and player position and score
                    self.player.body.c = Pos3::new(0.0, PBHS, 0.0);
                    self.wall.reset(self.score);
                    self.score = 0;
                    // start playing wall sound
                    let file = std::fs::File::open("content/wallTrainSound.mp3").unwrap();
                    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                    let source = source.amplify(3.0);
                    let sound = self
                        .audio
                        .scene
                        .play_at(source.convert_samples(), [0.0, 0.0, WIZ]);
                    self.audio.sound4 = Some(sound);
                }
                // if player hits load save object, load save
                if !self.pl.is_empty() {
                    self.mode = Mode::GamePlay;
                    self.load_game();
                    // start playing wall sound
                    let file = std::fs::File::open("content/wallTrainSound.mp3").unwrap();
                    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                    let source = source.amplify(3.0).repeat_infinite();
                    let sound = self
                        .audio
                        .scene
                        .play_at(source.convert_samples(), [0.0, 0.0, WIZ]);
                    self.audio.sound4 = Some(sound);
                }

                // clear wall blocks from view once they get far away
                // let mut to_keep: Vec<bool> = Vec::new();
                // for i in 0..self.wall.body.len() {
                // if (self.wall.body[i].c - self.player.body.c).magnitude() < 50.0 {
                // to_keep.push(true);
                // } else {
                // to_keep.push(false);
                // }
                // }
                // self.wall.body.retain(|_| *to_keep.iter().next().unwrap());
                // self.wall.vels.retain(|_| *to_keep.iter().next().unwrap());
            }
        }

        self.camera.update_camera(engine.camera_mut());
    }
    fn load_game(&mut self) {
        let file = File::open("savefile.txt").unwrap();
        let buf_reader = BufReader::new(file);
        let save_state: GameState = serde_json::from_reader(buf_reader).unwrap();

        // generate wall
        let (boxes, missing_x, missing_y) = Wall::generate_components(
            save_state.wall_z,
            Matrix3::one(),
            Some((save_state.missing_x, save_state.missing_y)),
        );
        self.wall.body = boxes;
        self.wall.missing_x = missing_x;
        self.wall.missing_y = missing_y;
        let n_boxes = self.wall.body.len();
        self.wall.vels = vec![WIV * (save_state.score + 1) as f32 * WVSF; n_boxes];
        self.wall.rots = vec![Quat::new(1.0, 0.0, 0.0, 0.0); n_boxes];
        self.wall.omegas = vec![Vec3::zero(); n_boxes];
        self.wall.control = (0, 0);

        // load player posn and score
        self.player.body.c = save_state.player_posn;
        self.score = save_state.score;
    }
}

fn main() {
    env_logger::init();
    let title = env!("CARGO_PKG_NAME");
    let window = winit::window::WindowBuilder::new().with_title(title);
    run::<GameData, Game<OrbitCamera>>(window, std::path::Path::new("content"));
}
