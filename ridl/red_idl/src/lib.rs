extern crate macros;
extern crate static_assertions;

pub use macros::*;
pub use static_assertions::*;

// Marks traits that contain only members
pub trait Functional {}
pub trait RRefable {}
pub trait SafeCopy {}

impl RRefable for SafeCopy {}

impl SafeCopy for bool {}
impl SafeCopy for u8 {}
impl SafeCopy for u16 {}
impl SafeCopy for u32 {}
impl SafeCopy for u64 {}
impl SafeCopy for u128 {}
impl SafeCopy for usize {}
impl SafeCopy for i8 {}
impl SafeCopy for i16 {}
impl SafeCopy for i32 {}
impl SafeCopy for i64 {}
impl SafeCopy for i128 {}
impl SafeCopy for isize {}
