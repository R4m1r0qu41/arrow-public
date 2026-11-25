use crate::{
    errors::ParseError,
    types::message::{Action, ArrowMessage, Class, Confidence, State},
};

/// Parses the canonical six-field Arrow message string, e.g. "2:CST:FN:PR:1:C".
/// Returns a ParseError on any structural or format violation.
pub fn parse(raw: &str) -> Result<ArrowMessage, ParseError> {
    // MSG-002 — no leading/trailing whitespace
    if raw != raw.trim() {
        return Err(ParseError::new(
            "MSG-002",
            "message string must not have leading or trailing whitespace",
        ));
    }

    let parts: Vec<&str> = raw.split(':').collect();

    // MSG-001 — exactly six fields
    if parts.len() != 6 {
        return Err(ParseError::new(
            "MSG-001",
            format!("expected 6 colon-separated fields, got {}", parts.len()),
        ));
    }

    let [class_str, origin, state_str, action_str, finality_str, confidence_str] = parts.as_slice()
    else {
        unreachable!()
    };

    // Class — numeric
    let class_num: u8 = class_str.parse().map_err(|_| {
        ParseError::new(
            "MSG-003",
            format!("class field `{class_str}` is not a valid integer"),
        )
    })?;

    // Origin — exactly 3 uppercase ASCII letters (ORG-001)
    if origin.len() != 3 || !origin.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ParseError::new(
            "ORG-001",
            format!("origin `{origin}` must be exactly 3 uppercase ASCII letters"),
        ));
    }

    // State — uppercase letters (STA-001)
    if !state_str.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ParseError::new(
            "STA-001",
            format!("state `{state_str}` must be uppercase letters"),
        ));
    }

    // Action — uppercase letters
    if !action_str.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ParseError::new(
            "ACT-001",
            format!("action `{action_str}` must be uppercase letters"),
        ));
    }

    // Finality — 0 or 1
    let finality: u8 = finality_str.parse().map_err(|_| {
        ParseError::new(
            "FIN-001",
            format!("finality `{finality_str}` must be 0 or 1"),
        )
    })?;
    if finality > 1 {
        return Err(ParseError::new(
            "FIN-001",
            format!("finality `{finality_str}` must be 0 or 1"),
        ));
    }

    // Confidence — single uppercase letter
    if confidence_str.len() != 1 || !confidence_str.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ParseError::new(
            "CFD-001",
            format!("confidence `{confidence_str}` must be a single uppercase letter"),
        ));
    }

    Ok(ArrowMessage {
        class: Class(class_num),
        origin: origin.to_string(),
        state: State(state_str.to_string()),
        action: Action(action_str.to_string()),
        finality,
        confidence: Confidence(confidence_str.to_string()),
        raw: raw.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_codes() {
        for raw in [
            "1:BTC:FN:PR:1:A",
            "2:CST:FN:PR:1:C",
            "3:LEG:CD:BL:0:C",
            "5:PCY:FN:TR:1:B",
        ] {
            assert!(parse(raw).is_ok(), "should parse: {raw}");
        }
    }

    #[test]
    fn rejects_wrong_field_count() {
        assert_eq!(parse("1:BTC:FN:PR:1").unwrap_err().rule_id(), "MSG-001");
        assert_eq!(parse("1:BTC:FN:PR:1:A:X").unwrap_err().rule_id(), "MSG-001");
    }

    #[test]
    fn rejects_trailing_whitespace() {
        assert_eq!(parse("1:BTC:FN:PR:1:A ").unwrap_err().rule_id(), "MSG-002");
    }

    #[test]
    fn rejects_lowercase_state() {
        assert_eq!(parse("1:BTC:fn:PR:1:A").unwrap_err().rule_id(), "STA-001");
    }

    #[test]
    fn rejects_invalid_origin() {
        // Too short (1 char)
        assert_eq!(parse("2:X:FN:PR:1:C").unwrap_err().rule_id(), "ORG-001");
        // Too short (2 chars)
        assert_eq!(parse("2:ET:FN:PR:1:C").unwrap_err().rule_id(), "ORG-001");
        // Too long (4 chars)
        assert_eq!(parse("2:CUST:FN:PR:1:C").unwrap_err().rule_id(), "ORG-001");
        // Contains digit
        assert_eq!(parse("2:ET1:FN:PR:1:C").unwrap_err().rule_id(), "ORG-001");
    }
}
