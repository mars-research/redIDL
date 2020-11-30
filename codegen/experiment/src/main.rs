#![no_std]

#[macro_use]
// use codegen_lib::generate_trampoline;
extern crate codegen_proc;

use codegen_proc::generate_proxy as interface;

// fn asd(expand_param_list!(asd, u8): u32) {}


// generate_trampoline!(s: usr::dom_c::DomC, fn yeet(asd: u8) -> u8);

#[interface]
#[interface]
pub trait DomC {
    fn no_arg(&self) -> RpcResult<()>;
    fn one_arg(&self, x: usize) -> RpcResult<usize>;
    fn one_rref(&self, x: RRef<usize>) -> RpcResult<RRef<usize>>;
}


// #[generate_proxy]
// pub trait DomC {
//     fn no_arg(&self) -> RpcResult<()>;
//     fn one_arg(&self, x: usize) -> RpcResult<usize>;
//     fn one_rref(&self, x: RRef<usize>) -> RpcResult<RRef<usize>>;
// }

#[generate_proxy]
pub struct Foo {

}



fn main() {
    
}
