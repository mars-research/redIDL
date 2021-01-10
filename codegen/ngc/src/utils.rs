#[macro_export]
macro_rules! has_attribute {
    ($item: ident, $attr: literal) => {
        {
            let mut ret = false;
            for attr in &$item.attrs {
                if let Ok(meta) = attr.parse_meta() {
                    if meta.path().is_ident($attr) {
                        ret = true;
                        break;
                    }
                } 
            }
            ret
        }
    };
}

#[macro_export]
macro_rules! remove_attribute {
    ($item: ident, $attr: literal) => {
        $item.attrs.retain(|attr| {
            if let Ok(meta) = attr.parse_meta() {
                if meta.path().is_ident($attr) {
                    return false;
                }
            }
            true
        });
    };
}

#[macro_export]
macro_rules! get_proxy_mod {
    () => {
        format_ident!("generated_proxy")
    };
}