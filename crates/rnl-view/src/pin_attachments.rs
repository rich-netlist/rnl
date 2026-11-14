//! Structural pin attachment in the editor interfaces, independent of drawing geometry.

use crate::{BuildOptions, MemoryProvider, Parts, Snapshot};

const SYMBOLS: &str = r#"
    $S = @DEFINE-SYMBOL {
        %p = (0,0) @PIN(terminal=1, direction="right") => %alias;
        %q = (0,1) @PIN(terminal=2);
        %free = (1,0) @PIN
    };
    #R1 @SET-SYMBOL $S; #R2 @SET-SYMBOL $S;
    $ONE = @DEFINE(roles=["t"]) {};
    $TWO = @DEFINE(roles=["t", "other"]) {};
    $SET = @DEFINE(roles=["{members}"]) {}
"#;

fn read(version: &str, body: &str) -> Snapshot {
    let snapshot = read_unchecked(version, body);
    assert!(!snapshot.diagnostics.iter().any(|d| d.severity == "error"), "{:?}", snapshot.diagnostics);
    snapshot
}

fn read_unchecked(version: &str, body: &str) -> Snapshot {
    let source = format!(
        "* attachments\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 b c 2k\n* @RNL {{ {} }}\n.end\n",
        body.replace('\n', "\n* ")
    );
    let options =
        BuildOptions { runtime: rnl_runtime::Options { std: false, ..Default::default() }, ..Default::default() };
    crate::snapshot(
        "attachments.sp",
        &MemoryProvider::new().with("attachments.sp", &source),
        &options,
        Parts { drawing: true, host: false, model: true },
    )
    .unwrap()
}

fn owner(snapshot: &Snapshot, usage: &str) -> Option<String> {
    let doc = snapshot.drawing.as_ref().unwrap();
    let row = doc.levels[0].usages.iter().find(|u| u.id == usage).unwrap();
    let got = row.pin_of.map(|i| &doc.objects[i]);
    let model = snapshot.levels[0].model.as_ref().unwrap();
    let view = model.objects.iter().find(|u| u.id == usage).unwrap();
    assert_eq!(view.pin_of, got.map(|o| if o.kind == "instance" { format!("#{}", o.name) } else { o.name.clone() }));
    got.map(|o| o.name.clone())
}

#[test]
fn pin_attachments_do_not_need_a_symbol_or_any_position() {
    let s = read("0.4", &format!("{SYMBOLS}; %u = #R1::p @AS $ONE"));
    assert_eq!(owner(&s, "%u").as_deref(), Some("R1"));
    let u = &s.drawing.as_ref().unwrap().levels[0].usages[0];
    assert!(!u.has_symbol);
    assert!(s.levels[0].model.as_ref().unwrap().objects.iter().all(|o| o.at.value.is_none()));
}

#[test]
fn pin_attachments_accept_terminals_aliases_unbound_pins_and_same_owner_roles() {
    for filler in ["#R1:1", "#R1::alias", "#R1::free"] {
        let s = read("0.4", &format!("{SYMBOLS}; %u = {filler}(t) #R1(other) @AS $TWO"));
        assert_eq!(owner(&s, "%u").as_deref(), Some("R1"), "{filler}");
    }
}

#[test]
fn pin_attachments_require_one_owner_and_ignore_net_roles() {
    for (other, want) in [("#R1::q", Some("R1")), ("&a", Some("R1")), ("#R2::p", None), ("#R2", None)] {
        let s = read("0.4", &format!("{SYMBOLS}; %u = #R1::p(t) {other}(other) @AS $TWO"));
        assert_eq!(owner(&s, "%u").as_deref(), want, "{other}");
    }
}

#[test]
fn pin_attachments_inspect_all_set_role_members() {
    for (members, want) in
        [("#R1::p #R1::q #R1::alias &a", Some("R1")), ("#R1::p #R2::p", None), ("#R1::p (0,0)", None), ("&a &b", None)]
    {
        let s = read("0.4", &format!("{SYMBOLS}; %u = {{{members}}} @AS $SET"));
        assert_eq!(owner(&s, "%u").as_deref(), want, "{members}");
    }
}

