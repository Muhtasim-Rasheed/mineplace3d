//! A module for working with the 3 basis axes.
//!
//! This module provides the [`Axis`] enum, which represents the 3 basis axes: X, Y, and Z.
//! Mineplace3D has traditionally followed the right-handed rule, so X points east, Y points
//! up and Z points north.

mod convert;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];
}
