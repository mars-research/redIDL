extern crate syn;

fn process_trait(tr: &syn::ItemTrait, tc: &mut crate::types::TypeSystemDecls) -> bool {
    for tr_item in &tr.items {
        if let syn::TraitItem::Method(func) = tr_item {
            if !tc.process_signature(&func.sig) {
                return false
            }
        }
        else {
            panic!() // Should never happen (msg me when it does)
        }
    }

    true
}

pub fn generate_signature_checks(item: &syn::Item, tc: &mut crate::types::TypeSystemDecls) -> bool {
    match item {
        syn::Item::Trait(tr) => process_trait(tr, tc),
        _ => true
    }
}
