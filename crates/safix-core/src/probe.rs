//! Compile-time probes for traits a type must *not* implement.
//!
//! A claim that [`Secret`](crate::Secret) has no `Debug` is worth nothing as a
//! sentence: the day someone derives one, every sentence saying otherwise keeps
//! reading exactly as it did, and the review that would have caught it is the
//! review that did not happen. So the claim is compiled.
//!
//! The mechanism is inherent-impl priority. `Implements<T>` has a bounded
//! inherent associated constant for each trait, and an unbounded trait constant
//! of the same name. When `T` satisfies the bound, the inherent constant applies
//! and resolves to `true`; when it does not, the inherent candidate is discarded
//! and the trait's default `false` is what the path resolves to.
//!
//! Each probe is consumed by a `const` assertion, so a trait appearing where one
//! of these says it is absent fails the build rather than a review.

use core::marker::PhantomData;

/// The probe carrier. Never constructed; only its associated constants are read.
pub struct Implements<T: ?Sized>(PhantomData<T>);

/// The `false` case for the `Debug` probe.
pub trait DebugFallback {
    /// Whether the probed type implements `Debug`.
    const DEBUG: bool = false;
}

impl<T: ?Sized> DebugFallback for Implements<T> {}

impl<T: ?Sized + core::fmt::Debug> Implements<T> {
    /// Whether the probed type implements `Debug`.
    pub const DEBUG: bool = true;
}

/// The `false` case for the `Display` probe.
pub trait DisplayFallback {
    /// Whether the probed type implements `Display`.
    const DISPLAY: bool = false;
}

impl<T: ?Sized> DisplayFallback for Implements<T> {}

impl<T: ?Sized + core::fmt::Display> Implements<T> {
    /// Whether the probed type implements `Display`.
    pub const DISPLAY: bool = true;
}

/// The `false` case for the `Serialize` probe.
pub trait SerializeFallback {
    /// Whether the probed type implements `serde::Serialize`.
    const SERIALIZE: bool = false;
}

impl<T: ?Sized> SerializeFallback for Implements<T> {}

impl<T: ?Sized + serde::Serialize> Implements<T> {
    /// Whether the probed type implements `serde::Serialize`.
    pub const SERIALIZE: bool = true;
}

/// The `false` case for the `From<String>` probe.
pub trait FromStringFallback {
    /// Whether the probed type is constructible from an owned string.
    const FROM_STRING: bool = false;
}

impl<T: ?Sized> FromStringFallback for Implements<T> {}

impl<T: From<String>> Implements<T> {
    /// Whether the probed type is constructible from an owned string.
    pub const FROM_STRING: bool = true;
}

/// The `false` case for the `From<&str>` probe.
pub trait FromStrFallback {
    /// Whether the probed type is constructible from a borrowed string.
    const FROM_STR: bool = false;
}

impl<T: ?Sized> FromStrFallback for Implements<T> {}

impl<T: for<'borrow> From<&'borrow str>> Implements<T> {
    /// Whether the probed type is constructible from a borrowed string.
    pub const FROM_STR: bool = true;
}

// The probe's own severity, asserted in the library rather than in a test: a
// probe that answers `false` unconditionally would satisfy every absence
// assertion in this crate while detecting nothing. These three say it can also
// answer `true`, and they are the reason the bounded constants above are
// reachable in a build with no tests in it.
const _: () = assert!(
    Implements::<u8>::DEBUG,
    "the probe must report a Debug that is present"
);
const _: () = assert!(
    Implements::<u8>::DISPLAY,
    "the probe must report a Display that is present"
);
const _: () = assert!(
    Implements::<u8>::SERIALIZE,
    "the probe must report a Serialize that is present"
);
const _: () = assert!(
    Implements::<String>::FROM_STRING,
    "the probe must report a From<String> that is present"
);
const _: () = assert!(
    Implements::<String>::FROM_STR,
    "the probe must report a From<&str> that is present"
);

#[cfg(test)]
mod tests {
    use super::{
        DebugFallback as _, DisplayFallback as _, FromStrFallback as _, FromStringFallback as _,
        Implements, SerializeFallback as _,
    };

    #[test]
    fn the_probe_reports_a_trait_that_is_absent() {
        struct Bare;

        assert!(!Implements::<Bare>::DEBUG);
        assert!(!Implements::<Bare>::DISPLAY);
        assert!(!Implements::<Bare>::SERIALIZE);
        assert!(!Implements::<Bare>::FROM_STRING);
        assert!(!Implements::<Bare>::FROM_STR);
    }

    #[test]
    fn the_probe_reports_a_trait_that_is_present() {
        assert!(Implements::<u8>::DEBUG);
        assert!(Implements::<u8>::DISPLAY);
        assert!(Implements::<u8>::SERIALIZE);
        assert!(Implements::<String>::FROM_STRING);
        assert!(Implements::<String>::FROM_STR);
    }
}
