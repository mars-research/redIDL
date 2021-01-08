#![no_std]
#![feature(extended_key_value_attributes)]

#[macro_use]
extern crate codegen_proc;

// fn no_arg(&self) -> u8;
// fn one_rref(&mut self, x: u8) -> u8;
// fn a1(&mut self, x: u8);
// fn a2(&mut self,);
// fn read(&self, bus: u8, dev: u8, func: u8, offset: u8) -> u32;


#[redidl_resolve_module_and_generate_proxy]
pub trait DomC {

    fn write(&self, bus: u8, dev: u8, func: u8, offset: u8, value: u32);
}


# [ass = module_path ! ()]
// #[generate_proxy = "f"]
pub struct Foo {

}



fn main() {
    
}
