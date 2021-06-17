pub use cgmath::prelude::*;
use serde::{Deserialize, Serialize};
pub type Vec3 = cgmath::Vector3<f32>;
pub type Pos3 = cgmath::Point3<f32>;
pub type Mat3 = cgmath::Matrix3<f32>;
pub type Mat4 = cgmath::Matrix4<f32>;
pub type Quat = cgmath::Quaternion<f32>;
pub const PI: f32 = std::f32::consts::PI;

pub const EPS: f32 = 0.01;

#[derive(Serialize, Deserialize)]
#[serde(remote = "Vec3")]
pub struct Vec3Def {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Pos3")]
pub struct Pos3Def {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Quat")]
pub struct QuatDef {
    pub s: f32,
    #[serde(with = "Vec3Def")]
    pub v: Vec3,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Mat3")]
pub struct Mat3Def {
    #[serde(with = "Vec3Def")]
    pub x: Vec3,
    #[serde(with = "Vec3Def")]
    pub y: Vec3,
    #[serde(with = "Vec3Def")]
    pub z: Vec3,
}

pub trait Shape {
    fn translate(&mut self, v: Vec3);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sphere {
    pub c: Pos3,
    pub r: f32,
}

impl Shape for Sphere {
    fn translate(&mut self, v: Vec3) {
        self.c += v;
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(remote = "Plane")]
pub struct Plane {
    #[serde(with = "Vec3Def")]
    pub n: Vec3, // unit vector normal
    pub d: f32, // distance of how far along the normal it is
}

impl Shape for Plane {
    fn translate(&mut self, _v: Vec3) {
        panic!();
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Box {
    #[serde(with = "Pos3Def")]
    pub c: Pos3, // center of box
    #[serde(with = "Mat3Def")]
    pub axes: Mat3, // rotation matrix
    #[serde(with = "Vec3Def")]
    pub half_sizes: Vec3, // how far from the center in each direction
}

impl Shape for Box {
    fn translate(&mut self, v: Vec3) {
        self.c += v;
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AABB {
    pub c: Pos3,
    pub half_sizes: Vec3,
}

impl Shape for AABB {
    fn translate(&mut self, v: Vec3) {
        self.c += v;
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ray {
    pub p: Pos3,
    pub dir: Vec3,
}

impl Shape for Ray {
    fn translate(&mut self, v: Vec3) {
        self.p += v;
    }
}

pub trait Collide<S: Shape>: Shape {
    fn touching(&self, s2: &S) -> bool {
        self.disp(s2).is_some()
    }
    fn disp(&self, s2: &S) -> Option<Vec3>;
}

impl Collide<Sphere> for Sphere {
    fn touching(&self, s2: &Sphere) -> bool {
        // Is the (squared) distance between the centers less than the
        // (squared) sum of the radii?
        s2.c.distance2(self.c) <= (self.r + s2.r).powi(2)
    }
    /// What's the offset I'd need to push s1 and s2 out of each other?
    fn disp(&self, s2: &Sphere) -> Option<Vec3> {
        let offset = s2.c - self.c;
        let distance = offset.magnitude();
        if distance < self.r + s2.r {
            // Make sure we don't divide by 0
            let distance = if distance == 0.0 { 1.0 } else { distance };
            // How much combined radius is "left over"?
            let disp_mag = (self.r + s2.r) - distance;
            // Normalize offset and multiply by the amount to push
            Some(offset * (disp_mag / distance))
        } else {
            None
        }
    }
}

impl Collide<Plane> for Sphere {
    fn touching(&self, p: &Plane) -> bool {
        // Find the distance of the sphere's center to the plane
        (self.c.dot(p.n) - p.d).abs() <= self.r
    }
    fn disp(&self, p: &Plane) -> Option<Vec3> {
        // Find the distance of the sphere's center to the plane
        let dist = self.c.dot(p.n) - p.d;
        if dist.abs() <= self.r {
            // If we offset from the sphere position opposite the normal,
            // we'll end up hitting the plane at `dist` units away.  So
            // the displacement is just the plane's normal * dist.
            Some(p.n * (self.r - dist))
        } else {
            None
        }
    }
}

impl Collide<Plane> for Box {
    fn touching(&self, p: &Plane) -> bool {
        // Treat plane as a huge box
        let plane_box = Box {
            c: Pos3::new(0.0, p.d - 50.0, 0.0),
            axes: Mat3::one(),
            half_sizes: Vec3::new(100.0, 50.0, 100.0),
        };

        self.touching(&plane_box)

        /*
        // Find the distance of the box's center to the plane
        let dist = self.c.dot(p.n) - p.d;

        // Check if any vertex is strictly below the plane (not including on the plane)
        let v = self.c
            + self.axes.x * self.half_sizes.x
            + self.axes.y * self.half_sizes.y
            + self.axes.z * self.half_sizes.z;
        if ((v.dot(p.n) - p.d).signum() - dist.signum()).abs() > 1.5 {
            return true;
        }
        let v = self.c + self.axes.x * self.half_sizes.x + self.axes.y * self.half_sizes.y
            - self.axes.z * self.half_sizes.z;
        if ((v.dot(p.n) - p.d).signum() - dist.signum()).abs() > 1.5 {
            return true;
        }
        let v = self.c + self.axes.x * self.half_sizes.x - self.axes.y * self.half_sizes.y
            + self.axes.z * self.half_sizes.z;
        if ((v.dot(p.n) - p.d).signum() - dist.signum()).abs() > 1.5 {
            return true;
        }
        let v = self.c + self.axes.x * self.half_sizes.x
            - self.axes.y * self.half_sizes.y
            - self.axes.z * self.half_sizes.z;
        if ((v.dot(p.n) - p.d).signum() - dist.signum()).abs() > 1.5 {
            return true;
        }
        let v = self.c - self.axes.x * self.half_sizes.x
            + self.axes.y * self.half_sizes.y
            + self.axes.z * self.half_sizes.z;
        if ((v.dot(p.n) - p.d).signum() - dist.signum()).abs() > 1.5 {
            return true;
        }
        let v = self.c - self.axes.x * self.half_sizes.x + self.axes.y * self.half_sizes.y
            - self.axes.z * self.half_sizes.z;
        if ((v.dot(p.n) - p.d).signum() - dist.signum()).abs() > 1.5 {
            return true;
        }
        let v = self.c - self.axes.x * self.half_sizes.x - self.axes.y * self.half_sizes.y
            + self.axes.z * self.half_sizes.z;
        if ((v.dot(p.n) - p.d).signum() - dist.signum()).abs() > 1.5 {
            return true;
        }
        let v = self.c
            - self.axes.x * self.half_sizes.x
            - self.axes.y * self.half_sizes.y
            - self.axes.z * self.half_sizes.z;
        if ((v.dot(p.n) - p.d).signum() - dist.signum()).abs() > 1.5 {
            return true;
        }

        false

        // assumes box is not rotated
        // dist.abs() <= self.half_sizes.x
        // || dist.abs() <= self.half_sizes.y
        // || dist.abs() <= self.half_sizes.z
        */
    }

    fn disp(&self, p: &Plane) -> Option<Vec3> {
        // Find the distance of the box's center to the plane
        // let dist = self.c.dot(p.n) - p.d;

        // if dist.abs() <= self.half_sizes.x {
        // Some(p.n * (self.half_sizes.x - dist))
        // } else if dist.abs() <= self.half_sizes.y {
        // Some(p.n * (self.half_sizes.y - dist))
        // } else if dist.abs() <= self.half_sizes.z {
        // Some(p.n * (self.half_sizes.z - dist))
        // } else {
        // None
        // }

        if self.touching(p) {
            Some(p.n)
        } else {
            None
        }
    }
}

impl Collide<Box> for Box {
    fn touching(&self, b: &Box) -> bool {
        // Oriented bounding box collision detection, based on Ericson pp.103-5
        let mut rot = Mat3::zero();
        let mut absrot = Mat3::zero();
        for i in 0..3 {
            for j in 0..3 {
                rot[j][i] = self.axes[i].dot(b.axes[j]);
            }
        }

        let mut trans = b.c - self.c;
        trans = Vec3::new(
            trans.dot(self.axes[0]),
            trans.dot(self.axes[1]),
            trans.dot(self.axes[2]),
        );

        for i in 0..3 {
            for j in 0..3 {
                absrot[j][i] = rot[j][i].abs() + EPS;
            }
        }

        for i in 0..3 {
            let ra = self.half_sizes[i];
            let rb = b.half_sizes[0] * absrot[0][i]
                + b.half_sizes[1] * absrot[1][i]
                + b.half_sizes[2] * absrot[2][i];
            if trans[i].abs() > ra + rb {
                return false;
            }
        }

        for i in 0..3 {
            let ra = self.half_sizes[0] * absrot[i][0]
                + self.half_sizes[1] * absrot[i][1]
                + self.half_sizes[2] * absrot[i][2];
            let rb = b.half_sizes[i];
            if (trans[0] * rot[i][0] + trans[1] * rot[i][1] + trans[2] * rot[i][2]).abs() > ra + rb
            {
                return false;
            }
        }

        let ra = self.half_sizes[1] * absrot[0][2] + self.half_sizes[2] * absrot[0][1];
        let rb = b.half_sizes[1] * absrot[2][0] + b.half_sizes[2] * absrot[1][0];
        if (trans[2] * rot[0][1] - trans[1] * rot[0][2]).abs() > ra + rb {
            return false;
        }

        let ra = self.half_sizes[1] * absrot[1][2] + self.half_sizes[2] * absrot[1][1];
        let rb = b.half_sizes[0] * absrot[2][0] + b.half_sizes[2] * absrot[0][0];
        if (trans[2] * rot[1][1] - trans[1] * rot[1][2]).abs() > ra + rb {
            return false;
        }

        let ra = self.half_sizes[1] * absrot[2][2] + self.half_sizes[2] * absrot[2][1];
        let rb = b.half_sizes[0] * absrot[1][0] + b.half_sizes[1] * absrot[0][0];
        if (trans[2] * rot[2][1] - trans[1] * rot[2][2]).abs() > ra + rb {
            return false;
        }

        let ra = self.half_sizes[0] * absrot[0][2] + self.half_sizes[2] * absrot[0][0];
        let rb = b.half_sizes[1] * absrot[2][1] + b.half_sizes[2] * absrot[1][1];
        if (trans[0] * rot[0][2] - trans[2] * rot[0][0]).abs() > ra + rb {
            return false;
        }

        let ra = self.half_sizes[0] * absrot[1][2] + self.half_sizes[2] * absrot[1][0];
        let rb = b.half_sizes[0] * absrot[2][1] + b.half_sizes[2] * absrot[0][1];
        if (trans[0] * rot[1][2] - trans[2] * rot[1][0]).abs() > ra + rb {
            return false;
        }

        let ra = self.half_sizes[0] * absrot[2][2] + self.half_sizes[2] * absrot[2][0];
        let rb = b.half_sizes[0] * absrot[1][1] + b.half_sizes[1] * absrot[0][1];
        if (trans[0] * rot[2][2] - trans[2] * rot[2][0]).abs() > ra + rb {
            return false;
        }

        let ra = self.half_sizes[0] * absrot[0][1] + self.half_sizes[1] * absrot[0][0];
        let rb = b.half_sizes[1] * absrot[2][2] + b.half_sizes[2] * absrot[1][2];
        if (trans[1] * rot[0][0] - trans[0] * rot[0][1]).abs() > ra + rb {
            return false;
        }

        let ra = self.half_sizes[0] * absrot[1][1] + self.half_sizes[1] * absrot[1][0];
        let rb = b.half_sizes[0] * absrot[2][2] + b.half_sizes[2] * absrot[0][2];
        if (trans[1] * rot[1][0] - trans[0] * rot[1][1]).abs() > ra + rb {
            return false;
        }

        let ra = self.half_sizes[0] * absrot[2][1] + self.half_sizes[1] * absrot[2][0];
        let rb = b.half_sizes[0] * absrot[1][2] + b.half_sizes[1] * absrot[0][2];
        if (trans[1] * rot[2][0] - trans[0] * rot[2][1]).abs() > ra + rb {
            return false;
        }

        true
    }

    fn disp(&self, b: &Box) -> Option<Vec3> {
        // Ensure self and b are not touching, regardless of orientation
        // Will overcorrect most of the time
        if self.touching(b) {
            let disp = self.c - b.c;
            let dispabs = Vec3::new(disp.x.abs(), disp.y.abs(), disp.z.abs());

            // Get axis of b to consider as normal vector by finding biggest overlap with dispabs
            let overlap_x = disp.dot(b.axes.x).abs();
            let overlap_y = disp.dot(b.axes.y).abs();
            let overlap_z = disp.dot(b.axes.z).abs();

            let mut normal = Vec3::zero();
            if overlap_x > overlap_y && overlap_x > overlap_z {
                normal = b.axes.x * disp.dot(b.axes.x).signum();
            } else if overlap_y > overlap_x && overlap_y > overlap_z {
                normal = b.axes.y * disp.dot(b.axes.y).signum();
            } else {
                normal = b.axes.z * disp.dot(b.axes.z).signum();
            }

            Some(normal.normalize())

            // let final_dist = self.half_sizes.magnitude() + b.half_sizes.magnitude();
            // Some(disp.normalize_to(final_dist) - disp)
        } else {
            None
        }
    }
}

type CastHit = Option<(Pos3, f32)>;

trait Cast<S: Shape> {
    fn cast(&self, s: &S) -> CastHit;
}

impl Cast<Sphere> for Ray {
    fn cast(&self, s: &Sphere) -> CastHit {
        let m = self.p - s.c;
        let b = self.dir.dot(m);
        let c = m.dot(m) - s.r * s.r;
        let discr = b * b - c;
        if (c > 0.0 && b > 0.0) || discr < 0.0 {
            return None;
        }
        let t = (-b - discr.sqrt()).max(0.0);
        Some((self.p + t * self.dir, t))
    }
}
impl Cast<Plane> for Ray {
    fn cast(&self, b: &Plane) -> CastHit {
        let denom = self.dir.dot(b.n);
        if denom == 0.0 {
            return None;
        }
        let t = (b.d - self.p.dot(b.n)) / denom;
        if t >= 0.0 {
            Some((self.p + self.dir * t, t))
        } else {
            None
        }
    }
}
impl Cast<Box> for Ray {
    fn cast(&self, b: &Box) -> CastHit {
        let mut tmin = 0.0_f32;
        let mut tmax = f32::MAX;
        let delta = b.c - self.p;
        for i in 0..3 {
            let axis = b.axes[i];
            let e = axis.dot(delta);
            let mut f = self.dir.dot(axis);
            if f.abs() < f32::EPSILON {
                if -e - b.half_sizes[i] > 0.0 || -e + b.half_sizes[i] < 0.0 {
                    return None;
                }
                f = f32::EPSILON;
            }
            let mut t1 = (e + b.half_sizes[i]) / f;
            let mut t2 = (e - b.half_sizes[i]) / f;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }
        Some((self.p + self.dir * tmin, tmin))
    }
}
impl Cast<AABB> for Ray {
    fn cast(&self, b: &AABB) -> CastHit {
        let mut tmin = 0.0_f32;
        let mut tmax = f32::MAX;
        let min = b.c - b.half_sizes;
        let max = b.c + b.half_sizes;
        for i in 0..3 {
            if self.dir[i].abs() < f32::EPSILON {
                if self.p[i] < min[i] {
                    return None;
                }
                continue;
            }
            let ood = 1.0 / self.dir[i];
            let mut t1 = (min[i] - self.p[i]) * ood;
            let mut t2 = (max[i] - self.p[i]) * ood;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }
        Some((self.p + self.dir * tmin, tmin))
    }
}
