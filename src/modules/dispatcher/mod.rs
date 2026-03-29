#[cfg(feature = "dispatcher_buffered")]
pub mod buffered;
#[cfg(feature = "dispatcher_direct")]
pub mod direct;
#[cfg(feature = "dispatcher_managed")]
pub mod managed;

#[cfg(feature = "ipc_message_passing")]
pub mod upcall;
#[cfg(feature = "dispatcher_vectored")]
pub mod vectored;

#[cfg(feature = "dispatcher_buffered")]
pub use buffered::Buffered;
#[cfg(feature = "dispatcher_direct")]
pub use direct::DirectForwarding;
#[cfg(feature = "dispatcher_managed")]
pub use managed::Managed;
#[cfg(feature = "dispatcher_vectored")]
pub use vectored::VectoredDispatcher;

pub mod selector {
    use super::*;

    #[cfg(all(param_dispatcher = "DirectForwarding", feature = "dispatcher_direct"))]
    pub type ActiveDispatcher = DirectForwarding;

    #[cfg(all(param_dispatcher = "Buffered", feature = "dispatcher_buffered"))]
    pub type ActiveDispatcher = Buffered;

    #[cfg(all(param_dispatcher = "Managed", feature = "dispatcher_managed"))]
    pub type ActiveDispatcher = Managed;

    #[cfg(all(param_dispatcher = "Vectored", feature = "dispatcher_vectored"))]
    pub type ActiveDispatcher = VectoredDispatcher;

    #[cfg(not(any(
        all(param_dispatcher = "DirectForwarding", feature = "dispatcher_direct"),
        all(param_dispatcher = "Buffered", feature = "dispatcher_buffered"),
        all(param_dispatcher = "Managed", feature = "dispatcher_managed"),
        all(param_dispatcher = "Vectored", feature = "dispatcher_vectored")
    )))]
    mod internal_fallback {
        use super::*;
        #[cfg(feature = "dispatcher_vectored")]
        pub type Fallback = VectoredDispatcher;

        #[cfg(all(not(feature = "dispatcher_vectored"), feature = "dispatcher_managed"))]
        pub type Fallback = Managed;

        #[cfg(all(
            not(feature = "dispatcher_vectored"),
            not(feature = "dispatcher_managed"),
            feature = "dispatcher_buffered"
        ))]
        pub type Fallback = Buffered;

        #[cfg(all(
            not(feature = "dispatcher_vectored"),
            not(feature = "dispatcher_managed"),
            not(feature = "dispatcher_buffered"),
            feature = "dispatcher_direct"
        ))]
        pub type Fallback = DirectForwarding;

        #[cfg(not(any(
            feature = "dispatcher_vectored",
            feature = "dispatcher_managed",
            feature = "dispatcher_buffered",
            feature = "dispatcher_direct"
        )))]
        pub type Fallback = VectoredDispatcher; // Final fallback (will error if feature missing, but what else?)
    }

    #[cfg(not(any(
        all(param_dispatcher = "DirectForwarding", feature = "dispatcher_direct"),
        all(param_dispatcher = "Buffered", feature = "dispatcher_buffered"),
        all(param_dispatcher = "Managed", feature = "dispatcher_managed"),
        all(param_dispatcher = "Vectored", feature = "dispatcher_vectored")
    )))]
    pub type ActiveDispatcher = internal_fallback::Fallback;
}
