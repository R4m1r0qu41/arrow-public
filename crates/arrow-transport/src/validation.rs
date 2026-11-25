use chrono::{DateTime, Duration, Utc};
use iso8601_duration::Duration as IsoDuration;

use crate::{envelope::TransportEnvelope, error::TransportError};
use arrow_core::registry::loader::Registries;

pub enum TransportOutcome {
    /// Envelope is well-formed at the transport layer.
    Accepted,
    /// Envelope is rejected at the transport layer.
    Rejected(TransportError),
}

pub struct TransportCheck {
    pub outcome: TransportOutcome,
    /// TPT-040 warnings, e.g. ["TPT-STALE-ENVELOPE"]
    pub warnings: Vec<String>,
}

/// Stateless transport validation — Transport Model Part 8.
/// Implements: TPT-001 (required fields), TPT-002 (version), TPT-003 (timestamp),
/// TPT-004 (channel binding registry), TPT-010 (nonce format), TPT-040 (stale window
/// warning), TPT-041 (future tolerance).
/// TPT-005 (payload structure) is enforced structurally by Rust deserialization before
/// this function is called — no explicit rule check is required.
/// Replay check (TPT-REPLAY-DETECTED) is stateful and handled by the caller.
pub fn validate_envelope(envelope: &TransportEnvelope, registries: &Registries) -> TransportCheck {
    let mut warnings = Vec::new();

    // TPT-002 — version must be "1.0"
    if envelope.envelope_version != "1.0" {
        return TransportCheck {
            outcome: TransportOutcome::Rejected(TransportError::VersionUnsupported(format!(
                "envelope_version `{}` is not supported; expected \"1.0\"",
                envelope.envelope_version
            ))),
            warnings,
        };
    }

    // TPT-001 — required field presence (non-empty strings)
    for (field, value) in [
        ("envelope_id", &envelope.envelope_id),
        ("sender_id", &envelope.sender_id),
        ("channel_binding", &envelope.channel_binding),
        ("sent_at", &envelope.sent_at),
    ] {
        if value.trim().is_empty() {
            return TransportCheck {
                outcome: TransportOutcome::Rejected(TransportError::Malformed(format!(
                    "required field `{field}` is missing or empty"
                ))),
                warnings,
            };
        }
    }

    // TPT-010 — nonce must match ^n-[0-9a-f]{32}$
    if !is_valid_nonce(&envelope.nonce) {
        return TransportCheck {
            outcome: TransportOutcome::Rejected(TransportError::Malformed(format!(
                "nonce `{}` does not match required format ^n-[0-9a-f]{{32}}$ (TPT-010)",
                envelope.nonce
            ))),
            warnings,
        };
    }

    // TPT-001 (receiver_id) — non-empty string or non-empty array of non-empty strings
    match &envelope.receiver_id {
        crate::envelope::ReceiverId::One(s) if s.trim().is_empty() => {
            return TransportCheck {
                outcome: TransportOutcome::Rejected(TransportError::Malformed(
                    "receiver_id is an empty string".to_string(),
                )),
                warnings,
            };
        }
        crate::envelope::ReceiverId::Many(v) if v.is_empty() => {
            return TransportCheck {
                outcome: TransportOutcome::Rejected(TransportError::Malformed(
                    "receiver_id array is empty".to_string(),
                )),
                warnings,
            };
        }
        crate::envelope::ReceiverId::Many(v) if v.iter().any(|s| s.trim().is_empty()) => {
            return TransportCheck {
                outcome: TransportOutcome::Rejected(TransportError::Malformed(
                    "receiver_id array contains one or more empty strings".to_string(),
                )),
                warnings,
            };
        }
        crate::envelope::ReceiverId::Many(_) => {}
        _ => {}
    }

    // TPT-003 — sent_at must parse as RFC 3339
    let sent_at: DateTime<Utc> = match DateTime::parse_from_rfc3339(&envelope.sent_at) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => {
            return TransportCheck {
                outcome: TransportOutcome::Rejected(TransportError::Malformed(format!(
                    "sent_at `{}` is not a valid RFC 3339 timestamp (TPT-003)",
                    envelope.sent_at
                ))),
                warnings,
            }
        }
    };

    // TPT-004 — channel_binding must be registered and active
    match registries
        .channel_bindings
        .get(envelope.channel_binding.as_str())
    {
        None => {
            return TransportCheck {
                outcome: TransportOutcome::Rejected(TransportError::Malformed(format!(
                    "channel_binding `{}` is not a registered binding (TPT-004)",
                    envelope.channel_binding
                ))),
                warnings,
            }
        }
        Some(binding) if binding.status != "active" => {
            return TransportCheck {
                outcome: TransportOutcome::Rejected(TransportError::Malformed(format!(
                    "channel_binding `{}` is not active (status: {}) (TPT-004)",
                    envelope.channel_binding, binding.status
                ))),
                warnings,
            }
        }
        Some(binding) => {
            // Freshness windows from binding, falling back to spec defaults
            let stale_window = parse_duration_or_default(
                binding.freshness_stale_window.as_deref(),
                Duration::hours(24),
            );
            let future_tolerance = parse_duration_or_default(
                binding.freshness_future_tolerance.as_deref(),
                Duration::minutes(5),
            );

            let now = Utc::now();

            // TPT-041 — future tolerance (Fatal)
            if sent_at > now + future_tolerance {
                return TransportCheck {
                    outcome: TransportOutcome::Rejected(TransportError::Stale(format!(
                        "sent_at is beyond the allowed future tolerance of {:?} (TPT-041)",
                        future_tolerance
                    ))),
                    warnings,
                };
            }

            // TPT-040 — stale window (Warning, not rejection)
            if sent_at < now - stale_window {
                warnings.push("TPT-STALE-ENVELOPE".to_string());
            }
        }
    }

    TransportCheck {
        outcome: TransportOutcome::Accepted,
        warnings,
    }
}

