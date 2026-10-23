//! The shared native frontend codec (§3.1): UTF-8, with optional BOM, or Latin-1.
//! Reading strips a UTF-8 BOM. Saving preserves the source encoding unless the user explicitly
//! requests UTF-8; an unrepresentable character is an error before any output is written.

use serde::Deserialize;

const BOM: &[u8] = &[0xef, 0xbb, 0xbf];

/// A user's output choice. Explicit UTF-8 adds a BOM only to protect leading text U+FEFF.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputEncoding {
    #[default]
    Preserve,
    Utf8,
}

impl std::str::FromStr for OutputEncoding {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "preserve" => Ok(Self::Preserve),
            "utf8" | "utf-8" => Ok(Self::Utf8),
            _ => Err(format!("unknown encoding {value:?}: use preserve or utf8")),
        }
    }
}

/// Decode the bytes as the native providers read them, without changing line endings.
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.strip_prefix('\u{feff}').unwrap_or(s).to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

// The reader removes one initial U+FEFF as an encoding BOM. Keep a leading text U+FEFF
// behind a separate BOM so saving and reopening cannot consume it.
fn utf8(text: &str) -> Vec<u8> {
    if text.starts_with('\u{feff}') {
        [BOM, text.as_bytes()].concat()
    } else {
        text.as_bytes().to_vec()
    }
}

/// Encode decoded text using the source bytes, never the destination file's encoding.
/// A new source has UTF-8 encoding. An encoding BOM is separate from any leading U+FEFF in text.
pub fn encode(original: Option<&[u8]>, text: &str, encoding: OutputEncoding) -> Result<Vec<u8>, String> {
    if encoding == OutputEncoding::Utf8 {
        return Ok(utf8(text));
    }
    match original {
        Some(bytes) if std::str::from_utf8(bytes).is_err() => text
            .chars()
            .map(|c| {
                u8::try_from(u32::from(c)).map_err(|_| {
                    format!(
                        "the source is Latin-1 and cannot represent {c:?}; \
                         explicitly select UTF-8 conversion to save it"
                    )
                })
            })
            .collect(),
        Some(bytes) if bytes.starts_with(BOM) => Ok([BOM, text.as_bytes()].concat()),
        _ => Ok(utf8(text)),
    }
}

/// Encode against an actual source file. A missing/unreadable named source cannot establish
/// its original encoding and therefore fails in preserve mode; only `None` is a new memory source.
pub fn encode_file(source: Option<&std::path::Path>, text: &str, encoding: OutputEncoding) -> Result<Vec<u8>, String> {
    if encoding == OutputEncoding::Utf8 {
        return encode(None, text, encoding);
    }
    let original = source
        .map(|path| {
            std::fs::read(path).map_err(|e| {
                format!(
                    "cannot read {} to preserve its encoding: {e}; explicitly select UTF-8 conversion to save it",
                    path.display()
                )
            })
        })
        .transpose()?;
    encode(original.as_deref(), text, encoding)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserve_roundtrips_latin1_utf8_bom_and_line_endings() {
        for bytes in [
            b"* caf\xe9\r\n.end".as_slice(),
            "* café\n.end\n".as_bytes(),
            b"\xef\xbb\xbf* ascii\r\n",
            b"\xef\xbb\xbf\xef\xbb\xbftext",
        ] {
            assert_eq!(encode(Some(bytes), &decode(bytes), OutputEncoding::Preserve).unwrap(), bytes);
        }
        assert_eq!(encode(None, "Ω", OutputEncoding::Preserve).unwrap(), "Ω".as_bytes());
    }

    #[test]
    fn utf8_conversion_preserves_leading_text_feff_when_reopened() {
        for (original, expected) in [
            (b"* caf\xe9\n".as_slice(), "* café\n".as_bytes()),
            (b"\xef\xbb\xbftext", b"text".as_slice()),
            (b"\xef\xbb\xbf\xef\xbb\xbftext", b"\xef\xbb\xbf\xef\xbb\xbftext".as_slice()),
        ] {
            let text = decode(original);
            let converted = encode(Some(original), &text, OutputEncoding::Utf8).unwrap();
            assert_eq!(converted, expected);
            assert_eq!(decode(&converted), text);
        }
        // A new memory source or an edit adding U+FEFF to an unmarked UTF-8 source
        // needs the same distinction, including with the default output choice.
        for original in [None, Some(b"text".as_slice())] {
            for encoding in [OutputEncoding::Preserve, OutputEncoding::Utf8] {
                let text = "\u{feff}text";
                let encoded = encode(original, text, encoding).unwrap();
                assert_eq!(encoded, b"\xef\xbb\xbf\xef\xbb\xbftext");
                assert_eq!(decode(&encoded), text);
            }
        }
    }

    #[test]
    fn latin1_refuses_new_wide_characters_unless_utf8_is_explicit() {
        let original = b"* caf\xe9\r\n";
        let text = "* café Ω 中\r\n";
        let error = encode(Some(original), text, OutputEncoding::Preserve).unwrap_err();
        assert!(error.contains("Latin-1") && error.contains("UTF-8"), "{error}");
        assert_eq!(encode(Some(original), text, OutputEncoding::Utf8).unwrap(), text.as_bytes());
        assert_eq!(encode(Some(b"\xef\xbb\xbftext"), "text", OutputEncoding::Utf8).unwrap(), b"text");
    }
}
