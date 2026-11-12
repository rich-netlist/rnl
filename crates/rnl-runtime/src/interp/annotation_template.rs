//! Inspect general-definition syntax for ANNOTATION without applying the definition.
//!
//! This is intentionally a prefilter, not an evaluator. A caller must resolve aliases in the
//! definition's lexical unit, validate graphic parameters and produce the final draw objects.
//! In particular, `Deferred` must never be treated as an eligible literal or as a warning: a
//! member reference or pipe can require lexical resolution, and `?` is legal only for TEXT.

use std::rc::Rc;

use crate::syntax::{Coord, Member, ObjExpr, Segment, SetExpr};

/// One direct drawing description in the original definition body. The original member and
/// segment retain parameter spelling, source location, aliases, style, and declaration order.
#[derive(Clone, Debug)]
pub(crate) struct TemplateMember {
    pub member: Rc<Member>,
    pub segment: usize,
    pub input_segment: usize,
    pub keyword: String,
    pub input: InputKind,
}

/// Preclassification of the written inputs, before any evaluation or role substitution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputKind {
    /// Every input point was written as a finite numeric point; a text may use the owner `?`.
    Literal,
    /// A direct host reference or role reference cannot denote a literal point of the template.
    OtherObject,
    /// A pipe, placeholder coordinate, empty set, or nested operation requires lexical
    /// analysis; caller must resolve its written origin without evaluating a general definition.
    Deferred,
}

fn merge(a: InputKind, b: InputKind) -> InputKind {
    use InputKind::*;
    match (a, b) {
        (OtherObject, _) | (_, OtherObject) => OtherObject,
        (Deferred, _) | (_, Deferred) => Deferred,
        _ => Literal,
    }
}

fn input(obj: &ObjExpr, text: bool) -> InputKind {
    use InputKind::*;
    match obj {
        ObjExpr::Point(p) => match (&p.x, &p.y) {
            (Coord::Num(x), Coord::Num(y)) if x.is_finite() && y.is_finite() => Literal,
            _ => Deferred,
        },
        ObjExpr::Ref(r) if text && r.prefix == '?' && r.name.is_none() && r.path.is_empty() => Literal,
        ObjExpr::Ref(r) if matches!(r.prefix, '#' | '&' | '?' | '%' | '$') => OtherObject,
        ObjExpr::Ref(_) => Deferred,
        ObjExpr::Set(set) => {
            if set.members.is_empty() {
                return Deferred;
            }
            set.members.iter().fold(Literal, |kind, member| {
                if member.error.is_some() || member.segments.len() != 1 {
                    return merge(kind, Deferred);
                }
                let segment = &member.segments[0];
                if segment.call.is_some() || segment.args.is_empty() {
                    return merge(kind, Deferred);
                }
                segment.args.iter().fold(kind, |k, a| merge(k, input(&a.obj, false)))
            })
        }
    }
}

fn segment_input(segment: &Segment, text: bool) -> InputKind {
    // A pipe can supply its input from its predecessor even when explicit args are present.
    if segment.piped || segment.args.is_empty() {
        return InputKind::Deferred;
    }
    segment.args.iter().fold(InputKind::Literal, |k, arg| merge(k, input(&arg.obj, text)))
}

/// Whether a set contains another written set as an element. A graphic's point
/// set is one level: nested sets are not point objects.
pub(crate) fn nested_set(expr: &ObjExpr) -> bool {
    let ObjExpr::Set(set) = expr else {
        return false;
    };
    set.members.iter().any(|member| {
        member.segments.iter().any(|segment| segment.args.iter().any(|arg| matches!(arg.obj, ObjExpr::Set(_))))
    })
}

