//! Shared helpers used across the validation rule modules.

/// Maps a Confidence code to an ordinal where A=5 (highest) and E=1 (lowest).
/// Returns 0 for unrecognized codes — treated as below E, so ceiling checks
/// reject rather than pass silently.
pub(crate) fn confidence_rank(c: &str) -> u8 {
    match c {
        "A" => 5,
        "B" => 4,
        "C" => 3,
        "D" => 2,
        "E" => 1,
        _ => 0,
    }
}