#[test]
fn pin_attachments_reject_other_points_even_at_the_same_coordinates() {
    for other in ["(0,0)", "#R1::p::position", "%point", "%text", "%v"] {
        let s = read(
            "0.4",
            &format!(
                r#"{SYMBOLS};
            #R1 @PLACE(0,0); %point = (0,0); %text = (0,0) @TEXT("label");
            %v = &a @AS $ONE => @PLACE(0,0);
            %u = #R1::p(t) {other}(other) @AS $TWO
        "#
            ),
        );
        assert_eq!(owner(&s, "%u"), None, "{other}");
    }
}

#[test]
fn pin_attachments_follow_the_converted_role_kind() {
    for (kind, want) in [("pin", Some("R1")), ("terminal", Some("R1")), ("point", None), ("net", None)] {
        let s = read(
            "0.4",
            &format!(
                r#"{SYMBOLS};
            $TYPED = @DEFINE(roles=[["t","{kind}"]]) {{}};
            %u = #R1:1 @AS $TYPED
        "#
            ),
        );
        assert_eq!(owner(&s, "%u").as_deref(), want, "{kind}");
    }
}

#[test]
fn pin_attachments_reach_the_pins_of_the_default_block_symbol() {
    // #R1 has no symbol of its own: the default block symbol gives each terminal a pin (9.2.6)
    let s = read("0.4", "$D = @DEFINE(roles=[\"t\"]) {}; %u = #R1:1 @AS $D");
    assert_eq!(owner(&s, "%u").as_deref(), Some("R1"));
}

#[test]
fn pin_attachments_expose_parent_chains_without_inheriting_the_field() {
    let s = read(
        "0.4",
        &format!(
            r#"{SYMBOLS};
        $INNER = @DEFINE(roles=["n"]) {{ %child = ?n @AS $ONE }};
        $OUTER = @DEFINE(roles=[["t","pin"]]) {{ %inner = ?t::net @AS $INNER }};
        %u = #R1:1 @AS $OUTER
    "#
        ),
    );
    let doc = s.drawing.as_ref().unwrap();
    let rows = &doc.levels[0].usages;
    let root = rows.iter().find(|u| u.id == "%u").unwrap();
    let inner = rows.iter().find(|u| u.parent == Some(root.object)).unwrap();
    let child = rows.iter().find(|u| u.parent == Some(inner.object)).unwrap();
    assert_eq!(owner(&s, "%u").as_deref(), Some("R1"));
    assert_eq!(owner(&s, &inner.id), None);
    assert_eq!(owner(&s, &child.id), None);
    let model = s.levels[0].model.as_ref().unwrap();
    assert_eq!(model.objects.iter().find(|o| o.id == child.id).unwrap().parent.as_deref(), Some(inner.id.as_str()));
}

#[test]
fn pin_attachments_do_not_add_fields_to_old_versions() {
    for version in ["0.2", "0.3"] {
        let s = read(
            version,
            r#"
            $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S;
            $INNER = @DEFINE(roles=["t"]) {};
            $OUTER = @DEFINE(roles=["t"]) { %child = ?t @AS $INNER };
            %u = #R1:1 @AS $OUTER
        "#,
        );
        let value = serde_json::to_value(s).unwrap();
        for o in value["levels"][0]["model"]["objects"].as_array().unwrap() {
            assert!(o.get("pin_of").is_none() && o.get("parent").is_none());
        }
        for u in value["drawing"]["levels"][0]["usages"].as_array().unwrap() {
            assert!(u.get("pin_of").is_none() && u.get("parent").is_none());
        }
    }
}