/// Resolve parameter holes without evaluating or applying the definition body. A missing
/// coordinate is a free point and therefore not a literal drawing input.
pub(crate) fn resolved_points<F>(expr: &ObjExpr, coord: &mut F) -> Option<Vec<(f64, f64)>>
where
    F: FnMut(&Coord, usize) -> Option<f64>,
{
    match expr {
        ObjExpr::Point(p) => Some(vec![(coord(&p.x, p.start)?, coord(&p.y, p.start)?)]),
        ObjExpr::Set(set) => {
            let mut out = Vec::new();
            for member in &set.members {
                if member.error.is_some() || member.segments.len() != 1 {
                    return None;
                }
                let segment = &member.segments[0];
                if segment.call.is_some() || segment.args.is_empty() {
                    return None;
                }
                for arg in &segment.args {
                    out.extend(resolved_points(&arg.obj, coord)?);
                }
            }
            Some(out)
        }
        _ => None,
    }
}

/// Collect only drawing descriptions directly in this body. This does not walk nested definitions,
/// AS/ATTACH bodies, execute rules, allocate usages, or apply role fillers. `keyword` resolves the
/// call's spelling in the definition's own lexical environment; a missing alias is ignored here and
/// remains the normal definition-validation diagnostic. Non-drawing statements are ignored.
pub(crate) fn direct_members<F>(body: &SetExpr, mut keyword: F) -> Vec<TemplateMember>
where
    F: FnMut(&str) -> Option<String>,
{
    let mut out = Vec::new();
    for member in &body.members {
        if member.error.is_some() {
            continue;
        }
        for (index, segment) in member.segments.iter().enumerate() {
            let Some(call) = &segment.call else {
                continue;
            };
            let Some(kw) = keyword(&call.kw) else {
                continue;
            };
            let text = kw == "TEXT";
            if !text && !matches!(kw.as_str(), "LINE" | "RECTANGLE" | "CIRCLE" | "ELLIPSE" | "ARC" | "BEZIER") {
                continue;
            }
            let input_segment =
                if segment.piped && index > 0 && member.segments[index - 1].call.is_none() { index - 1 } else { index };
            let input = if input_segment == index {
                segment_input(segment, text)
            } else {
                let piped = segment_input(&member.segments[input_segment], text);
                segment.args.iter().fold(piped, |kind, arg| merge(kind, input(&arg.obj, text)))
            };
            out.push(TemplateMember { member: Rc::clone(member), segment: index, input_segment, keyword: kw, input });
        }
    }
    out
}

/// Mirror about the vertical axis first, then rotate counterclockwise, then translate. This helper
/// is independent of symbol `Orient`; the text owner's separate upright/fix_rotate rules still apply.
#[cfg(test)]
fn transform(local: (f64, f64), anchor: (f64, f64), rotate: f64, mirror: bool) -> (f64, f64) {
    let (x, y) = if mirror { (-local.0, local.1) } else { local };
    let (s, c) = rotate.to_radians().sin_cos();
    (anchor.0 + c * x - s * y, anchor.1 + s * x + c * y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse_record;

    #[test]
    fn direct_filter_never_applies_the_general_definition() {
        let body = parse_record(
            r#"@RNL { (0,0) (1,1) @LINE; ? @TEXT("x"); ?p (1,0) @LINE; $CHILD @ATTACH (2,3); @PLACE(3,4) }"#,
        )
        .body
        .unwrap();
        let got = direct_members(&body, |kw| Some(kw.to_owned()));
        assert_eq!(
            got.iter().map(|x| (x.keyword.as_str(), x.input)).collect::<Vec<_>>(),
            [("LINE", InputKind::Literal), ("TEXT", InputKind::Literal), ("LINE", InputKind::OtherObject)]
        );
    }

    #[test]
    fn references_and_pipes_are_deferred_not_falsely_classified() {
        let body = parse_record(r#"@RNL { %p = (0,0); %p (1,1) @LINE; (2,3) => @TEXT("x") }"#).body.unwrap();
        let got = direct_members(&body, |kw| Some(kw.to_owned()));
        assert_eq!(got.iter().map(|x| x.input).collect::<Vec<_>>(), [InputKind::OtherObject, InputKind::Literal]);
    }

    #[test]
    fn mirror_precedes_rotation_and_translation() {
        let q = transform((2.0, 1.0), (10.0, 20.0), 90.0, true);
        assert!((q.0 - 9.0).abs() < 1e-12 && (q.1 - 18.0).abs() < 1e-12);
    }
}
