use crate::{
    data, decode_problem, encode_problem,
    json::{Error, Node, Result},
    VERSION,
};
use rnl_runtime::problem::{input_digest, LevelProblem};
use rnl_solver::SolverOptions;
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct LevelInput {
    pub problem: LevelProblem,
    /// Caller settings before file/package settings are applied. Explicit override metadata
    /// must survive transport so a file cannot replace an explicit caller value.
    pub options: SolverOptions,
}
#[derive(Clone, Debug, PartialEq)]
pub struct InputDocument {
    pub levels: Vec<LevelInput>,
}

fn options_value(options: &SolverOptions) -> Result<Value> {
    options.validate().map_err(|message| Error { path: "$.options".into(), message })?;
    let value = json!({"canonical":options.canonical(),"hints_for_low":options.hints_for_low,"safety_seconds":options.safety_seconds,"legacy_alias":options.settings.legacy_alias,"overrides":options.settings.overrides.iter().map(data::setting_value).collect::<Vec<_>>()});
    if decode_options(Node::root(&value))? != *options {
        return Err(Error {
            path: "$.options".into(),
            message: "caller settings are not losslessly representable".into(),
        });
    }
    Ok(value)
}
fn decode_options(n: Node<'_>) -> Result<SolverOptions> {
    let canonical = n.field("canonical")?;
    let mut options = SolverOptions::parse(canonical.string()?).map_err(|e| canonical.error(e))?;
    options.hints_for_low = n.field("hints_for_low")?.boolean()?;
    options.safety_seconds = n.field("safety_seconds")?.nullable(|n| n.number())?;
    options.settings.legacy_alias = n.field("legacy_alias")?.boolean()?;
    options.settings.overrides = n.field("overrides")?.list(data::setting)?;
    let mut keys = std::collections::BTreeSet::new();
    for setting in &options.settings.overrides {
        if !keys.insert(&setting.key) {
            return Err(n.field("overrides")?.error(format!("duplicate tool override {}", setting.key)));
        }
    }
    options.validate().map_err(|e| n.error(e))?;
    let overrides = options
        .settings
        .overrides
        .iter()
        .map(|s| format!("{}={}", s.key, rnl_runtime::problem::written_lit(&s.value)))
        .collect::<Vec<_>>()
        .join(",");
    let override_node = n.field("overrides")?;
    options.with_config(&overrides).map_err(|e| override_node.error(e))?;
    if options.canonical() != canonical.string()? {
        return Err(canonical.error("settings differ from their canonical representation"));
    }
    Ok(options)
}

/// A request contains the complete input for each requested level and the caller settings.
/// The original human-readable `rnl problem` projection is not this transport format.
pub fn encode_input(input: &InputDocument) -> Result<Value> {
    let levels = input
        .levels
        .iter()
        .enumerate()
        .map(|(request, l)| {
            let problem = encode_problem(&l.problem)?;
            let options = options_value(&l.options)?;
            Ok(json!({"request":request,"input":input_digest(&l.problem),"options":options,"problem":problem}))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"format":"rnl-solver-input","version":VERSION,"levels":levels}))
}

/// Read a versioned request and verify the problem digest and ordinal before solving.
pub fn decode_input(value: &Value) -> Result<InputDocument> {
    let n = Node::root(value);
    if n.field("format")?.string()? != "rnl-solver-input" {
        return Err(n.field("format")?.error("expected rnl-solver-input"));
    }
    if n.field("version")?.uint()? != VERSION {
        return Err(n.field("version")?.error("unsupported solver input protocol version"));
    }
    let mut next = 0usize;
    let levels = n.field("levels")?.list(|l| {
        let request = l.field("request")?;
        if request.index()? != next {
            return Err(request.error("request ordinals must match their array positions"));
        }
        next += 1;
        let p = l.field("problem")?;
        let problem = decode_problem(p.value)
            .map_err(|e| Error { path: format!("{}{}", p.path, e.path.trim_start_matches('$')), message: e.message })?;
        if input_digest(&problem) != l.field("input")?.string()? {
            return Err(l.field("input")?.error("problem digest does not match the input"));
        }
        let options = decode_options(l.field("options")?)?;
        Ok(LevelInput { problem, options })
    })?;
    Ok(InputDocument { levels })
}
