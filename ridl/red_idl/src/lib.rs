extern crate macros;
extern crate static_assertions;

pub use macros::*;
pub use static_assertions::*;

// Marks traits that contain only members
pub trait Functional {}
pub trait RRefable {}
