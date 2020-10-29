#[macro_use]
use codegen_lib::generate_trampoline;

// fn asd(expand_param_list!(asd, u8): u32) {}


generate_trampoline!(s: usr::dom_c::DomC, fn yeet(asd: u8) -> u8);

fn main() {
    println!("Hello, world!");
}
