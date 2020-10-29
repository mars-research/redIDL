// #[macro_export]
// macro_rules! generate_trampoline1 {
//     ($id: ident, $param_list: tt) => {
//         fn $id($id $crate::expand_param_list!($param_list)){}
//     };
// }

#[macro_export]
macro_rules! generate_trampoline2 {
    (fn $func:ident($($arg:ident : $ty:ty),*)) => {
        fn $func(self, $($arg: $ty,)*) 
        {
            unimplemented!()
        }
    };
}

#[macro_export]
macro_rules! generate_trampoline1 {
    ($id:ident,$($param:ident,$type:ty),*) => {
        
        // fn $id($($param, $type),*){}
        // let arr = [$($param),+];
    };
}

#[macro_export]
macro_rules! expand_param_list {
    ($param:expr, $type:ty) => {
        $param
    };
    // ($($param:expr, $type:ty), +) => {
    //     $param: $type, +
    // };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