/// Parse an ISO 8601 duration string (hour/minute/second components only).
/// Returns the default if the string is None, empty, or parse fails.
fn parse_duration_or_default(s: Option<&str>, default: Duration) -> Duration {
    let s = match s {
        Some(s) if !s.is_empty() => s,
        _ => return default,
    };
    match s.parse::<IsoDuration>() {
        Ok(d) => {
            // Only honour H/M/S components for the pilot (overlay Section 2.8)
            let seconds = (d.hour as i64 * 3600) + (d.minute as i64 * 60) + (d.second as i64);
            if seconds > 0 {
                Duration::seconds(seconds)
            } else {
                default
            }
        }
        Err(_) => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::ReceiverId;
    use arrow_core::{registry::loader::Registries, types::event::ArrowEvent};

    fn registries() -> Registries {
        Registries::load_embedded().unwrap()
    }

    fn valid_event() -> ArrowEvent {
        serde_json::from_str(include_str!("../tests/fixtures/valid-event.json")).unwrap()
    }

    fn base_envelope() -> TransportEnvelope {
        TransportEnvelope {
            envelope_version: "1.0".to_string(),
            envelope_id: "env-test-001".to_string(),
            sender_id: "did:arrow:pilot:node-a".to_string(),
            receiver_id: ReceiverId::One("did:arrow:pilot:node-b".to_string()),
            channel_binding: "http-rest-local".to_string(),
            sent_at: Utc::now().to_rfc3339(),
            nonce: "n-aabbccddeeff00112233445566778899".to_string(),
            sequence: None,
            signature: None,
            signing: None,
            correlation_id: None,
            reply_to: None,
            metadata: None,
            payload: valid_event(),
        }
    }

    #[test]
    fn valid_envelope_accepted() {
        let check = validate_envelope(&base_envelope(), &registries());
        assert!(matches!(check.outcome, TransportOutcome::Accepted));
        assert!(check.warnings.is_empty());
    }

    #[test]
    fn wrong_version_rejected_tpt002() {
        let mut e = base_envelope();
        e.envelope_version = "2.0".to_string();
        let check = validate_envelope(&e, &registries());
        assert!(matches!(
            check.outcome,
            TransportOutcome::Rejected(TransportError::VersionUnsupported(_))
        ));
    }

    #[test]
    fn invalid_timestamp_rejected_tpt003() {
        let mut e = base_envelope();
        e.sent_at = "not-a-date".to_string();
        let check = validate_envelope(&e, &registries());
        assert!(matches!(
            check.outcome,
            TransportOutcome::Rejected(TransportError::Malformed(_))
        ));
    }

    #[test]
    fn unregistered_channel_binding_rejected_tpt004() {
        let mut e = base_envelope();
        e.channel_binding = "grpc".to_string();
        let check = validate_envelope(&e, &registries());
        assert!(matches!(
            check.outcome,
            TransportOutcome::Rejected(TransportError::Malformed(_))
        ));
    }

    #[test]
    fn empty_nonce_rejected_tpt010() {
        let mut e = base_envelope();
        e.nonce = "".to_string();
        let check = validate_envelope(&e, &registries());
        assert!(matches!(
            check.outcome,
            TransportOutcome::Rejected(TransportError::Malformed(_))
        ));
    }

    #[test]
    fn malformed_nonce_rejected_tpt010() {
        for bad in [
            "no-prefix-aabbccddeeff00112233445566778899",
            "n-AABBCCDDEEFF00112233445566778899",
            "n-short",
            "n-",
        ] {
            let mut e = base_envelope();
            e.nonce = bad.to_string();
            let check = validate_envelope(&e, &registries());
            assert!(
                matches!(
                    check.outcome,
                    TransportOutcome::Rejected(TransportError::Malformed(_))
                ),
                "expected Malformed for nonce: {bad}"
            );
        }
    }

    #[test]
    fn empty_receiver_array_rejected() {
        let mut e = base_envelope();
        e.receiver_id = crate::envelope::ReceiverId::Many(vec![]);
        let check = validate_envelope(&e, &registries());
        assert!(matches!(
            check.outcome,
            TransportOutcome::Rejected(TransportError::Malformed(_))
        ));
    }

    #[test]
    fn receiver_array_with_empty_string_rejected() {
        let mut e = base_envelope();
        e.receiver_id = crate::envelope::ReceiverId::Many(vec![
            "did:arrow:pilot:node-b".to_string(),
            "".to_string(),
        ]);
        let check = validate_envelope(&e, &registries());
        assert!(matches!(
            check.outcome,
            TransportOutcome::Rejected(TransportError::Malformed(_))
        ));
    }

    #[test]
    fn future_envelope_beyond_tolerance_rejected_tpt041() {
        let mut e = base_envelope();
        // 10 minutes in the future — beyond the 5-minute tolerance
        e.sent_at = (Utc::now() + Duration::minutes(10)).to_rfc3339();
        let check = validate_envelope(&e, &registries());
        assert!(matches!(
            check.outcome,
            TransportOutcome::Rejected(TransportError::Stale(_))
        ));
    }

    #[test]
    fn stale_envelope_warns_but_accepts_tpt040() {
        let mut e = base_envelope();
        // 25 hours in the past — beyond the 24-hour stale window
        e.sent_at = (Utc::now() - Duration::hours(25)).to_rfc3339();
        let check = validate_envelope(&e, &registries());
        assert!(matches!(check.outcome, TransportOutcome::Accepted));
        assert!(check.warnings.contains(&"TPT-STALE-ENVELOPE".to_string()));
    }
}

/// Validates nonce against the required format: ^n-[0-9a-f]{32}$
fn is_valid_nonce(nonce: &str) -> bool {
    if let Some(hex_part) = nonce.strip_prefix("n-") {
        hex_part.len() == 32
            && hex_part
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    } else {
        false
    }
}

#[cfg(test)]
mod nonce_tests {
    use super::*;

    #[test]
    fn valid_nonce_formats() {
        assert!(is_valid_nonce("n-aabbccddeeff00112233445566778899"));
        assert!(is_valid_nonce("n-00000000000000000000000000000000"));
        assert!(is_valid_nonce("n-deadbeefdeadbeefdeadbeefdeadbeef"));
    }

    #[test]
    fn invalid_nonce_formats() {
        assert!(!is_valid_nonce(""));
        assert!(!is_valid_nonce("aabbccddeeff00112233445566778899")); // missing prefix
        assert!(!is_valid_nonce("n-AABBCCDDEEFF00112233445566778899")); // uppercase
        assert!(!is_valid_nonce("n-aabb")); // too short
        assert!(!is_valid_nonce("n-aabbccddeeff001122334455667788990011")); // too long
        assert!(!is_valid_nonce("n-aabbccddeeff0011223344556677889g")); // invalid hex char
    }
}
