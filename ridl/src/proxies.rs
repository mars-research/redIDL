use crate::error;
use error::Result;
use std::path;
use std::io::Write;

const PREAMBLE: &str = "\
    use proxy;\n\
    use usr;\n\
    use create;\n\
    use rref::{RRef, RRefDeque};\n\
    use alloc::boxed::Box;\n\
    use alloc::sync::Arc;\n\
    use libsyscalls::syscalls::{sys_get_current_domain_id, sys_update_current_domain_id};\n\
    use syscalls::{Heap, Domain, Interrupt};\n\
    use usr::{bdev::{BDev, BSIZE}, vfs::{UsrVFS, VFS}, xv6::Xv6, dom_a::DomA, dom_c::DomC, net::Net, pci::{PCI, PciBar, PciResource}};\n\
    use usr::rpc::{RpcResult, RpcError};\n\
    use usr::error::Result;\n\
    use console::{println, print};\n\
    use unwind::trampoline;\n";

// *Shadow are special-cased

pub fn generate(root: &path::Path) -> Result<()> {
    let _idl_root = crate::open_subdir(root, "sys/interfaces/usr/");
    let mut proxy_file = crate::create_subfile(root, "usr/proxy/src/_gen.rs")?;
    writeln!(proxy_file, "{}", PREAMBLE)?;
    Ok(())
}