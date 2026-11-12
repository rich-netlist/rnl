use rnl_runtime::{source::MemoryProvider, Options};
use rnl_solver_io::{
    decode_input, decode_result, encode_input, encode_result, InputDocument, LevelInput, LevelResult, ResultDocument,
    SolverIdentity,
};

#[test]
fn auto_source_semantics_survive_transport_and_invalidate_old_geometry() {
    for version in ["0.3", "0.4"] {
        let text = format!("* auto source\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ &a @WIRE {{ (0,0) (2,3) }} }}\n.end\n");
        let files = MemoryProvider::new().with("auto.sp", &text);
        let read = rnl_view::interpret("auto.sp", None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        let p = read.problems[0].clone();
        assert_eq!(p.wires[0].default_auto, version == "0.4");
        let mut changed = p.clone();
        changed.wires[0].default_auto = !changed.wires[0].default_auto;
        assert_ne!(rnl_runtime::problem::input_digest(&p), rnl_runtime::problem::input_digest(&changed));
        let input = InputDocument { levels: vec![LevelInput { problem: p, options: Default::default() }] };
        let value = encode_input(&input).unwrap();
        assert_eq!(decode_input(&value).unwrap(), input);
        let mut invalid = value.clone();
        invalid["levels"][0]["problem"]["wires"][0]["mode"] = serde_json::json!("straight");
        invalid["levels"][0]["problem"]["wires"][0]["default_auto"] = serde_json::json!(true);
        assert!(decode_input(&invalid).unwrap_err().to_string().contains("default_auto"));
        let mut missing = value;
        missing["levels"][0]["problem"]["wires"][0].as_object_mut().unwrap().remove("default_auto");
        assert!(decode_input(&missing).unwrap_err().to_string().contains("default_auto"));
    }
}

#[test]
fn version_four_rejects_old_transport_and_validates_fatal_wire() {
    let text = "* protocol fixture\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a b 1k\n* @RNL { &a @WIRE(mode=\"straight\") { (0,0) (1,0) } }\n.end\n";
    let files = MemoryProvider::new().with("test.sp", text);
    let read = rnl_view::interpret("test.sp", None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    let p = read.problems[0].clone();
    assert!(!p.wires.is_empty());
    let input = InputDocument { levels: vec![LevelInput { problem: p.clone(), options: Default::default() }] };
    let value = encode_input(&input).unwrap();
    assert_eq!(value["version"], 4);
    for old in [2, 3] {
        let mut incompatible = value.clone();
        incompatible["version"] = serde_json::json!(old);
        assert!(decode_input(&incompatible).unwrap_err().to_string().contains("unsupported"));
    }
    for field in ["annotations", "existing_junctions"] {
        let mut missing = value.clone();
        missing["levels"][0]["problem"].as_object_mut().unwrap().remove(field);
        assert!(decode_input(&missing).unwrap_err().to_string().contains(field));
    }
    let report = rnl_solver::SolveReport {
        fatal_written_wire: Some(0),
        errors: vec!["unevaluable wire".into()],
        ..Default::default()
    };
    let document = ResultDocument {
        solver: SolverIdentity { name: "test".into(), version: "1".into() },
        levels: vec![LevelResult {
            request: 0,
            level: p.level,
            input: rnl_runtime::problem::input_digest(&p),
            options: String::new(),
            solution: Default::default(),
            report: Some(report),
        }],
    };
    let mut value = encode_result(&document).unwrap();
    assert_eq!(value["version"], 4);
    assert_eq!(decode_result(&value, std::slice::from_ref(&p)).unwrap(), document);
    for old in [2, 3] {
        let mut incompatible = value.clone();
        incompatible["version"] = serde_json::json!(old);
        assert!(decode_result(&incompatible, std::slice::from_ref(&p))
            .unwrap_err()
            .to_string()
            .contains("unsupported"));
    }
    value["levels"][0]["report"]["fatal_written_wire"] = serde_json::json!(p.wires.len());
    assert!(decode_result(&value, &[p]).is_err());
}
