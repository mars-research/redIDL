extern crate markers;

#[derive(Copy, Clone)]
struct FooCopy {
}

#[derive(Copy, Clone)]
struct Bar {
    foo: FooCopy
}

macros::declare_rrefable!(Bar);
macros::declare_functional!(Bar);
macros::is_copy!(Bar);
macros::is_functional!(Bar);
macros::is_rrefable!(Bar);

fn main() {
    println!("Hello, world!");
}
