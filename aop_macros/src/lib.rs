use proc_macro::TokenStream;

mod config;
mod macros;

#[proc_macro_attribute]
pub fn log_entry(attr: TokenStream, item: TokenStream) -> TokenStream {
    macros::log_entry::expand(attr, item)
}

#[proc_macro_attribute]
pub fn irq_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    macros::irq_handler::expand(attr, item)
}

#[proc_macro_attribute]
pub fn perf_trace(attr: TokenStream, item: TokenStream) -> TokenStream {
    macros::perf_trace::expand(attr, item)
}

#[proc_macro_attribute]
pub fn trace_args(attr: TokenStream, item: TokenStream) -> TokenStream {
    macros::trace_args::expand(attr, item)
}

#[proc_macro_attribute]
pub fn retry(attr: TokenStream, item: TokenStream) -> TokenStream {
    macros::retry::expand(attr, item)
}

#[proc_macro_attribute]
pub fn lock_monitor(attr: TokenStream, item: TokenStream) -> TokenStream {
    macros::lock_monitor::expand(attr, item)
}

#[proc_macro_attribute]
pub fn precondition(attr: TokenStream, item: TokenStream) -> TokenStream {
    macros::contracts::expand_precondition(attr, item)
}

#[proc_macro_attribute]
pub fn postcondition(attr: TokenStream, item: TokenStream) -> TokenStream {
    macros::contracts::expand_postcondition(attr, item)
}
