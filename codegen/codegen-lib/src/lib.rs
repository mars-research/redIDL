#![no_std]
pub use paste::paste;

#[macro_export]
macro_rules! generate_trampoline {
    ($dom:ident : $dom_type:ty, fn $func:ident($($arg:ident : $ty:ty),*) -> $ret:ty) => {
        $crate::paste! {
            #[no_mangle]
            extern fn $func($dom: $dom_type, $($arg: $ty,)*) -> $ret {
                $dom.$func($($arg), *)
            }

            #[no_mangle]
            extern fn [<$func _err>]($dom: $dom_type, $($arg: $ty,)*) -> $ret {
                #[cfg(feature = "proxy-log-error")]
                ::console::println!("proxy: {} aborted", stringify!($func));

                Err(unsafe{::usr::rpc::RpcError::panic()})
            }

            #[no_mangle]
            extern "C" fn [<$func _addr>]() -> u64 {
                [<$func _err>] as u64
            }

            extern {
                fn [<$func _tramp>]($dom: $dom_type, $($arg: $ty,)*) -> $ret;
            }

            trampoline!($func);
        }
    };
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
