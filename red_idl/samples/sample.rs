trait Device {
	fn reset() -> RpcResult<()>;
	fn get_buffer_size() -> RpcResult<usize>;
	fn foo() -> RpcResult<&dyn Device>;
}