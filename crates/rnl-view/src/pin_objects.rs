//! Independent pins in the editor interfaces, before and after placement.
use crate::{BuildOptions, MemoryProvider, Parts, Snapshot};

const SYMBOL: &str = r#"
    $S = @DEFINE-SYMBOL {
        %p = (2,3) @PIN(terminal=1, direction="right") => %alias;
        %q = (1,0) @PIN(terminal=1);
        %bus = (0,1) @PIN(terminal=[2]);
        (3,0) @PIN;
        (4,0) @PIN(terminal=2)
    }; #R1 @SET-SYMBOL $S
"#;

fn read(version: &str, body: &str) -> Snapshot {
    let source = format!(
        "* pins\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {} }}\n.end\n",
        body.replace('\n', "\n* ")
    );
    let options =
        BuildOptions { runtime: rnl_runtime::Options { std: false, ..Default::default() }, ..Default::default() };
    let s = crate::snapshot(
        "pins.sp",
        &MemoryProvider::new().with("pins.sp", &source),
        &options,
        Parts { drawing: true, host: false, model: true },
    )
    .unwrap();
    assert!(!s.diagnostics.iter().any(|d| d.severity == "error"), "{:?}", s.diagnostics);
    s
}

#[test]
fn independent_pins_exist_before_placement_and_have_one_physical_index() {
    let s = read("0.4", SYMBOL);
    let model = s.levels[0].model.as_ref().unwrap();
    let pins = &model.objects.iter().find(|o| o.id == "#R1").unwrap().pins;
    let doc = s.drawing.as_ref().unwrap();
    let objects: Vec<_> = doc.objects.iter().filter(|o| o.kind == "pin").collect();
    assert_eq!(pins.len(), 5);
    assert_eq!(objects.len(), 5);
    for (index, (pin, object)) in pins.iter().zip(objects).enumerate() {
        assert_eq!(pin.index, Some(index));
        assert_eq!(pin.kind, Some("pin"));
        assert_eq!(pin.id.as_deref(), Some(object.name.as_str()));
        assert!(pin.at.is_none());
        let data = object.pin.as_ref().unwrap();
        assert_eq!(data.index, index);
        assert_eq!(doc.objects[data.owner].name, "R1");
        assert_eq!(data.at, None);
    }
    assert_eq!(pins[0].names.as_ref().unwrap(), &["p", "alias"]);
    assert_eq!(pins[0].reference.as_deref(), Some("#R1::p"));
    assert_eq!(pins[1].reference.as_deref(), Some("#R1::q"));
    assert_eq!(pins[2].bus, Some(true));
    assert_eq!(pins[3].reference, None);
    assert_eq!(pins[3].nets, Vec::<String>::new());
    assert_eq!(pins[4].reference.as_deref(), Some("#R1:2"));
}

#[test]
fn an_unnamed_earlier_binding_never_borrows_the_later_pins_terminal_reference() {
    let s = read("0.4", "$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (1,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S");
    let pins = &s.levels[0].model.as_ref().unwrap().objects[0].pins;
    assert_eq!(pins.len(), 2);
    assert_ne!(pins[0].id, pins[1].id);
    assert_eq!(pins[0].reference, None);
    assert_eq!(pins[1].reference.as_deref(), Some("#R1:1"));
    assert_eq!(pins[0].nets, pins[1].nets);
}

#[test]
fn pin_positions_transform_with_the_owner_and_keep_symbol_local_coordinates() {
    for (orientation, offset) in [
        ("R0", [2., 3.]),
        ("R90", [-3., 2.]),
        ("R180", [-2., -3.]),
        ("R270", [3., -2.]),
        ("MX", [2., -3.]),
        ("MY", [-2., 3.]),
        ("MXR90", [3., 2.]),
        ("MYR90", [-3., -2.]),
    ] {
        let s = read("0.4", &format!("{SYMBOL}; #R1 @PLACE(5,7) => @ORIENT(\"{orientation}\")"));
        let pin = &s.levels[0].model.as_ref().unwrap().objects[0].pins[0];
        assert_eq!(pin.local_at, Some([2., 3.]));
        assert_eq!(pin.at, Some([5. + offset[0], 7. + offset[1]]));
    }
}

#[test]
fn unbound_pins_have_independent_document_hits() {
    let s = read("0.4", &format!("{SYMBOL}; #R1 @PLACE(0,0)"));
    let doc = s.drawing.as_ref().unwrap();
    let hits: Vec<_> = doc.levels[0].hits.iter().filter(|h| h.kind == "pin").collect();
    assert_eq!(hits.len(), 5);
    for (index, hit) in hits.iter().enumerate() {
        let object = &doc.objects[hit.pin.unwrap()];
        assert_eq!(object.kind, "pin");
        assert_eq!(object.pin.as_ref().unwrap().index, index);
    }
    assert!(hits[3].objects.is_empty());
}

#[test]
fn generated_usage_pin_references_include_the_parent_path() {
    let s = read(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal="n") };
        $INNER = @DEFINE(roles=["n"]) {};
        $OUTER = @DEFINE(roles=["n"]) { %inner = ?n @AS $INNER => @SET-SYMBOL $S };
        %u = &a @AS $OUTER
    "#,
    );
    let model = s.levels[0].model.as_ref().unwrap();
    let inner = model.objects.iter().find(|o| o.definition.as_deref() == Some("$INNER")).unwrap();
    assert_eq!(inner.pins[0].reference.as_deref(), Some("%u::inner::p"));
    assert_eq!(inner.pins[0].nets, ["&a"]);
}

