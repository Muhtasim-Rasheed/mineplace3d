//! The core of the Mineplace3D engine. This module contains essential
//! functionalities and definitions required for the engine to operate
//! such as world management, entity handling, etc.

use glam::{IVec3, Vec3};
pub mod block;
pub mod command;
pub mod datapack;
pub mod direction;
pub mod entity;
pub mod item;
pub mod protocol;
pub mod saving;
pub mod server;
pub mod textcomponent;
pub mod uniquequeue;
pub mod world;

pub(crate) fn aabb_overlap(a_min: Vec3, a_max: Vec3, b_min: Vec3, b_max: Vec3) -> bool {
    !(a_max.x <= b_min.x
        || a_min.x >= b_max.x
        || a_max.y <= b_min.y
        || a_min.y >= b_max.y
        || a_max.z <= b_min.z
        || a_min.z >= b_max.z)
}

pub(crate) fn ray_intersect_aabb(
    ray_origin: Vec3,
    ray_dir: Vec3,
    aabb_min: Vec3,
    aabb_max: Vec3,
) -> Option<IVec3> {
    let inv_dir = 1.0 / ray_dir;
    let t1 = (aabb_min - ray_origin) * inv_dir;
    let t2 = (aabb_max - ray_origin) * inv_dir;

    let tmin = t1.min(t2);
    let tmax = t1.max(t2);

    let t_enter = tmin.x.max(tmin.y).max(tmin.z);
    let t_exit = tmax.x.min(tmax.y).min(tmax.z);

    if t_enter < t_exit && t_exit > 0.0 {
        // Determine which face was hit based on which component of t_enter is largest
        if t_enter == tmin.x {
            Some(Vec3::new(-inv_dir.x.signum(), 0.0, 0.0).as_ivec3())
        } else if t_enter == tmin.y {
            Some(Vec3::new(0.0, -inv_dir.y.signum(), 0.0).as_ivec3())
        } else {
            Some(Vec3::new(0.0, 0.0, -inv_dir.z.signum()).as_ivec3())
        }
    } else {
        None
    }
}
