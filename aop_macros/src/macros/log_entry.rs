use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};
use crate::config::parse_aop_config;

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let config = parse_aop_config(attr);
    
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let name = sig.ident.to_string();
    let level = config.level;
    let target = config.target;
    let is_async = sig.asyncness.is_some();

    let log_enter = if !config.silent {
        quote! { crate::core::log::log_event(#level, &format!("[{}:{}] enter: {}", #target, #level, #name)); }
    } else {
        quote! {}
    };

    let log_exit = if !config.silent {
        if config.duration {
            quote! {
                let __aop_end = crate::core::time::cycle_count();
                let __aop_elapsed = __aop_end.saturating_sub(__aop_start);
                crate::core::log::log_event(#level, &format!("[{}:{}] exit: {} (took {} cycles)", #target, #level, #name, __aop_elapsed));
            }
        } else {
            quote! { crate::core::log::log_event(#level, &format!("[{}:{}] exit: {}", #target, #level, #name)); }
        }
    } else {
        quote! {}
    };

    let start_timer = if config.duration {
        quote! { let __aop_start = crate::core::time::cycle_count(); }
    } else {
        quote! {}
    };

    let body = if is_async {
        quote! {
            #log_enter
            #start_timer
            let __aop_result = async move { #block }.await;
            #log_exit
            __aop_result
        }
    } else {
        quote! {
            #log_enter
            #start_timer
            let __aop_result = (|| #block)();
            #log_exit
            __aop_result
        }
    };

    let expanded = quote! {
        #vis #sig {
            #body
        }
    };

    TokenStream::from(expanded)
}
