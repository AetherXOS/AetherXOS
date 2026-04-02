//! Typed identifier helpers.

use core::fmt;

/// Generic typed identifier wrapper.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedId<T>(pub T);

impl<T> TypedId<T> {
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: fmt::Display> fmt::Display for TypedId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Declares a lightweight typed newtype identifier.
#[macro_export]
macro_rules! typed_id {
    ($vis:vis struct $name:ident($inner:ty)) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $name(pub $inner);

        impl $name {
            #[must_use]
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            #[must_use]
            pub fn into_inner(self) -> $inner {
                self.0
            }
        }

        impl core::fmt::Display for $name
        where
            $inner: core::fmt::Display,
        {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
