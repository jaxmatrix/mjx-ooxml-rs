//! Hand-written support for the generated OOXML types: the wire-parse error and the boolean
//! normalizers referenced by the generated two-valued type aliases.

/// Returned when a string is not a valid wire token for an enumerated OOXML type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownWireValue {
    value: String,
}

impl UnknownWireValue {
    /// Records an unrecognized wire value.
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
        }
    }

    /// The offending value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl core::fmt::Display for UnknownWireValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unknown OOXML wire value: {:?}", self.value)
    }
}

impl std::error::Error for UnknownWireValue {}

/// Normalizer for `ST_OnOff` — accepts `true`/`false`/`1`/`0`/`on`/`off`; writes `true`/`false`.
pub mod on_off {
    /// Parses any accepted spelling to a boolean, or `None` if unrecognized.
    #[must_use]
    pub fn from_wire(s: &str) -> Option<bool> {
        match s {
            "true" | "1" | "on" => Some(true),
            "false" | "0" | "off" => Some(false),
            _ => None,
        }
    }

    /// The canonical wire spelling for a boolean.
    #[must_use]
    pub fn to_wire(value: bool) -> &'static str {
        if value {
            "true"
        } else {
            "false"
        }
    }
}

/// Normalizer for `ST_TrueFalse` — accepts `t`/`f`/`true`/`false` (any case); writes `true`/`false`.
pub mod true_false {
    /// Parses any accepted spelling to a boolean, or `None` if unrecognized.
    #[must_use]
    pub fn from_wire(s: &str) -> Option<bool> {
        match s {
            "t" | "true" | "True" => Some(true),
            "f" | "false" | "False" => Some(false),
            _ => None,
        }
    }

    /// The canonical wire spelling for a boolean.
    #[must_use]
    pub fn to_wire(value: bool) -> &'static str {
        if value {
            "true"
        } else {
            "false"
        }
    }
}

/// Normalizer for `ST_TrueFalseBlank` — like [`true_false`] but the empty string means "unset".
pub mod true_false_blank {
    /// Parses to `Some(bool)`, or `Some(None)` for the blank/unset value.
    ///
    /// Returns the outer `None` only when the string is not a recognized spelling.
    #[must_use]
    pub fn from_wire(s: &str) -> Option<Option<bool>> {
        match s {
            "" => Some(None),
            "t" | "true" | "True" => Some(Some(true)),
            "f" | "false" | "False" => Some(Some(false)),
            _ => None,
        }
    }

    /// The canonical wire spelling (`""` for unset).
    #[must_use]
    pub fn to_wire(value: Option<bool>) -> &'static str {
        match value {
            None => "",
            Some(true) => "true",
            Some(false) => "false",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_off_normalizes_all_spellings() {
        for s in ["true", "1", "on"] {
            assert_eq!(on_off::from_wire(s), Some(true));
        }
        for s in ["false", "0", "off"] {
            assert_eq!(on_off::from_wire(s), Some(false));
        }
        assert_eq!(on_off::from_wire("nope"), None);
        assert_eq!(on_off::to_wire(true), "true");
        assert_eq!(on_off::to_wire(false), "false");
    }

    #[test]
    fn true_false_blank_handles_unset() {
        assert_eq!(true_false_blank::from_wire(""), Some(None));
        assert_eq!(true_false_blank::from_wire("True"), Some(Some(true)));
        assert_eq!(true_false_blank::from_wire("x"), None);
        assert_eq!(true_false_blank::to_wire(None), "");
        assert_eq!(true_false_blank::to_wire(Some(false)), "false");
    }
}
