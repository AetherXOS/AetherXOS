use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Expr};

pub fn expand_precondition(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let expr: Expr = syn::parse_macro_input!(attr as Expr);
    
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let expr_str = quote!(#expr).to_string();

    let expanded = quote! {
        #vis #sig {
            if !( #expr ) {
                crate::core::log::log_event("error", &format!("[ASSERT] Precondition failed: {}", #expr_str));
                panic!("Precondition failed: {}", #expr_str);
            }
            #block
        }
    };

    TokenStream::from(expanded)
}

pub fn expand_postcondition(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let expr: Expr = syn::parse_macro_input!(attr as Expr);
    
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let expr_str = quote!(#expr).to_string();

    let expanded = quote! {
        #vis #sig {
            let __res = (|| #block)();
            if !( #expr ) {
                crate::core::log::log_event("error", &format!("[ASSERT] Postcondition failed: {}", #expr_str));
                panic!("Postcondition failed: {}", #expr_str);
            }
            __res
        }
    };

    TokenStream::from(expanded)
}