#[test]
fn the_pins_of_the_default_block_symbol_are_pin_objects() {
    // the default block symbol is a symbol like any other (9.2.6)
    let s = read("0.4", "#R1 @PLACE(0,0)");
    assert!(s.drawing.as_ref().unwrap().objects.iter().any(|o| o.kind == "pin"));
}

#[test]
fn earlier_versions_keep_only_their_placed_pin_view() {
    for version in ["0.2", "0.3"] {
        let s = read(version, "$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S => @PLACE(0,0)");
        let value = serde_json::to_value(s).unwrap();
        let pin = &value["levels"][0]["model"]["objects"][0]["pins"][0];
        assert_eq!(pin["at"], serde_json::json!([0.0, 0.0]));
        for field in ["kind", "id", "index", "names", "bus", "reference", "local_at"] {
            assert!(pin.get(field).is_none(), "{version}: {field}");
        }
        assert!(value["drawing"]["objects"].as_array().unwrap().iter().all(|o| o.get("pin").is_none()));
    }
}

#[test]
fn pin_alias_roles_keep_written_members_and_share_an_editable_object() {
    let s = read(
        "0.4",
        &format!(
            r#"{SYMBOL}; #R1 @PLACE(0,0);
        $PAIR = @DEFINE(roles=["first","second"]) {{}};
        %u = #R1::p(first) #R1::alias(second) @AS $PAIR
    "#
        ),
    );
    let doc = s.drawing.as_ref().unwrap();
    let u = doc.levels[0].usages.iter().find(|u| u.id == "%u").unwrap();
    assert_eq!(u.roles[0].members, ["#R1::p"]);
    assert_eq!(u.roles[1].members, ["#R1::alias"]);
    let (pin, object) =
        doc.objects.iter().enumerate().find(|(_, o)| o.pin.as_ref().is_some_and(|p| p.index == 0)).unwrap();
    assert!(u.roles.iter().all(|r| r.objects.contains(&pin)));
    assert_eq!(u.roles[0].bounds, u.roles[1].bounds);
    let membership = object.usages.iter().find(|m| m.id == "%u").unwrap();
    assert_eq!(membership.roles.len(), 2);
    assert_eq!(membership.roles[0].member, "#R1::p");
    assert_eq!(membership.roles[1].member, "#R1::alias");
}

#[test]
fn selected_pins_use_physical_ids_without_inventing_source_references() {
    let s = read(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (1,0) @PIN(terminal=1) };
        #R1 @SET-SYMBOL $S => @PLACE(0,0);
        $D = @DEFINE(roles=["p"]) {};
        %selected = #R1 @SELECT-PIN;
        %selected @AS $D
    "#,
    );
    let doc = s.drawing.as_ref().unwrap();
    let pins: Vec<_> = doc.objects.iter().filter(|o| o.kind == "pin").collect();
    assert_eq!(pins.len(), 2);
    assert!(pins.iter().all(|p| p.usages.iter().any(|u| u.definition == "$D")));
    assert!(pins.iter().all(|p| p.name.starts_with("pin(#R1,")));
    assert_eq!(pins[0].pin.as_ref().unwrap().reference, None);
    assert_eq!(pins[1].pin.as_ref().unwrap().reference.as_deref(), Some("#R1:1"));
}

#[test]
fn pin_interfaces_follow_symbol_and_assignment_versions_in_mixed_includes() {
    for (outer, inner) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let files = MemoryProvider::new()
            .with("top.sp", &format!(r#"* mixed
* @RNL(type="header") {{ @VERSION("{outer}") }}
.include child.sp
* @RNL(scope="child") {{
* $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1) }};
* #R1 @SET-SYMBOL $S
* }}
.end
"#))
            .with("child.sp", &format!("* child\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a b\nR1 a b 1k\n.ends\n"));
        let options =
            BuildOptions { runtime: rnl_runtime::Options { std: false, ..Default::default() }, ..Default::default() };
        let s = crate::snapshot("top.sp", &files, &options, Parts { drawing: true, host: false, model: true }).unwrap();
        assert!(!s.diagnostics.iter().any(|d| d.severity == "error"), "{:?}", s.diagnostics);
        let doc = s.drawing.as_ref().unwrap();
        let pin = doc.objects.iter().find_map(|o| o.pin.as_ref()).unwrap();
        assert_eq!(pin.at, None);
        assert_eq!(doc.objects[pin.owner].name, "R1");
        assert_eq!(pin.reference.as_deref(), Some("#R1:1"));
    }
}

#[test]
fn generated_wire_vertices_do_not_turn_opaque_pin_ids_into_source() {
    let source = r#"* pins
* @RNL(type="header") { @VERSION("0.4") }
R1 a b 1k
* @RNL {
* $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (1,0) @PIN(terminal=1) };
* #R1 @SET-SYMBOL $S => @PLACE(0,0);
* $D = @DEFINE(roles=[["p","pin"]]) { ?p::net @WIRE {?p (8,8)} };
* #R1 @SELECT-PIN => @AS $D => @PLACE(0,0)
* }
.end
"#;
    let files = MemoryProvider::new().with("pins.sp", source);
    let r =
        crate::interpret("pins.sp", None, &files, &rnl_runtime::Options { std: false, ..Default::default() }).unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error));
    let wires = &r.model.levels[0].wires;
    assert_eq!(wires.len(), 2);
    assert!(matches!(&wires[0].via[0], rnl_runtime::model::Vertex::Pin { reference: None, pin: 0, .. }));
    assert!(
        matches!(&wires[1].via[0], rnl_runtime::model::Vertex::Pin { reference: Some(r), pin:1, .. } if r == "#R1:1")
    );
}
