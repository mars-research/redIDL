#![feature(min_const_generics)]

trait Exchangeable {}

trait Proxy {}

trait Device {

}

impl Proxy for dyn Device {}

// Make proxy references exchangeable
impl<T: Proxy + ?Sized> Exchangeable for &T {}
impl<T: Proxy + ?Sized> Exchangeable for &mut T {}

impl Exchangeable for i8 {}
impl Exchangeable for i16 {}
impl Exchangeable for i32 {}
impl Exchangeable for i64 {}
impl Exchangeable for i128 {}
impl Exchangeable for isize {}
impl Exchangeable for u8 {}
impl Exchangeable for u16 {}
impl Exchangeable for u32 {}
impl Exchangeable for u64 {}
impl Exchangeable for u128 {}
impl Exchangeable for usize {}
impl Exchangeable for f32 {}
impl Exchangeable for f64 {}
impl Exchangeable for char {}
impl Exchangeable for bool {}

impl<T: Exchangeable, const N: usize> Exchangeable for [T; N] {}
impl<T: Exchangeable> Exchangeable for [T] {}

struct _Foo {

}

impl Device for _Foo {}

fn _test_val<T: Exchangeable>(_: T) {

}

fn _check_type<T: Exchangeable + ?Sized>() {}

fn _unwrap<'a, T: ?Sized>(item: &'a T) -> &'a T {
	item
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        crate::_check_type::<[i32]>();
    }
}
