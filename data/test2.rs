#[domain_creation = "asd"]
pub trait CreateDomC: Send + Sync {
    fn create_domain_dom_c(&self) -> (Box<dyn Domain>, Box<dyn DomC>);
    fn recreate_domain_dom_c(&self, dom: Box<dyn Domain>) -> (Box<dyn Domain>, Box<dyn DomC>);
}