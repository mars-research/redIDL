// #[derive(Debug)]
// pub struct ModuleNodeInner {
//     /// Whether the module is public.
//     pub public: bool,
//     /// The module itself.
//     pub module: SymbolTreeNodeOld,
// }

// impl ModuleNodeInner {
//     fn new(public: bool, module: SymbolTreeNodeOld) -> Self {
//         Self {
//             public,
//             module,
//         }
//     }   
// }

// #[derive(Debug, Clone)]
// pub struct ModuleNodeOld {
//     inner: Rc<RefCell<ModuleNodeInner>>,
// }

// impl ModuleNodeOld {
//     pub fn new(public: bool, module: SymbolTreeNodeOld) -> Self {
//         Self {
//             inner: Rc::new(RefCell::new(ModuleNodeInner::new(public, module)))
//         }
//     }

//     pub fn borrow(&self) -> Ref<ModuleNodeInner> {
//         RefCell::borrow(&self.inner)
//     }

//     pub fn borrow_mut(&self) -> RefMut<ModuleNodeInner> {
//         RefCell::borrow_mut(&self.inner)
//     }
// }