#[test]
fn pin_attachments_recognize_all_four_standard_stubs() {
    let imported = read_unchecked("0.4", &format!(r#"@USE-PACKAGE(name="rnl-std-basic"); {SYMBOLS}"#));
    let errors = |s: &Snapshot| {
        s.diagnostics
            .iter()
            .filter(|d| d.severity == "error")
            .map(|d| (d.code.clone(), d.message.clone()))
            .collect::<std::collections::BTreeSet<_>>()
    };
    for name in ["GROUND_STUB", "SUPPLY_STUB", "IO_PORT_STUB", "NET_LABEL_STUB"] {
        let s = read_unchecked(
            "0.4",
            &format!(
                r#"
            @USE-PACKAGE(name="rnl-std-basic"); {SYMBOLS};
            %u = #R1:1 @AS ${name}
        "#
            ),
        );
        // Importing the draft package may report features still awaiting implementation.
        // Expanding a stub may repeat those errors, but must introduce no different ones.
        assert_eq!(errors(&s), errors(&imported), "{name}");
        assert_eq!(owner(&s, "%u").as_deref(), Some("R1"), "{name}");
        let rows = &s.drawing.as_ref().unwrap().levels[0].usages;
        let root = rows.iter().find(|u| u.id == "%u").unwrap();
        assert!(!root.has_symbol);
        assert!(rows.iter().any(|u| u.parent == Some(root.object) && u.has_symbol), "{name}");
    }
}

#[test]
fn pin_attachments_do_not_depend_on_placement_or_priority() {
    for modifier in ["", "=> @PLACE(100,200)", "=> @PRIORITY(\"disabled\")"] {
        let s = read("0.4", &format!("{SYMBOLS}; #R1 @PLACE(0,0); %u = #R1:1 @AS $ONE {modifier}"));
        assert_eq!(owner(&s, "%u").as_deref(), Some("R1"));
    }
}

#[test]
fn pin_attachments_can_belong_to_a_usage() {
    let s = read(
        "0.4",
        &format!(
            r#"{SYMBOLS};
        $V = @DEFINE-SYMBOL {{ %p = (0,0) @PIN(terminal="n") }};
        $D = @DEFINE(roles=["n"]) {{}};
        %v = &a @AS $D => @SET-SYMBOL $V;
        %u = %v::p @AS $ONE
    "#
        ),
    );
    assert_eq!(owner(&s, "%u").as_deref(), Some("%v"));
}

#[test]
fn pin_attachments_keep_a_bundle_distinct_from_its_host_instances() {
    let symbols = SYMBOLS.replace("#R1 @SET-SYMBOL $S; #R2 @SET-SYMBOL $S;", "");
    for members in ["#R1", "#R1 #R2"] {
        let s = read(
            "0.4",
            &format!(
                r#"{symbols};
            %b = {{{members}}} @BUNDLE => @SET-SYMBOL $S;
            %u = %b::p @AS $ONE
        "#
            ),
        );
        assert_eq!(owner(&s, "%u").as_deref(), Some("%b"));
    }
}

#[test]
fn pin_attachments_expand_converted_set_roles() {
    let s = read(
        "0.4",
        &format!(
            r#"{SYMBOLS};
        $D = @DEFINE(roles=[["{{pins}}","pin"]]) {{}};
        %u = {{#R1:1 #R1:2}} @AS $D
    "#
        ),
    );
    assert_eq!(owner(&s, "%u").as_deref(), Some("R1"));
}

#[test]
fn pin_attachments_follow_the_creating_file_in_mixed_includes() {
    for (outer, inner) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let files = MemoryProvider::new()
            .with("top.sp", &format!(r#"* mixed
* @RNL(type="header") {{ @VERSION("{outer}") }}
.include new.sp
* @RNL(scope="child") {{
* $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1) }};
* #R1 @SET-SYMBOL $S;
* $D = @DEFINE(roles=["t"]) {{}}; %u = #R1:1 @AS $D
* }}
.end
"#))
            .with("new.sp", &format!("* child\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a b\nR1 a b 1k\n.ends\n"));
        let options =
            BuildOptions { runtime: rnl_runtime::Options { std: false, ..Default::default() }, ..Default::default() };
        let s = crate::snapshot("top.sp", &files, &options, Parts { drawing: true, host: false, model: true }).unwrap();
        assert!(!s.diagnostics.iter().any(|d| d.severity == "error"), "{:?}", s.diagnostics);
        let doc = s.drawing.as_ref().unwrap();
        let usage = doc.levels.iter().flat_map(|l| &l.usages).find(|u| u.id == "%u").unwrap();
        assert_eq!(usage.pin_of.map(|i| doc.objects[i].name.as_str()), (outer == "0.4").then_some("R1"));
        let view =
            s.levels.iter().filter_map(|l| l.model.as_ref()).flat_map(|m| &m.objects).find(|o| o.id == "%u").unwrap();
        assert_eq!(view.pin_of.as_deref(), (outer == "0.4").then_some("#R1"));
    }
}
