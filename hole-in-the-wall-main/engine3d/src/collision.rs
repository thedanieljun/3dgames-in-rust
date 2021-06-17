use serde::{Serialize, Deserialize};
use crate::geom::*;

#[derive(Clone, Copy, Debug)]
#[derive(Serialize, Deserialize)]
pub struct Contact<T: Copy> {
    pub a: T,
    pub b: T,
    #[serde(with = "Vec3Def")]
    pub mtv: Vec3,
}

pub fn restitute_dyn_stat<S1: Shape, S2: Shape>(
    ashapes: &mut [S1],
    avel: &mut [Vec3],
    bshapes: &[S2],
    contacts: &mut [Contact<usize>],
    bounce: bool,
) where
    S1: Collide<S2>,
{
    contacts.sort_unstable_by(|a, b| b.mtv.magnitude2().partial_cmp(&a.mtv.magnitude2()).unwrap());
    for c in contacts.iter() {
        let a = c.a;
        let b = c.b;

        if let Some(disp) = ashapes[a].disp(&bshapes[b]) {
            // ashapes[a].translate(disp);
            // avels[a] += disp;
            // println!("prev {:?}", avel[a]);
            if bounce {
                avel[a] -= 2.0 * disp.dot(avel[a]) * disp;
            } else {
                avel[a] -= disp.dot(avel[a]) * disp;
            }
            // println!("afte {:?}", avel[a]);
        }
    }
}

pub fn restitute_dyn_dyn<S1: Shape, S2: Shape>(
    ashapes: &mut [S1],
    avels: &mut [Vec3],
    bshapes: &mut [S2],
    bvels: &mut [Vec3],
    contacts: &mut [Contact<usize>],
) where
    S1: Collide<S2>,
{
    contacts.sort_unstable_by(|a, b| b.mtv.magnitude2().partial_cmp(&a.mtv.magnitude2()).unwrap());
    // That can bump into each other in perfectly elastic collisions!
    for c in contacts.iter() {
        let a = c.a;
        let b = c.b;
        // Just split the difference.  In crowded situations this will
        // cause issues, but those will always be hard to solve with
        // this kind of technique.
        if let Some(disp) = ashapes[a].disp(&bshapes[b]) {
            // ashapes[a].translate(-disp / 2.0);

            let vel_diff = disp.dot(avels[a]).abs() * disp + disp.dot(bvels[b]).abs() * disp;
            avels[a] += vel_diff;
            bvels[b] -= vel_diff;
            // avels[a] += disp.dot(avels[a]).abs() * disp;
            // bshapes[b].translate(disp / 2.0);
            // bvels[b] -= disp.dot(bvels[b]).abs() * disp;
        }
    }
}

pub fn restitute_dyns<S1: Shape>(
    ashapes: &mut [S1],
    avels: &mut [Vec3],
    contacts: &mut [Contact<usize>],
) where
    S1: Collide<S1>,
{
    contacts.sort_unstable_by(|a, b| b.mtv.magnitude2().partial_cmp(&a.mtv.magnitude2()).unwrap());
    // That can bump into each other in perfectly elastic collisions!
    for c in contacts.iter() {
        let a = c.a;
        let b = c.b;
        // Just split the difference.  In crowded situations this will
        // cause issues, but those will always be hard to solve with
        // this kind of technique.
        if let Some(disp) = ashapes[a].disp(&ashapes[b]) {
            let vel_diff = disp.dot(avels[a]).abs() * disp + disp.dot(avels[b]).abs() * disp;
            avels[a] += vel_diff;
            avels[b] -= vel_diff;
            // ashapes[a].translate(-disp / 2.0);
            // avels[a] -= disp / 2.0;
            // ashapes[b].translate(disp / 2.0);
            // avels[b] += disp / 2.0;
        }
    }
}

pub fn gather_contacts_ab<S1: Shape, S2: Shape>(a: &[S1], b: &[S2], into: &mut Vec<Contact<usize>>)
where
    S1: Collide<S2>,
{
    for (ai, a) in a.iter().enumerate() {
        for (bi, b) in b.iter().enumerate() {
            if let Some(disp) = a.disp(b) {
                into.push(Contact {
                    a: ai,
                    b: bi,
                    mtv: disp,
                });
            }
        }
    }
}

pub fn gather_contacts_aa<S1: Shape>(ss: &[S1], into: &mut Vec<Contact<usize>>)
where
    S1: Collide<S1>,
{
    for (ai, a) in ss.iter().enumerate() {
        for (bi, b) in ss[(ai + 1)..].iter().enumerate() {
            let bi = ai + 1 + bi;
            if let Some(disp) = a.disp(b) {
                into.push(Contact {
                    a: ai,
                    b: bi,
                    mtv: disp,
                });
            }
        }
    }
}
