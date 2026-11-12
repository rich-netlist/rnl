//! Modern diagnostic locations link tool settings and geometry hints to their document.
use std::path::Path;

use rnl_runtime::diag::codes;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Options;
use rnl_view::hints::{interpretation_hints, Kind};

fn source(version: &str) -> String {
    format!("* locations\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ &a @WIRE(mode=\"straight\") {{(0,0) (2,2)}} }}\n* @RNL {{ &b @WIRE(mode=\"straight\") {{(0,2) (2,0)}} }}\n")
}

#[test]
fn a_crossing_links_to_a_wire_and_keeps_its_coordinates() {
    for version in ["0.2", "0.3", "0.4"] {
        let provider = MemoryProvider::new().with("main.sp", &source(version));
        let r = rnl_cli::interpret_file(
            Path::new("main.sp"),
            None,
            &provider,
            &Options { std: false, ..Options::solver() },
        )
        .unwrap();
        let hints = interpretation_hints(&r);
        let crossing = hints.iter().flat_map(|lv| &lv.hints).find(|h| h.kind == Kind::Crossings).unwrap();
        assert!(crossing.message.contains("1 crossing of wires") && crossing.message.contains("at (1, 1)"));
        let loc = crossing.loc.as_ref().unwrap();
        assert!(loc.file.ends_with("main.sp"));
        if version == "0.4" {
            assert!([4, 5].contains(&loc.line));
        } else {
            assert_eq!(loc.line, 1);
        }
    }
}

#[test]
fn crossing_locations_follow_the_included_levels_language() {
    for (root, child) in [("0.3", "0.4"), ("0.4", "0.3")] {
        let main =
            format!("* main\n* @RNL(type=\"header\") {{ @VERSION(\"{root}\") }}\n.include child.inc\nX1 a b cell\n");
        let child_text = source(child).replace("R1 a b 1k", ".subckt cell a b\nR1 a b 1k") + ".ends cell\n";
        let provider = MemoryProvider::new().with("main.sp", &main).with("child.inc", &child_text);
        let r = rnl_cli::interpret_file(
            Path::new("main.sp"),
            None,
            &provider,
            &Options { std: false, ..Options::solver() },
        )
        .unwrap();
        let hints = interpretation_hints(&r);
        let crossing = hints
            .iter()
            .find(|lv| lv.title == "cell")
            .unwrap()
            .hints
            .iter()
            .find(|h| h.kind == Kind::Crossings)
            .unwrap();
        let loc = crossing.loc.as_ref().unwrap();
        assert!(loc.file.ends_with("child.inc"));
        if child == "0.4" {
            assert!([5, 6].contains(&loc.line));
        } else {
            assert_eq!(loc.line, 3);
        }
    }
}

#[test]
fn a_missing_binding_is_linked_to_the_modern_main_document() {
    for version in ["0.2", "0.3", "0.4"] {
        let provider = MemoryProvider::new().with("main.sp", &source(version));
        let options = rnl_view::BuildOptions {
            runtime: Options { std: false, ..Default::default() },
            bind: Some(rnl_view::Bind { subckt: "absent".into(), ..Default::default() }),
            ..Default::default()
        };
        let document = rnl_view::build("main.sp", &provider, &options).unwrap();
        let diagnostic = document.diagnostics.iter().find(|d| d.code == codes::VIEW_BIND_NO_SUBCKT.number()).unwrap();
        assert_eq!(diagnostic.message, "no subcircuit absent to bind parameters to");
        if version == "0.4" {
            assert_eq!(diagnostic.file, "main.sp");
            assert_eq!(diagnostic.line, 1);
            assert!(diagnostic.file_index.is_some());
        } else {
            assert_eq!(diagnostic.line, 0);
            assert!(diagnostic.file.is_empty() && diagnostic.file_index.is_none());
        }
    }
}

#[test]
fn text_errors_point_to_the_call_instead_of_the_record_start() {
    for version in ["0.3", "0.4"] {
        for (body, code) in [
            ("(8,8) @CIRCLE(radius=1); (0,0) @TEXT(\"bad\",key=\"x\")", codes::TEXT_POINT_TAKES_NO_KEY),
            ("(8,8) @CIRCLE(radius=1); &a @TEXT(\"bad\",dx=1)", codes::TEXT_WIRE_NET_NO_OFFSET),
            ("$D = @DEFINE(roles=[]) {}; $D @TEXT(\"bad\")", codes::TEXT_ATTACH_NOT_ALLOWED),
        ] {
            let line = format!("* @RNL {{ {body} }}");
            let text =
                format!("* text locations\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n{line}\n");
            let provider = MemoryProvider::new().with("main.sp", &text);
            let r = rnl_cli::interpret_file(
                Path::new("main.sp"),
                None,
                &provider,
                &Options { std: false, ..Options::solver() },
            )
            .unwrap();
            let matching: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == code).collect();
            assert_eq!(matching.len(), 1, "{version} {body}: {:?}", r.diagnostics());
            let loc = matching[0].loc.as_ref().unwrap();
            assert_eq!(loc.line, 4);
            assert_eq!(loc.col as usize, if version == "0.4" { line.find("@TEXT").unwrap() + 1 } else { 3 });
        }
    }
}
