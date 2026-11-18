//! Parameter failures found by checking the vocabulary (5.3, 6.4, 6.12, 9.2.4).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(path: &str, text: &str) -> Interpretation {
    let files = MemoryProvider::new().with(path, text);
    interpret_file(Path::new(path), None, &files, &Options::default()).unwrap()
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn pin_file(terminal: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n\
         * @RNL {{ $S = @DEFINE-SYMBOL(box=[-1,-1,1,1]) {{ (0,0) @PIN(terminal={terminal}); (0,1) @PIN(terminal=2) }}; #R1 @SET-SYMBOL $S; #R1 @PLACE(0,0) }}\n.end\n"
    )
}

#[test]
fn invalid_pin_terminals_fail_instead_of_becoming_different_terminals() {
    for terminal in ["[1,true]", "[1,[2]]", "[false]", "1.5", "0", "-1", "4294967296", "[1,2.5]", "[]"] {
        let r = read("t.sp", &pin_file(terminal));
        assert_eq!(errors(&r).len(), 1, "{terminal}: {:?}", errors(&r));
        let symbol = r.model.levels[0].symbols.iter().find(|s| s.name == "$S").unwrap();
        // The failed pin contributes nothing; the next, valid pin still takes effect.
        assert_eq!(symbol.pins.len(), 1, "{terminal}");
        assert_eq!(symbol.pins[0].terminals, ["2"], "{terminal}");
    }
}

#[test]
fn numeric_named_and_mixed_bus_pin_terminals_remain_valid() {
    for terminal in ["1", "1.0", "\"p\"", "[1,\"p\"]"] {
        let r = read("t.sp", &pin_file(terminal));
        assert!(errors(&r).is_empty(), "{terminal}: {:?}", errors(&r));
        let symbol = r.model.levels[0].symbols.iter().find(|s| s.name == "$S").unwrap();
        assert_eq!(symbol.pins.len(), 2, "{terminal}");
    }
}

#[test]
fn malformed_device_names_do_not_shift_or_rename_terminals() {
    for terminals in ["[\"first\",true,\"last\"]", "[7,\"last\"]", "[[\"first\"],\"last\"]"] {
        let text = format!(
            "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @DEVICE(name=\"R*\", terminals={terminals}) }}\nR1 a b 1k\n.end\n"
        );
        let r = read("t.sp", &text);
        assert_eq!(errors(&r), ["the terminals of @DEVICE must be an array of names (strings) (6.12)"], "{terminals}");
        assert_eq!(r.model.levels[0].terminal_names[0], [Some("p".into()), Some("n".into())]);
    }
}

#[test]
fn header_parameters_reject_wrong_types_before_using_their_defaults() {
    let cases = [
        ("@DEVICE(name=\"R*\", model=7, terminals=[\"first\",\"last\"])", "model", "DEVICE", "a string"),
        ("@DEVICE(name=\"R*\", terminals=\"first\")", "terminals", "DEVICE", "an array"),
        ("@USE-PACKAGE(name=\"rnl-std-all\", version=7)", "version", "USE-PACKAGE", "a string"),
        ("@USE-PACKAGE(name=\"rnl-std-all\", path=false)", "path", "USE-PACKAGE", "a string"),
        ("@USE-ALIASES(name=\"alias-zh\", version=7)", "version", "USE-ALIASES", "a string"),
        ("@USE-ALIASES(name=\"alias-zh\", path=false)", "path", "USE-ALIASES", "a string"),
    ];
    for (declaration, parameter, keyword, expected) in cases {
        let text = format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); {declaration} }}\nR1 a b 1k\n.end\n");
        let r = read("t.sp", &text);
        assert_eq!(errors(&r), [format!("the {parameter} of @{keyword} must be {expected} (5.3)")], "{declaration}");
        assert_eq!(r.model.levels[0].terminal_names[0], [Some("p".into()), Some("n".into())]);
    }
    for (declaration, parameter) in
        [("@PACKAGE(name=7, version=\"1.0\")", "name"), ("@PACKAGE(name=\"test\", version=true)", "version")]
    {
        let r = read("t.rnl", &format!("@RNL(type=\"header\") {{ @VERSION(\"0.4\"); {declaration} }}\n"));
        assert!(
            errors(&r).contains(&format!("the {parameter} of @PACKAGE must be a string (5.3)")),
            "{declaration}: {:?}",
            errors(&r)
        );
    }
}

#[test]
fn parameterless_keywords_reject_parameters_before_changing_objects() {
    for (keyword, statement) in [
        ("SET-SYMBOL", "#R1 @SET-SYMBOL(bogus=true) $S"),
        ("SET-STYLE", "#R1 @SET-STYLE(bogus=true) $style"),
        ("GROUP", "%members @GROUP(bogus=true)"),
        ("BUNDLE", "%members @BUNDLE(bogus=true)"),
        ("ADD-TO", "#R1 %members @ADD-TO(bogus=true)"),
        ("REMOVE-FROM", "#R1 %members @REMOVE-FROM(bogus=true)"),
        ("UNION", "%members @UNION(bogus=true)"),
    ] {
        let text = format!(
            "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n\
             * @RNL {{ $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1) }}; $style = @DEFINE-STYLE {{}}; %members = {{#R1}}; {statement}; #R1 @PLACE(0,0) }}\n.end\n"
        );
        let r = read("t.sp", &text);
        assert_eq!(errors(&r).len(), 1, "{statement}: {:?}", errors(&r));
        assert!(errors(&r)[0].starts_with(&format!("@{keyword} has no parameter bogus")), "{:?}", errors(&r));
        // A malformed modifying statement must not prevent later, independent facts from applying.
        assert_eq!(r.model.levels[0].items.iter().find(|i| i.id == "#R1").unwrap().at, (0.0, 0.0));
    }
}

#[test]
fn terminal_selection_rejects_arrays_and_nonpositive_or_fractional_ordinals() {
    for terminal in ["true", "[]", "[1]", "1.5", "0", "-1", "4294967296"] {
        let text = format!(
            "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n\
             * @RNL {{ #R1 @PLACE(0,0); #R1 @SELECT-TERMINAL(terminal={terminal}) => @TEXT(\"invalid\", key=\"bad\"); #R1 @TEXT(\"valid\", key=\"good\") }}\n.end\n"
        );
        let r = read("t.sp", &text);
        assert_eq!(
            errors(&r),
            [
                "the terminal of @SELECT-TERMINAL must be a positive terminal number or a terminal name (5.3)",
                "the object the TEXT is attached to failed"
            ],
            "{terminal}"
        );
        let texts = &r.model.levels[0].texts;
        assert!(!texts.iter().any(|t| t.markup == "invalid"), "{terminal}");
        assert!(texts.iter().any(|t| t.markup == "valid"), "{terminal}");
    }
}
