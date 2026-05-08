use proc_macro::TokenStream;
use syn::{Lit, Meta, Token, parse::Parser};

/// Advanced configuration for AOP macros.
pub struct AopConfig {
    pub level: String,
    pub threshold: u64,
    pub priority: u32,
    pub target: String,
    pub retries: u32,
    pub silent: bool,
    pub duration: bool,     // Log execution duration
    pub backoff: String,    // Retry backoff strategy (fixed, exponential)
    pub warmup: u32,        // Warmup calls to ignore in metrics
}

impl Default for AopConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            threshold: 1000,
            priority: 0,
            target: "kernel".to_string(),
            retries: 0,
            silent: false,
            duration: false,
            backoff: "fixed".to_string(),
            warmup: 0,
        }
    }
}

pub fn parse_aop_config(attr: TokenStream) -> AopConfig {
    let mut config = AopConfig::default();
    if attr.is_empty() {
        return config;
    }

    let tokens: proc_macro2::TokenStream = attr.into();
    let parser = syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = parser.parse2(tokens).unwrap_or_default();

    for meta in metas {
        match meta {
            Meta::Path(path) => {
                if path.is_ident("trace") { config.level = "trace".to_string(); }
                else if path.is_ident("debug") { config.level = "debug".to_string(); }
                else if path.is_ident("warn") { config.level = "warn".to_string(); }
                else if path.is_ident("error") { config.level = "error".to_string(); }
                else if path.is_ident("silent") { config.silent = true; }
                else if path.is_ident("duration") { config.duration = true; }
            }
            Meta::NameValue(nv) => {
                if nv.path.is_ident("level") {
                    if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = nv.value {
                        config.level = s.value();
                    }
                } else if nv.path.is_ident("threshold") {
                    if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Int(i), .. }) = nv.value {
                        config.threshold = i.base10_parse().unwrap_or(1000);
                    }
                } else if nv.path.is_ident("priority") {
                    if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Int(i), .. }) = nv.value {
                        config.priority = i.base10_parse().unwrap_or(0);
                    }
                } else if nv.path.is_ident("target") {
                    if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = nv.value {
                        config.target = s.value();
                    }
                } else if nv.path.is_ident("retries") {
                    if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Int(i), .. }) = nv.value {
                        config.retries = i.base10_parse().unwrap_or(0);
                    }
                } else if nv.path.is_ident("backoff") {
                    if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = nv.value {
                        config.backoff = s.value();
                    }
                } else if nv.path.is_ident("warmup") {
                    if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Int(i), .. }) = nv.value {
                        config.warmup = i.base10_parse().unwrap_or(0);
                    }
                }
            }
            _ => {}
        }
    }
    config
}
