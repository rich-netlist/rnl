//! Declaration order is shared by graphics, fixed text and drawing-only usages (9.2.1, 9.2.7).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_render::{primitives, svg, Kind, RenderOptions};
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(body: &str) -> Interpretation {
    let text = format!(
        "* symbol drawing order\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\nR1 a b 1k\n* @RNL {{\n{}\n* }}\n.end\n",
        body.lines().map(|line| format!("* {line}")).collect::<Vec<_>>().join("\n")
    );
    let files = MemoryProvider::new().with("order.sp", &text);
    let r = interpret_file(Path::new("order.sp"), None, &files, &Options { std: false, ..Options::default() }).unwrap();
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{:?}", errors.iter().map(|d| r.sink.render(d)).collect::<Vec<_>>());
    r
}

fn colors(r: &Interpretation) -> Vec<String> {
    primitives(&r.model.levels[0], &RenderOptions::default())
        .iter()
        .filter(|p| p.owner == "#R1")
        .filter_map(|p| match &p.kind {
            Kind::Rect { fill, .. } => fill.clone(),
            Kind::Text(t) => Some(t.color.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn nested_graphics_and_fixed_text_keep_their_position_through_repeated_inclusion() {
    let r = read(
        r##"$MARK = @DEFINE(roles=["at"]) {};
$INNER = @DEFINE-SYMBOL {
    @RECTANGLE(fill="fillcolor", fillcolor="#ff0000") {(-1,-1) (1,1)};
    (0,0) @TEXT("inner", textcolor="#00ff00")
};
$FRAGMENT = @DEFINE-SYMBOL {
    $MARK @ATTACH (0,0) => @SET-SYMBOL $INNER;
    @RECTANGLE(fill="fillcolor", fillcolor="#0000ff") {(-1,-1) (1,1)}
};
$OUTER = @DEFINE-SYMBOL {
    @RECTANGLE(fill="fillcolor", fillcolor="#112233") {(-1,-1) (1,1)};
    $FRAGMENT;
    (0,0) @TEXT("between", textcolor="#445566");
    $FRAGMENT;
    @RECTANGLE(fill="fillcolor", fillcolor="#778899") {(-1,-1) (1,1)}
};
#R1 @SET-SYMBOL $OUTER => @PLACE(0,0) => @ORIENT("R90")"##,
    );
    assert_eq!(
        colors(&r),
        ["#112233", "#ff0000", "#00ff00", "#0000ff", "#445566", "#ff0000", "#00ff00", "#0000ff", "#778899"]
    );
    let rendered = svg(&r.model.levels[0], &RenderOptions::default());
    let mut tail = rendered.as_str();
    for color in colors(&r) {
        let index = tail.find(&color).unwrap();
        tail = &tail[index + color.len()..];
    }
}

#[test]
fn nested_order_does_not_depend_on_a_fixed_primitive_count() {
    let mut body = String::from(
        "$MARK = @DEFINE(roles=[\"at\"]) {};\n$INNER = @DEFINE-SYMBOL { @RECTANGLE(fill=\"fillcolor\", fillcolor=\"#ff0000\") {(-1,-1) (1,1)} };\n$OUTER = @DEFINE-SYMBOL {\n",
    );
    for _ in 0..130 {
        body.push_str("@RECTANGLE(fill=\"fillcolor\", fillcolor=\"#112233\") {(-1,-1) (1,1)};\n");
    }
    body.push_str(
        "$MARK @ATTACH (0,0) => @SET-SYMBOL $INNER;\n@RECTANGLE(fill=\"fillcolor\", fillcolor=\"#0000ff\") {(-1,-1) (1,1)} };\n#R1 @SET-SYMBOL $OUTER => @PLACE(0,0)",
    );
    let actual = colors(&read(&body));
    assert_eq!(actual.len(), 132);
    assert!(actual[..130].iter().all(|color| color == "#112233"));
    assert_eq!(&actual[130..], ["#ff0000", "#0000ff"]);
}
