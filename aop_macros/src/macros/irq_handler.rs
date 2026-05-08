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
    let priority = config.priority;

    let expanded = quote! {
        #vis #sig {
            crate::core::log::log_event("trace", &format!("[IRQ:{}] handler enter: {}", #priority, #name));
            let __irq_ts_start = crate::core::time::cycle_count();
            let __irq_res = (|| #block)();
            let __irq_ts_end = crate::core::time::cycle_count();
            let __irq_elapsed = __irq_ts_end.saturating_sub(__irq_ts_start);
            if __irq_elapsed > 10000 {
                crate::core::log::log_event("warn", &format!("[IRQ:{}] handler {} took {} cycles (CRITICAL)", #priority, #name, __irq_elapsed));
            } else {
                crate::core::log::log_event("trace", &format!("[IRQ:{}] handler exit: {} ({} cycles)", #priority, #name, __irq_elapsed));
            }
            __irq_res
        }
    };

    TokenStream::from(expanded)
}
