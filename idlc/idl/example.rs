use crate::sys::{RRef, RRefArray, RpcResult};
use crate::bdev::BDev;

struct Widget {
	a: (u32, u32),
	b: RRefArray<Widget> // Assuming RRef<> becomes nullable
}

trait Foo {
	fn add_widget(&self, dev: &mut BDev, widget: &RRef<Widget>) -> RpcResult<()>
}
