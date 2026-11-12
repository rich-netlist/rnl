use rnl_runtime::{
    problem::{input_digest, PointRef},
    source::MemoryProvider,
    Options,
};
use rnl_solver_io::{decode_problem, encode_problem};

#[test]
fn protocol_three_and_digest_bind_existing_junction_identity() {
    let text = "* protocol dot\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(junction=\"$DOT\") }\nR1 a b 1k\n* @RNL { $DOT = @DEFINE(roles=[[\"p\",\"point\"]]) {}; %p = (0,0); $DOT @ATTACH %p }\n.end\n";
    let files = MemoryProvider::new().with("test.sp", text);
    let r = rnl_view::interpret("test.sp", None, &files, &Options { std: false, problems: true, ..Default::default() })
        .unwrap();
    let p = &r.problems[0];
    assert_eq!(p.existing_junctions.len(), 1);
    let mut value = encode_problem(p).unwrap();
    assert_eq!(decode_problem(&value).unwrap(), *p);
    let mut undotted = p.clone();
    undotted.existing_junctions.clear();
    assert_ne!(input_digest(p), input_digest(&undotted));
    value["existing_junctions"][0]["object"] = serde_json::json!(usize::MAX);
    assert!(decode_problem(&value).is_err());
    let mut invalid = p.clone();
    invalid.existing_junctions.push(PointRef::Origin(usize::MAX));
    assert!(encode_problem(&invalid).is_err());
}
