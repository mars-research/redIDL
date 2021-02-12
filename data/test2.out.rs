// impl domain_creation::CreateDomC for PDomain {
//     fn create_domain_dom_c(&self) -> (Box<dyn syscalls::Domain>, Box<dyn interface::dom_c::DomC>) {
//         disable_irq();
//         let r = create_domain_dom_c();
//         enable_irq();
//         r
//     }

//     fn recreate_domain_dom_c(
//         &self,
//         dom: Box<dyn syscalls::Domain>,
//     ) -> (Box<dyn syscalls::Domain>, Box<dyn interface::dom_c::DomC>) {
//         disable_irq();
//         let r = create_domain_dom_c(dom);
//         enable_irq();
//         r
//     }
// }

pub fn create_domain_dom_c() -> (Box<dyn syscalls::Domain>, Box<dyn interface::dom_c::DomC>) {
    extern "C" {
        fn _binary_domains_build_dom_c_start();
        fn _binary_domains_build_dom_c_end();
    }

    let binary_range = (
        _binary_domains_build_dom_c_start as *const u8,
        _binary_domains_build_dom_c_end as *const u8,
    );

    let domain_name = "dom_c";

    type UserInit =
        fn(Box<dyn syscalls::Syscall>, Box<dyn syscalls::Heap>) -> Box<dyn interface::dom_c::DomC>;

    let (dom, entry) = unsafe { load_domain(name, binary_range) };

    let user_ep: UserInit = unsafe { core::mem::transmute::<*const (), UserInit>(entry) };

    let pdom = Box::new(PDomain::new(Arc::clone(&dom)));
    let pheap = Box::new(PHeap::new());

    // update current domain id
    let thread = thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    let dom_c = user_ep(pdom, pheap);
    disable_irq();

    // change domain id back
    {
        thread.lock().current_domain_id = old_id;
    }

    println!("domain/{}: returned from entry point", name);
    (Box::new(PDomain::new(Arc::clone(&dom))), dom_c)
}
