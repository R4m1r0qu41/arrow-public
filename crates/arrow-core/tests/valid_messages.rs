use arrow_core::{
    parser::message::parse, registry::loader::Registries, validation::pipeline::ValidationPipeline,
};

#[test]
fn valid_messages_from_spec_examples() {
    let registries = Registries::load_embedded().unwrap();
    let pipeline = ValidationPipeline::new(&registries);
    for raw in [
        // Pending observation — Action HL is non-executing per SEM-013 (v1.5.1 FN+0 ceiling).
        "1:BTC:PD:HL:0:A",
        "1:BTC:FN:PR:1:A",
        "2:CST:FN:PR:1:C",
        "3:LEG:CD:BL:0:C",
        "2:LEG:FN:PR:1:C",
        "5:PCY:FN:TR:1:B",
        "6:SOL:FN:CL:1:A",
        "2:THR:FN:PR:1:B",
    ] {
        let msg = parse(raw).unwrap();
        let result = pipeline.validate_message(&msg);
        assert!(result.passed, "{raw}: {:?}", result.violations);
    }
}
