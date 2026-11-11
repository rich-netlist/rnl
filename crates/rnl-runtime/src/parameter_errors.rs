//! The same parameter error has the same kind and wording in a header or a statement.
use crate::diag::{codes, Code};
use crate::interp::unit::closest;
use crate::syntax::Named;

#[derive(Clone, Copy)]
pub(crate) enum Site<'a> {
    Header { standard: bool, first: &'a str },
    Body,
}

pub(crate) fn positional(kw: &str, site: Site<'_>, modern: bool) -> (Code, String) {
    if let Site::Header { first, .. } = site {
        if !modern {
            return (codes::HEADER_PARAM_NOT_POSITIONAL, format!("the parameters of @{kw} cannot be written by position; write each by name, such as {first}=… (4.2 rule 9)"));
        }
    }
    let example = if kw == "DEFINE" {
        ", such as @DEFINE(roles=[\"a\", \"b\"], parameters=[[\"gap\", 2]])"
    } else {
        ", as name=value"
    };
    (
        codes::PARAM_POSITIONAL_NOT_ALLOWED,
        format!("the parameters of @{kw} cannot be written by position; write each by name{example} (4.2 rule 9)"),
    )
}

pub(crate) fn too_many(kw: &str, order: &[String], actual: usize) -> (Code, String) {
    (
        codes::PARAM_TOO_MANY_POSITIONAL,
        format!(
            "@{kw} takes at most {} parameters by position ({}), {actual} are written (4.2 rule 9)",
            order.len(),
            order.join(", ")
        ),
    )
}

pub(crate) fn unknown(
    kw: &str,
    named: &Named,
    standard_name: &str,
    choices: &[&str],
    site: Site<'_>,
    modern: bool,
) -> (Code, String) {
    let mut message = format!("@{kw} has no parameter {}", named.name);
    if let Some(close) = closest(standard_name, choices.iter().copied()) {
        message.push_str(&format!(", did you mean {close}?"));
    }
    if !named.quoted && !named.name.is_ascii() {
        message.push_str(match site {
            Site::Header { standard: true, .. } => {
                "; a header record is read in the standard form, without aliases (16.5)"
            }
            Site::Body if !modern => " Is an alias package not loaded?",
            _ => "; is an alias package not loaded?",
        });
    }
    message.push_str(" (16.4)");
    let code =
        if !modern && matches!(site, Site::Header { .. }) { codes::HEADER_PARAM_UNKNOWN } else { codes::PARAM_UNKNOWN };
    (code, message)
}

pub(crate) fn twice(kw: &str, name: &str) -> (Code, String) {
    (codes::PARAM_WRITTEN_TWICE, format!("parameter {name} of @{kw} is written twice (4.2 rule 9)"))
}
