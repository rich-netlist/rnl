//! Invalid command options fail before reading input or producing output.
use std::process::Command;

fn reject(args: &[&str], code: &str, detail: &str) {
    let result = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).output().unwrap();
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert_eq!(result.status.code(), Some(2), "{args:?}: {stderr}");
    assert!(result.stdout.is_empty(), "{args:?}");
    assert!(stderr.contains(code) && stderr.contains(detail), "{args:?}: {stderr}");
    assert!(!stderr.contains("cannot read"), "{args:?}: {stderr}");
}

#[test]
fn an_unknown_dialect_is_rejected_before_reading_a_file() {
    reject(&["check", "missing.sp", "--dialect", "spcie"], "view-format-unknown", "spcie");
    reject(&["write", "missing.json", "-o", "unused.sp", "--dialect", "spcie"], "view-format-unknown", "--dialect");
}

#[test]
fn options_that_used_to_be_ignored_identify_the_option_and_command() {
    for options in [
        vec!["--level", "cell"],
        vec!["--parts", "host"],
        vec!["--author", "test"],
        vec!["--report"],
        vec!["--in-place"],
        vec!["--solver-options", "time_limit=none"],
        vec!["--previous"],
        vec!["--png"],
        vec!["--scale", "2"],
        vec!["--primitives"],
        vec!["--no-svg"],
        vec!["--status"],
        vec!["--flylines"],
        vec!["--net-names"],
        vec!["--hints"],
        vec!["--package", "example"],
        vec!["--all-versions"],
        vec!["-o", "unused"],
    ] {
        let mut args = vec!["check", "missing.sp"];
        args.extend(options.iter().copied());
        reject(&args, "cli-option-wrong-command", options[0]);
    }
}

#[test]
fn a_format_override_uses_the_shared_error_message() {
    reject(&["check", "missing.sp", "--format", "missing.sp=spcie"], "view-format-unknown", "spcie");
    let message = rnl_view::validate_format("spcie", "file.inc").unwrap_err();
    assert!(message.contains("file.inc") && message.contains("spice") && message.contains("spectre"));
    for name in ["spice", "spectre", "rnl", "text"] {
        assert!(rnl_view::validate_format(name, "file").is_ok());
    }
}

#[test]
fn overlay_errors_name_the_invalid_field_and_value() {
    for (source, field, value) in [
        (r##"{"palette":["#123456","bad"]}"##, "palette[1]", "bad"),
        (r#"{"current_color":"wrong"}"#, "current_color", "wrong"),
    ] {
        let options = rnl_view::OverlayOptions { style: serde_json::from_str(source).unwrap(), ..Default::default() };
        let message = options.validate().unwrap_err();
        assert!(message.contains(field) && message.contains(value), "{message}");
    }
}

#[test]
fn unknown_commands_and_missing_option_values_have_numbered_errors() {
    reject(&["unknown-command"], "cli-unknown-command", "unknown-command");
    reject(&["render", "missing.sp", "--overlay-style"], "cli-option-needs-value", "--overlay-style");
}

#[test]
fn invalid_overlay_values_keep_their_registered_codes_in_the_cli() {
    let dir = std::env::temp_dir().join(format!("rnl-overlay-arguments-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("style.json");
    for (style, code, detail) in [
        (r#"{"palette":["bad"]}"#, "view-overlay-colors", "palette[0]"),
        (r#"{"current_color":"wrong"}"#, "view-overlay-colors", "wrong"),
        (r#"{"width":0}"#, "view-overlay-width", "width"),
        (r#"{"dashes":[-1]}"#, "view-overlay-width", "dash"),
    ] {
        std::fs::write(&file, style).unwrap();
        reject(&["render", "missing.sp", "--overlay-style", file.to_str().unwrap()], code, detail);
    }
    std::fs::remove_file(file).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
