//! Literal tool overrides in the same text as SOLVER-CONFIG parentheses (14.10).
//! This API validates syntax only; the selected solver validates its own keys and values.

use crate::diag::codes;
use crate::interp::obj::{Lit, WStr};
use crate::problem::SolverSetting;
use crate::syntax::{parameter_text, ParseError, Value};
use std::collections::BTreeMap;

/// Parse explicit tool settings. There is no enclosing record, alias environment or source file.
/// Strings that name definitions are resolved in the target level by the consumer.
pub fn parse_overrides(text: &str) -> Result<Vec<SolverSetting>, ParseError> {
    let params = parameter_text(text)?;
    if !params.positional.is_empty() {
        return Err(ParseError {
            code: codes::PARAM_POSITIONAL_NOT_ALLOWED,
            message: "solver settings take named parameters, as key=value (14.10)".into(),
        });
    }
    fn literal(v: &Value, array: bool) -> Option<Lit> {
        Some(match v {
            Value::Num { value, .. } => Lit::Num(*value),
            Value::Str(s) => Lit::Str(WStr::new(s, None)),
            Value::Bool(b) => Lit::Bool(*b),
            Value::Arr(a) if !array => Lit::Arr(a.iter().map(|v| literal(v, true)).collect::<Option<_>>()?),
            _ => return None,
        })
    }
    params
        .named
        .into_iter()
        .map(|p| {
            let value = literal(&p.value, false).ok_or_else(|| ParseError {
                code: codes::SOLVER_CONFIG_VALUE_NOT_LITERAL,
                message: "a solver setting is a number, string, boolean, or flat array (14.10)".into(),
            })?;
            Ok(SolverSetting { key: p.name, value, source: None, package: String::new() })
        })
        .collect()
}

/// Merge only explicitly supplied tool values over file/package values: a key the tool gives has
/// the tool's last value alone, and every other key keeps all its values, in order (14.10).
/// Solver defaults are not overrides. The returned order is by key, independent of the tool's
/// spelling order.
pub fn with_overrides(settings: &[SolverSetting], text: &str) -> Result<Vec<SolverSetting>, ParseError> {
    let tool: BTreeMap<_, _> = parse_overrides(text)?.into_iter().map(|s| (s.key.clone(), s)).collect();
    let mut values: Vec<_> = settings.iter().filter(|s| !tool.contains_key(&s.key)).cloned().collect();
    values.extend(tool.into_values());
    values.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_text_round_trips_and_preserves_open_keys() {
        let text = r#""custom-key"=[1, "two, )", true], junction="$DOT", grid=false, spacing_x=-2"#;
        let settings = parse_overrides(text).unwrap();
        let output = settings.iter().map(|s| format!("\"{}\"={}", s.key, s.value.show())).collect::<Vec<_>>().join(",");
        assert_eq!(parse_overrides(&output).unwrap(), settings);
        assert!(settings.iter().all(|s| s.source.is_none()));
        assert!(parse_overrides(" /* comment */ ").unwrap().is_empty());
    }

    #[test]
    fn nonliteral_or_extra_syntax_is_rejected_atomically() {
        for bad in [
            "1",
            "x=?param",
            "x=[[1]]",
            "x=word",
            "x=#R1",
            "x=1,x=2",
            "x=1); @SOLVER-CONFIG(y=2",
            "x=1) /*",
            "x=1;y=2",
            "x=1) blah (",
        ] {
            assert!(parse_overrides(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn only_written_overrides_replace_existing_values() {
        let original = parse_overrides("a=1,b=2").unwrap();
        let values = with_overrides(&original, "b=3,c=4").unwrap();
        assert_eq!(
            values.iter().map(|s| (s.key.as_str(), s.value.show())).collect::<Vec<_>>(),
            [("a", "1".into()), ("b", "3".into()), ("c", "4".into())]
        );
        assert_eq!(with_overrides(&original, "").unwrap(), original);
        assert!(with_overrides(&original, "b=9,c=?bad").is_err());
        assert_eq!(original[1].value, Lit::Num(2.0));
        // a key the file gives twice keeps both values, unless the tool gives it
        let mut twice = parse_overrides("a=1,b=2").unwrap();
        twice.insert(1, parse_overrides("a=5").unwrap().remove(0));
        let values = with_overrides(&twice, "b=3").unwrap();
        assert_eq!(
            values.iter().map(|s| (s.key.as_str(), s.value.show())).collect::<Vec<_>>(),
            [("a", "1".into()), ("a", "5".into()), ("b", "3".into())]
        );
        let values = with_overrides(&twice, "a=7").unwrap();
        assert_eq!(
            values.iter().map(|s| (s.key.as_str(), s.value.show())).collect::<Vec<_>>(),
            [("a", "7".into()), ("b", "2".into())]
        );
    }
}
