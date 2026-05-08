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
    let threshold = config.threshold;

    let expanded = quote! {
        #vis #sig {
            let __lock_start = crate::core::time::cycle_count();
            let __res = (|| #block)();
            let __lock_end = crate::core::time::cycle_count();
            let __lock_held = __lock_end.saturating_sub(__lock_start);
            
            crate::aop::lock_monitor::record_lock_stats(#name, 0, __lock_held);
            
            if __lock_held > #threshold {
                crate::core::log::log_event("warn", &format!("[LOCK] {} held for too long: {} cycles", #name, __lock_held));
            }
            __res
        }
    };

    TokenStream::from(expanded)
}
