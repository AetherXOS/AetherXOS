use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, FnArg, Pat};
use crate::config::parse_aop_config;

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let config = parse_aop_config(attr);
    
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let name = sig.ident.to_string();
    let level = config.level;

    let mut arg_names = Vec::new();
    for arg in &sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                arg_names.push(pat_ident.ident.clone());
            }
        }
    }

    let format_string = format!("[ARGS] {}({})", name, arg_names.iter().map(|n| format!("{}={{:?}}, ", n)).collect::<Vec<_>>().join(""));

    let log_args = quote! {
        crate::core::log::log_event(#level, &format!(#format_string, #( &#arg_names ),* ));
    };

    let expanded = quote! {
        #vis #sig {
            #log_args
            #block
        }
    };

    TokenStream::from(expanded)
}
