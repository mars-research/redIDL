#[macro_use]
use codegen_lib::generate_trampoline;
use codegen_proc::generate_proxy;

// fn asd(expand_param_list!(asd, u8): u32) {}


// generate_trampoline!(s: usr::dom_c::DomC, fn yeet(asd: u8) -> u8);

#[generate_proxy]
pub trait DomC {
    fn no_arg() -> RpcResult<()>;
    fn one_arg(x: usize) -> RpcResult<usize>;
    fn one_rref(x: RRef<usize>) -> RpcResult<RRef<usize>>;
}

// #[generate_proxy]
// pub trait DomC {
//     fn no_arg(&self) -> RpcResult<()>;
//     fn one_arg(&self, x: usize) -> RpcResult<usize>;
//     fn one_rref(&self, x: RRef<usize>) -> RpcResult<RRef<usize>>;
// }

// #[generate_proxy]
// pub struct Foo {

// }



fn main() {
    println!("Hello, world!");
}
