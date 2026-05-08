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
    let is_async = sig.asyncness.is_some();

    let body = if is_async {
        quote! {
            let __perf_start = crate::core::time::cycle_count();
            let __perf_res = async move { #block }.await;
            let __perf_end = crate::core::time::cycle_count();
            let __perf_elapsed = __perf_end.saturating_sub(__perf_start);
            
            crate::aop::perf_trace::record_metric(#name, __perf_elapsed, #threshold);
            
            if __perf_elapsed > #threshold {
                crate::core::log::log_event("warn", &format!("[PERF] {} exceeded threshold: {} / {} cycles", #name, __perf_elapsed, #threshold));
            }
            __perf_res
        }
    } else {
        quote! {
            let __perf_start = crate::core::time::cycle_count();
            let __perf_res = (|| #block)();
            let __perf_end = crate::core::time::cycle_count();
            let __perf_elapsed = __perf_end.saturating_sub(__perf_start);
            
            crate::aop::perf_trace::record_metric(#name, __perf_elapsed, #threshold);
            
            if __perf_elapsed > #threshold {
                crate::core::log::log_event("warn", &format!("[PERF] {} exceeded threshold: {} / {} cycles", #name, __perf_elapsed, #threshold));
            }
            __perf_res
        }
    };

    let expanded = quote! {
        #vis #sig {
            #body
        }
    };

    TokenStream::from(expanded)
}
