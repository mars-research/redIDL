pub mod bdev {
    /// RedLeaf block device interface
    use crate::rref::{RRefDeque};
    use crate::rpc::RpcResult;
    pub struct BlkReq {}
    #[interface]
    pub trait NvmeBDev: Send {
        fn poll_rref(
            &self,
            collect: RRefDeque<BlkReq, 1024>,
        ) -> RpcResult<(usize, RRefDeque<BlkReq, 1024>)>;
    }
}
pub mod rpc {
    /// `RpcResult` is a wrapper around the `Result` type. It forces the users
    /// can only return an `Ok` and an `RpcError` must be raise by the proxy(trusted)
    pub type RpcResult<T> = Result<T, RpcError>;
    /// A wrapper that hides the ErrorEnum
    pub struct RpcError {
        error: ErrorEnum,
    }
}
pub mod rref {
    #![no_std]
    pub mod rref {
        pub struct RRef<T>
        {
            pub(crate) value_pointer: *mut T,
        }
    }
    pub mod rref_deque {
        use super::rref_array::RRefArray;
        pub struct RRefDeque<T: RRefable, const N: usize>
        {
            arr: RRefArray<T, N>,
        }
    }
    pub mod rref_array {
        use super::rref::RRef;
        pub struct RRefArray<T, const N: usize>
        {
            arr: RRef<[Option<RRef<T>>; N]>,
        }
    }

    pub use self::rref::RRef;
    pub use self::rref_array::RRefArray;
    pub use self::rref_deque::RRefDeque;
}

