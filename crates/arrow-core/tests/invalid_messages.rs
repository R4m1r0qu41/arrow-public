use arrow_core::{
    parser::message::parse, registry::loader::Registries, validation::pipeline::ValidationPipeline,
};

#[test]
fn invalid_parser_examples() {
    assert_eq!(parse("1:BTC:FN:PR:1").unwrap_err().rule_id(), "MSG-001");
    assert_eq!(parse("2:CST:FN:PR:1:C:X").unwrap_err().rule_id(), "MSG-001");
    assert_eq!(parse("1:BTC:fn:PR:1:A").unwrap_err().rule_id(), "STA-001");
    assert_eq!(parse("1:BTC:FN:PR:1:A ").unwrap_err().rule_id(), "MSG-002");
    assert_eq!(parse("2:CUST:FN:PR:1:C").unwrap_err().rule_id(), "ORG-001");
}

#[test]
fn invalid_semantic_examples() {
    let registries = Registries::load_embedded().unwrap();
    let pipeline = ValidationPipeline::new(&registries);
    for (raw, expected) in [
        ("6:SOL:FN:PR:0:A", "SEM-005"),
        ("3:LEG:FN:PR:1:C", "SEM-006"),
        ("5:ETH:PD:TR:0:E", "SEM-009"),
        ("6:SOL:PD:CL:1:A", "SEM-012"),
    ] {
        let msg = parse(raw).unwrap();
        let result = pipeline.validate_message(&msg);
        assert!(!result.passed, "{raw} unexpectedly passed");
        assert!(
            result.violations.iter().any(|v| v.rule_id == expected),
            "{raw} missing {expected}: {:?}",
            result.violations
        );
    }
}

#[test]
fn old_pol_origin_code_is_not_registered() {
    let registries = Registries::load_embedded().unwrap();
    let pipeline = ValidationPipeline::new(&registries);
    let msg = parse("5:POL:FN:TR:1:B").unwrap();
    let result = pipeline.validate_message(&msg);

    assert!(
        !result.passed,
        "old POL governance origin unexpectedly passed"
    );
    assert!(
        result.violations.iter().any(|v| v.rule_id == "ORG-003"),
        "old POL origin should fail registry lookup: {:?}",
        result.violations
    );
}

#[test]
fn legal_attestation_confidence_profiles_are_separated() {
    let registries = Registries::load_embedded().unwrap();

    assert!(
        !registries
            .verification_regimes
            .contains_key("legal_oracle_structured"),
        "legal_oracle_structured should remain superseded by structured_legal_attestation"
    );
    assert!(
        registries
            .verification_regimes
            .contains_key("structured_legal_attestation"),
        "single-Sibyl/LBRY-style proof should have its own structured legal attestation regime"
    );
    assert!(
        registries
            .verification_regimes
            .contains_key("multiple_legal_attestation"),
        "independent multi-Sibyl proof should have its own legal attestation regime"
    );
    assert!(
        !registries
            .confidence_subprofiles
            .contains_key("legal_attestation_panel"),
        "legal authority panels should not be confidence subprofiles"
    );
    assert!(
        registries
            .confidence_subprofiles
            .contains_key("structured_legal_attestation"),
        "structured legal attestation should be the single legal-oracle confidence path"
    );
    assert!(
        registries
            .confidence_subprofiles
            .contains_key("multiple_legal_attestation"),
        "multiple legal attestation should be the independent multi-Sibyl confidence path"
    );
}
