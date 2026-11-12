use rnl_runtime::{
    diag::Severity,
    problem::{input_digest, FactKind, LevelProblem, Position, TableCell},
    source::MemoryProvider,
    Options,
};
use rnl_solver_io::{decode_problem, encode_problem, parse_json};

fn read(version: &str, body: &str) -> LevelProblem {
    let source=format!("* input protocol\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a n 1k\nR2 n b 1k\n* @RNL {{ {} }}\n.end\n",body.replace('\n'," "));
    let source = if version == "0.4" { source.replace("@ROUTE;", "@ROUTE-NET;") } else { source };
    let files = MemoryProvider::new().with("input.sp", &source);
    let r = rnl_view::interpret("input.sp", None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
    r.problems[0].clone()
}
fn rich() -> LevelProblem {
    read(
        "0.4",
        r#"
        $DOT = @DEFINE(roles=[["p","point"]]) {};
        $S = @DEFINE-SYMBOL {
            (-1,-1) (1,1) @RECTANGLE;
            %p=(-2,0) @PIN(terminal=1,label="p_1"); (2,0) @PIN(terminal=2);
            ? @TEXT("\(\frac{\sqrt{x^2+1}}{y_1}\)",key="name",fix_rotate=true) => @PLACE(0,2)
        };
        {#R1 #R2} @SET-SYMBOL $S;
        #R1::name @PLACE(x=4);
        #R1 @ORIENT(["R0","MY"]);
        #R1::p @CURRENT("in"); #R1:2 @CURRENT("out");
        $D = @DEFINE(roles=["a"],parameters=[["dx",2]]) { ?a @PLACE(?dx,3); %q=(-1,4) };
        %u=#R1 @AS(dx=2) $D; %u @PLACE(20,10);
        @TABLE rows: {{#R1 #R2}};
        {#R1 #R2} @GROUP(spacing_x=3,spacing_y=1);
        &n @ROUTE;
        &a @WIRE(mode="manhattan45") {#R1::p::position (6,7)};
        @SOLVER-CONFIG(spacing_x=2,junction="$DOT")
    "#,
    )
}
fn roundtrip(p: &LevelProblem) -> LevelProblem {
    let bytes = serde_json::to_vec(&encode_problem(p).unwrap()).unwrap();
    let decoded = decode_problem(&parse_json(&bytes).unwrap()).unwrap();
    assert_eq!(&decoded, p);
    assert_eq!(input_digest(&decoded), input_digest(p));
    decoded
}

#[test]
fn complete_runtime_inputs_round_trip_and_solve_identically() {
    for version in ["0.2", "0.3", "0.4"] {
        let p = read(
            version,
            r#"$S = @DEFINE-SYMBOL {(-1,-1) (1,1) @RECTANGLE; (-2,0) @PIN(terminal=1); (2,0) @PIN(terminal=2)}; {#R1 #R2} @SET-SYMBOL $S; &n @ROUTE; &a @WIRE {#R1:1 (-6,0)}"#,
        );
        assert!(!p.routes.is_empty());
        assert!(p.routes.iter().all(|r| r.tree == (version == "0.4")));
        let decoded = roundtrip(&p);
        let options = rnl_solver::SolverOptions::without_time_limit();
        assert_eq!(rnl_solver::solve(&decoded, &options), rnl_solver::solve(&p, &options));
    }
}

#[test]
fn rich_text_nested_tables_partial_text_axes_and_junctions_are_lossless() {
    let p = rich();
    assert!(p.objects.iter().any(|o| matches!(o.position, Position::InSymbol { replaced: Some(_), .. })));
    assert!(p
        .objects
        .iter()
        .filter_map(|o| o.text.as_ref())
        .flat_map(|t| &t.lines)
        .flatten()
        .any(|r| matches!(r, rnl_runtime::markup::Run::Frac { .. })));
    assert!(p.facts.iter().any(|f|matches!(&f.kind,FactKind::Table {rows:TableCell::List(items),..} if items.iter().any(|c|matches!(c,TableCell::List(_))))));
    assert!(!p.currents.is_empty() && !p.orient_lists.is_empty() && !p.usages.is_empty());
    assert_eq!(p.junction.definition.as_deref(), Some("$DOT"));
    let decoded = roundtrip(&p);
    let options = rnl_solver::SolverOptions::without_time_limit();
    assert_eq!(rnl_solver::solve(&decoded, &options), rnl_solver::solve(&p, &options));
}

#[test]
fn invalid_indices_and_cyclic_dependencies_are_rejected_before_solving() {
    let p = rich();
    let original = encode_problem(&p).unwrap();
    for case in 0..5 {
        let mut value = original.clone();
        match case {
            0 => value["objects"][0]["symbol"] = serde_json::json!(p.symbols.len()),
            1 => {
                value["objects"][0]["position"] = serde_json::json!({"kind":"at","point":{"kind":"origin","object":0}})
            }
            2 => value["usages"][0]["parent"] = serde_json::json!(0),
            3 => value["routes"][0]["wires"] = serde_json::json!([p.wires.len()]),
            _ => value["orient_lists"][0]["fact"] = serde_json::json!(p.facts.len()),
        }
        assert!(decode_problem(&value).is_err(), "case {case}");
    }
}

#[test]
fn deep_markup_and_exact_floats_survive_the_byte_transport() {
    use rnl_runtime::markup::{Run, RunStyle};
    let mut p = rich();
    let mut run = Run::Text { text: "x".into(), style: RunStyle { upright: true, level: -3, ..RunStyle::default() } };
    for _ in 0..64 {
        run = Run::Sqrt { radicand: vec![run], style: RunStyle::default() };
    }
    let text = p.objects.iter_mut().find_map(|o| o.text.as_mut()).unwrap();
    text.lines = vec![vec![run]];
    text.rotate = 0.02881699800591428;
    roundtrip(&p);
    let too_deep = format!("{}0{}", "[".repeat(257), "]".repeat(257));
    assert!(parse_json(too_deep.as_bytes()).unwrap_err().message.contains("256"));
    assert!(parse_json(br#"{} {}"#).is_err());
    assert!(parse_json(br#"{"text":"[\"{]"}"#).is_ok());
}

#[test]
fn request_preserves_explicit_tools_safety_and_reporting_before_file_merging() {
    use rnl_solver_io::{decode_input, encode_input, InputDocument, LevelInput};
    let p = rich();
    let options = rnl_solver::SolverOptions {
        hints_for_low: true,
        safety_seconds: None,
        ..rnl_solver::SolverOptions::without_time_limit()
    }
    .with_config("spacing_x=6,limit=\"work\",work=20000,report_hints=true")
    .unwrap();
    assert_eq!(options.for_problem(&p).0.settings.spacing[0], 6);
    let input = InputDocument { levels: vec![LevelInput { problem: p.clone(), options: options.clone() }] };
    let bytes = serde_json::to_vec(&encode_input(&input).unwrap()).unwrap();
    let decoded = decode_input(&parse_json(&bytes).unwrap()).unwrap();
    assert_eq!(decoded, input);
    let next = &decoded.levels[0];
    assert_eq!(next.options.for_problem(&next.problem).0.settings.spacing[0], 6);
    assert_eq!(rnl_solver::solve(&next.problem, &next.options), rnl_solver::solve(&p, &options));
    let original = encode_input(&input).unwrap();
    for case in 0..4 {
        let mut bad = original.clone();
        match case {
            0 => bad["levels"][0]["input"] = serde_json::json!("wrong"),
            1 => bad["levels"][0]["request"] = serde_json::json!(1),
            2 => bad["version"] = serde_json::json!(2),
            _ => {
                bad["levels"][0]["options"]["overrides"] =
                    serde_json::json!([{"key":"work","value":{"kind":"number","value":-1},"source":null,"package":""}])
            }
        }
        assert!(decode_input(&bad).is_err(), "case {case}");
    }
    for first in [-1, 10] {
        let mut bad = original.clone();
        bad["levels"][0]["options"]["overrides"] = serde_json::json!([
            {"key":"work","value":{"kind":"number","value":first},"source":null,"package":""},
            {"key":"work","value":{"kind":"number","value":20},"source":null,"package":""},
            {"key":"limit","value":{"kind":"string","value":"work","unit":null},"source":null,"package":""}
        ]);
        assert!(decode_input(&bad).unwrap_err().message.contains("duplicate tool override"));
    }
}

#[test]
fn every_fact_shape_and_literal_metadata_is_preserved() {
    use rnl_runtime::{
        geom::Orient,
        interp::obj::{Lit, Priority, WStr},
        junction::JunctionConfig,
        problem::{Axis, DirectedPin, PointRef, ProblemFact, Sense, SolverSetting},
    };
    let mut p = rich();
    let text = p.objects.iter().position(|o| o.text.is_some()).unwrap();
    let a = PointRef::Origin(0);
    let b = PointRef::Origin(1);
    let pin = DirectedPin { object: 0, pin: 0, direction: 180 };
    let to = DirectedPin { object: 1, pin: 1, direction: 0 };
    let symbol = p.objects[0].symbol.unwrap();
    p.symbols[symbol].pins[0].dir = Some(180);
    p.symbols[symbol].pins[1].dir = Some(0);
    let shapes = vec![
        FactKind::Place { at: PointRef::Centre(vec![a.clone(), b.clone()]), axis: Axis::Y, value: 1.25 },
        FactKind::PointCoordinate { point: text, axis: Axis::X, value: 3.5 },
        FactKind::Offset { text, from: a.clone(), axis: Axis::X, value: -2.75 },
        FactKind::Orient { object: 0, code: Orient::MX },
        FactKind::PinOrient { pin, codes: vec![Orient::R0, Orient::MY] },
        FactKind::PinOrientRelation { from: pin, to, relations: vec![Orient::R180, Orient::R0] },
        FactKind::Align { axis: Axis::Y, members: vec![a.clone(), b.clone()] },
        FactKind::Order { axis: Axis::X, sense: Sense::Decreasing, members: vec![a.clone(), b.clone()] },
        FactKind::Distribute { axis: Axis::Y, sense: Sense::Either, members: vec![a.clone(), b.clone()] },
        FactKind::Distance {
            axis: Axis::X,
            sense: Sense::Increasing,
            min: Some(-3.),
            max: Some(4.25),
            from: vec![a.clone()],
            to: vec![b.clone()],
        },
        FactKind::Table {
            axis: Axis::Y,
            rows: TableCell::List(vec![TableCell::List(vec![TableCell::Point(a)]), TableCell::Point(b)]),
        },
        FactKind::OrientRelation { relation: Orient::MY, from: vec![0], to: vec![1] },
    ];
    for kind in shapes {
        p.facts.push(ProblemFact {
            kind,
            priority: Priority::High,
            label: "fixture fact".into(),
            reference: None,
            source: None,
            chain: vec![],
        });
    }
    let setting = SolverSetting {
        key: "unknown".into(),
        value: Lit::Arr(vec![
            Lit::Num(1.25),
            Lit::Bool(true),
            Lit::Str(WStr::new("quoted value", Some(12))),
            Lit::Arr(vec![Lit::Bool(false)]),
        ]),
        source: None,
        package: "fixture".into(),
    };
    p.solver_config.push(setting.clone());
    p.junction.tool_override =
        Some(Box::new(JunctionConfig { definition: None, invalid: Some(setting), tool_override: None }));
    roundtrip(&p);
}
