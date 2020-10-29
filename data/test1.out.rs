#[no_mangle]
extern fn one_arg(s: &Box<dyn usr::dom_c::DomC>, x: usize) -> RpcResult<usize> {
    //println!("one_arg: x:{}", x);
    s.one_arg(x)
}

#[no_mangle]
extern fn one_arg_err(s: &Box<dyn usr::dom_c::DomC>, x: usize) -> RpcResult<usize> {
    println!("one_arg was aborted, x:{}", x);
    Err(unsafe{RpcError::panic()})
}

#[no_mangle]
extern "C" fn one_arg_addr() -> u64 {
    one_arg_err as u64
}

extern {
    fn one_arg_tramp(s: &Box<dyn usr::dom_c::DomC>, x: usize) -> RpcResult<usize>;
}

trampoline!(one_arg);
