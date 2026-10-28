//! The parameters of a usage in the editor interface's snapshot (`ObjectView::params`): those the
//! statement that applies it writes (9.4), each value of the type it is written in, and not the
//! defaults of the definition. A tool that writes what it alone knows as the parameters of a
//! definition that draws nothing, as Analog Canvas does with its extension package, reads them back
//! from the snapshot. A usage that writes none has no `params`, so its snapshot is what it was.

use rnl_runtime::source::MemoryProvider;
use rnl_view::{BuildOptions, Parts};
use serde_json::{json, Value};

const NETLIST: &str = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a 0 1k\nR2 a 0 1k\nR3 a 0 1k\n* @RNL {\n\
*   $NOTE = @DEFINE(roles=[[\"dev\", \"instance\"]], parameters=[[\"name\", \"\"], [\"raw\", \"\"], [\"n\", 1], [\"on\", false], [\"pts\", []]]) {};\n\
*   $NOTE @ATTACH(name=\"M1\", raw=\"100fF\", n=2.5, on=true, pts=[1, \"a\"]) #R1 => %n1;\n\
*   $NOTE @ATTACH #R2 => %n2;\n\
*   {#R1 #R2 #R3} @AS(direction=\"top-to-bottom\") $FLOW => %flow;\n\
*   {#R2 #R3} @AS $FLOW => %plain\n* }\n.end\n";

/// Usages that draw nothing attached to texts: a label by its key, and a named text without one.
const ON_TEXTS: &str = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a 0 1k\n* @RNL {\n\
*   $TAG = @DEFINE(roles=[\"label\"], parameters=[[\"binding\", \"\"]]) {};\n\
*   #R1 @PLACE(0, 0);\n\
*   (0, 6) @TEXT(\"note\") => %t3;\n\
*   $TAG @ATTACH(binding=\"reference\") #R1::name => %tag;\n\
*   $TAG @ATTACH %t3 => %tag2\n* }\n.end\n";

/// The main level's model.
fn model(netlist: &str) -> Value {
    let files = MemoryProvider::new().with("/w/t.sp", netlist);
    let parts = Parts { drawing: false, host: false, model: true };
    let s = rnl_view::snapshot("/w/t.sp", &files, &BuildOptions::default(), parts).unwrap();
    let mut s = serde_json::to_value(s).unwrap();
    s["levels"][0]["model"].take()
}

/// The objects of the main level's model.
fn objects() -> Vec<Value> {
    model(NETLIST)["objects"].as_array().unwrap().clone()
}

fn object<'a>(objects: &'a [Value], id: &str) -> &'a Value {
    objects.iter().find(|o| o["id"] == id).unwrap_or_else(|| panic!("no {id} in {objects:?}"))
}

#[test]
fn a_usage_gives_the_parameters_its_statement_writes_as_written() {
    let objects = objects();
    let n1 = object(&objects, "%n1");
    assert_eq!(n1["definition"], "$NOTE");
    // a number as the snapshot writes every number, 1 as 1.0
    assert_eq!(n1["params"], json!({ "name": "M1", "raw": "100fF", "n": 2.5, "on": true, "pts": [1.0, "a"] }));
    // a standard definition's parameter, written: the default is not repeated for the others
    assert_eq!(object(&objects, "%flow")["params"], json!({ "direction": "top-to-bottom" }));
}

#[test]
fn a_usage_that_writes_none_and_an_instance_have_no_parameters() {
    let objects = objects();
    for id in ["%n2", "%plain", "#R1"] {
        assert!(object(&objects, id).get("params").is_none(), "{id}: {}", object(&objects, id));
    }
    // a usage that draws nothing is still among the objects
    let n2 = object(&objects, "%n2");
    assert_eq!((&n2["kind"], &n2["symbol"]["value"]), (&json!("usage"), &Value::Null));
}

#[test]
fn a_usage_attached_to_a_text_names_it_as_the_texts_give_its_id() {
    let model = model(ON_TEXTS);
    let objects = model["objects"].as_array().unwrap();
    let texts = model["texts"].as_array().unwrap();
    let text = |id: &str| texts.iter().find(|t| t["id"] == id).unwrap_or_else(|| panic!("no {id} in {texts:?}"));
    for (usage, id) in [("%tag", "#R1::name"), ("%tag2", "%t3")] {
        let u = object(objects, usage);
        assert_eq!(u["attach"], json!({ "label": id }), "{u}");
        text(id);
    }
    assert_eq!(object(objects, "%tag")["params"], json!({ "binding": "reference" }));
}
