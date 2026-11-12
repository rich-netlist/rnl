use serde::Deserialize;
use serde_json::Value;

/// Parse a protocol message with exact floating-point round trips and enough nesting for
/// the runtime's 64-level markup. The structural bound is checked before recursive parsing.
pub fn parse_json(bytes: &[u8]) -> Result<Value> {
    let (mut depth, mut string, mut escape) = (0usize, false, false);
    for &b in bytes {
        if string {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                string = false;
            }
        } else {
            match b {
                b'"' => string = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 256 {
                        return Err(Error { path: "$".into(), message: "JSON nesting exceeds 256 levels".into() });
                    }
                }
                b'}' | b']' => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
        }
    }
    let text = std::str::from_utf8(bytes).map_err(|e| Error { path: "$".into(), message: e.to_string() })?;
    let raw: &serde_json::value::RawValue = read_raw(text)?;
    exact_value(raw)
}

fn read_raw<'a, T: Deserialize<'a>>(text: &'a str) -> Result<T> {
    let mut parser = serde_json::Deserializer::from_str(text);
    parser.disable_recursion_limit();
    let error = |e: serde_json::Error| Error { path: "$".into(), message: e.to_string() };
    let value = T::deserialize(&mut parser).map_err(error)?;
    parser.end().map_err(error)?;
    Ok(value)
}

// Use the standard float parser on raw numeric tokens. Enabling serde_json's global
// float_roundtrip feature would also change existing CLI/Viewer JSON readers through
// Cargo feature unification; the protocol must not change those released code paths.
fn exact_value(raw: &serde_json::value::RawValue) -> Result<Value> {
    let text = raw.get();
    Ok(match text.as_bytes().first().copied() {
        Some(b'{') => {
            let fields: std::collections::BTreeMap<String, &serde_json::value::RawValue> = read_raw(text)?;
            Value::Object(fields.into_iter().map(|(k, v)| Ok((k, exact_value(v)?))).collect::<Result<_>>()?)
        }
        Some(b'[') => {
            let values: Vec<&serde_json::value::RawValue> = read_raw(text)?;
            Value::Array(values.into_iter().map(exact_value).collect::<Result<_>>()?)
        }
        Some(b'"') => Value::String(read_raw(text)?),
        Some(b't') => Value::Bool(true),
        Some(b'f') => Value::Bool(false),
        Some(b'n') => Value::Null,
        _ => {
            if !text.contains(['.', 'e', 'E']) && text != "-0" {
                if let Ok(n) = text.parse::<u64>() {
                    return Ok(Value::Number(n.into()));
                }
                if let Ok(n) = text.parse::<i64>() {
                    return Ok(Value::Number(n.into()));
                }
            }
            let number = text
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .ok_or_else(|| Error { path: "$".into(), message: "expected a finite JSON number".into() })?;
            Value::Number(number)
        }
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}
impl std::error::Error for Error {}

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub(crate) struct Node<'a> {
    pub value: &'a Value,
    pub path: String,
}

impl<'a> Node<'a> {
    pub fn root(value: &'a Value) -> Self {
        Self { value, path: "$".into() }
    }
    pub fn error(&self, message: impl Into<String>) -> Error {
        Error { path: self.path.clone(), message: message.into() }
    }
    pub fn field(&self, name: &str) -> Result<Self> {
        let object = self.value.as_object().ok_or_else(|| self.error("expected an object"))?;
        Ok(Self {
            value: object
                .get(name)
                .ok_or_else(|| Error { path: format!("{}.{}", self.path, name), message: "missing field".into() })?,
            path: format!("{}.{}", self.path, name),
        })
    }
    pub fn string(&self) -> Result<&'a str> {
        self.value.as_str().ok_or_else(|| self.error("expected a string"))
    }
    pub fn boolean(&self) -> Result<bool> {
        self.value.as_bool().ok_or_else(|| self.error("expected a boolean"))
    }
    pub fn uint(&self) -> Result<u64> {
        self.value.as_u64().ok_or_else(|| self.error("expected a nonnegative integer"))
    }
    pub fn integer(&self) -> Result<i64> {
        self.value.as_i64().ok_or_else(|| self.error("expected a signed integer"))
    }
    pub fn u32(&self) -> Result<u32> {
        u32::try_from(self.uint()?).map_err(|_| self.error("integer exceeds u32"))
    }
    pub fn u16(&self) -> Result<u16> {
        u16::try_from(self.uint()?).map_err(|_| self.error("integer exceeds u16"))
    }
    pub fn index(&self) -> Result<usize> {
        usize::try_from(self.uint()?).map_err(|_| self.error("index exceeds this platform's range"))
    }
    pub fn number(&self) -> Result<f64> {
        self.value.as_f64().filter(|n| n.is_finite()).ok_or_else(|| self.error("expected a finite number"))
    }
    pub fn list<T>(&self, mut parse: impl FnMut(Node<'a>) -> Result<T>) -> Result<Vec<T>> {
        self.value
            .as_array()
            .ok_or_else(|| self.error("expected an array"))?
            .iter()
            .enumerate()
            .map(|(i, value)| parse(Self { value, path: format!("{}[{i}]", self.path) }))
            .collect()
    }
    pub fn nullable<T>(&self, parse: impl FnOnce(Node<'a>) -> Result<T>) -> Result<Option<T>> {
        if self.value.is_null() {
            Ok(None)
        } else {
            parse(self.clone()).map(Some)
        }
    }
    pub fn point(&self) -> Result<(f64, f64)> {
        let coords = self.list(|v| v.number())?;
        match coords.as_slice() {
            [x, y] => Ok((*x, *y)),
            _ => Err(self.error("expected two coordinates")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_numbers_round_trip_without_enabling_global_float_parsing_features() {
        let mut bits = 0x5a1717d2f96c402du64;
        for value in [0., -0., f64::MIN_POSITIVE, f64::from_bits(1), f64::MAX, 0.02881699800591428]
            .into_iter()
            .chain((0..64).map(|_| {
                bits = bits.wrapping_mul(6364136223846793005).wrapping_add(1);
                f64::from_bits(bits)
            }))
            .filter(|n| n.is_finite())
        {
            let parsed = parse_json(value.to_string().as_bytes()).unwrap();
            assert_eq!(parsed.as_f64().unwrap().to_bits(), value.to_bits(), "{value}");
        }
        let value =
            parse_json(b" \n { \"values\": [ 9007199254740993, -7, 1.25, true, null, \"\\\"[{\" ] } \n").unwrap();
        assert_eq!(value["values"][0].as_u64(), Some(9007199254740993));
        assert_eq!(value["values"][1].as_i64(), Some(-7));
        assert_eq!(value["values"][2].as_f64(), Some(1.25));
        assert_eq!(value["values"][3].as_bool(), Some(true));
        assert!(value["values"][4].is_null());
        assert_eq!(value["values"][5].as_str(), Some("\"[{"));
        for text in ["01.5", "1.2.3", "1e309", "[1,]", "{\"x\":1} false"] {
            assert!(parse_json(text.as_bytes()).is_err(), "{text}");
        }
    }
}
