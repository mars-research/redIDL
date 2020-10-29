#[macro_use]
use codegen_lib::{generate_trampoline2, expand_param_list};

// fn asd(expand_param_list!(asd, u8): u32) {}

generate_trampoline2!(fn yeet(asd: u8));

fn main() {
    println!("Hello, world!");
}
