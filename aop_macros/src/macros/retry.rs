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
    let retries = config.retries.max(1);
    let is_async = sig.asyncness.is_some();

    let backoff_logic = match config.backoff.as_str() {
        "exponential" => quote! {
            let __delay = (100 << __attempts).min(5000); // 100ms * 2^n, max 5s
            crate::core::time::delay_ms(__delay);
        },
        "fixed" => quote! {
            crate::core::time::delay_ms(100); // Fixed 100ms
        },
        _ => quote! {}, // No delay
    };

    let body = if is_async {
        quote! {
            let mut __attempts = 0;
            loop {
                let __res = async move { #block }.await;
                if __res.is_ok() || __attempts >= #retries {
                    return __res;
                }
                __attempts += 1;
                crate::core::log::log_event("warn", &format!("[RETRY] attempt {} failed, retrying...", __attempts));
                #backoff_logic
            }
        }
    } else {
        quote! {
            let mut __attempts = 0;
            loop {
                let __res = (|| #block)();
                if __res.is_ok() || __attempts >= #retries {
                    return __res;
                }
                __attempts += 1;
                crate::core::log::log_event("warn", &format!("[RETRY] attempt {} failed, retrying...", __attempts));
                #backoff_logic
            }
        }
    };

    let expanded = quote! {
        #vis #sig {
            #body
        }
    };

    TokenStream::from(expanded)
}
