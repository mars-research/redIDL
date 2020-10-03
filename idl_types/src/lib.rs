#![feature(min_const_generics)]

mod exchangeable;
mod rrefable;

use exchangeable::Exchangeable;

// And RRef-style types check for this

pub trait Proxy {}

trait Device {

}

impl Proxy for dyn Device {}

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
        crate::_check_type::<([i32; 4], [i32; 5])>();
    }
}
