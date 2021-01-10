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
macro_rules! add_attribute {
    ($item: ident, $attr: literal) => {
        $item.attrs.push(
            parse_quote! {
                $attr
            }
        );
    };
}

#[macro_export]
macro_rules! for_enums_add_attribute {
    ($item: ident, $attr: literal, $($variant: path)*) => {
        match $item {
            $($variant(x) => crate::add_attribute!(x, $attr),)*
            _ => {},
        }
    };
}

