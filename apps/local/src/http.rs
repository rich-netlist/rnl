//! The small piece of HTTP/1.1 the page needs, over `std::net`, with no crate: one request and one response on a
//! connection, which is then closed. The server is for one user's browser on the loopback address, so what it takes
//! is strict and small: a request line and headers of at most `MAX_HEAD` bytes, a body with a `Content-Length` of at
//! most `max_body` bytes, and nothing else (no chunked bodies, no upgrades, no pipelining).

use std::io::{self, Read, Write};
use std::net::TcpStream;

/// The most the request line and the headers may take.
const MAX_HEAD: usize = 64 * 1024;

/// A request, read.
#[derive(Debug)]
pub struct Request {
    pub method: String,
    /// The path with its percent-escapes decoded (UTF-8; no NUL).
    pub path: String,
    /// The query as sent, without the `?`.
    pub query: String,
    /// The headers: names in lower case, in the order sent.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    /// The value of a header, by its name in lower case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    /// The value of a cookie of the request.
    pub fn cookie(&self, name: &str) -> Option<&str> {
        self.headers.iter().filter(|(n, _)| n == "cookie").flat_map(|(_, v)| v.split(';')).find_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            (k.trim() == name).then(|| v.trim())
        })
    }

    /// The first value of a query parameter, decoded.
    pub fn param(&self, name: &str) -> Option<String> {
        self.query.split('&').find_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(k, true).as_deref() == Some(name)).then(|| decode(v, true)).flatten()
        })
    }
}

/// Why a request is not served: the status to answer with, and a word on why.
#[derive(Debug)]
pub struct Rejected(pub u16, pub &'static str);

/// A response: its status, headers and body.
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16, content_type: &str, body: impl Into<Vec<u8>>) -> Response {
        Response {
            status,
            headers: vec![("Content-Type".into(), content_type.into())],
            body: body.into(),
        }
    }

    pub fn text(status: u16, text: &str) -> Response {
        Response::new(status, "text/plain; charset=utf-8", text.as_bytes())
    }

    pub fn json(status: u16, value: &serde_json::Value) -> Response {
        Response::new(status, "application/json; charset=utf-8", value.to_string().into_bytes())
    }

    pub fn error(status: u16, message: &str) -> Response {
        Response::json(status, &serde_json::json!({ "error": message }))
    }

    pub fn redirect(to: &str) -> Response {
        let mut r = Response::new(302, "text/plain; charset=utf-8", Vec::new());
        r.headers.push(("Location".into(), to.into()));
        r
    }

    pub fn header(mut self, name: &str, value: &str) -> Response {
        self.headers.push((name.into(), value.into()));
        self
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        302 => "Found",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        _ => "Status",
    }
}

/// Write a response and flush it. Every response closes the connection and says it is never cached or sniffed.
pub fn write_response(stream: &mut TcpStream, response: &Response, head_only: bool) -> io::Result<()> {
    let mut head = format!("HTTP/1.1 {} {}\r\n", response.status, reason(response.status));
    for (name, value) in &response.headers {
        // (a header never holds a line end: the values are the program's own, but a path may reach one)
        if name.contains(['\r', '\n']) || value.contains(['\r', '\n']) {
            continue;
        }
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    head.push_str("Connection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\n\r\n");
    stream.write_all(head.as_bytes())?;
    if !head_only {
        stream.write_all(&response.body)?;
    }
    stream.flush()
}

/// Read one request from the stream.
pub fn read_request(stream: &mut TcpStream, max_body: usize) -> Result<Request, Rejected> {
    let mut buffer = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    let end = loop {
        if let Some(at) = find(&buffer, b"\r\n\r\n") {
            break at;
        }
        if buffer.len() > MAX_HEAD {
            return Err(Rejected(431, "the headers are too long"));
        }
        let n = stream.read(&mut chunk).map_err(|_| Rejected(400, "the request was not read"))?;
        if n == 0 {
            return Err(Rejected(400, "the request ended too soon"));
        }
        buffer.extend_from_slice(&chunk[..n]);
    };
    let head = std::str::from_utf8(&buffer[..end]).map_err(|_| Rejected(400, "the headers are not text"))?;
    let mut lines = head.split("\r\n");
    let line = lines.next().unwrap_or("");
    let mut parts = line.split(' ');
    let (method, target, version) = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(m), Some(t), Some(v), None) if !m.is_empty() && !t.is_empty() => (m, t, v),
        _ => return Err(Rejected(400, "the request line is not one")),
    };
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(Rejected(400, "only HTTP/1.1 is spoken"));
    }
    if !method.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(Rejected(400, "the method is not one"));
    }
    if !target.starts_with('/') {
        return Err(Rejected(400, "the target is not a path"));
    }
    let (raw_path, query) = target.split_once('?').unwrap_or((target, ""));
    let path = decode(raw_path, false).ok_or(Rejected(400, "the path is not valid"))?;

    let mut headers = Vec::new();
    for line in lines {
        if line.starts_with(' ') || line.starts_with('\t') {
            return Err(Rejected(400, "a folded header is not taken"));
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(Rejected(400, "a header has no colon"));
        };
        if name.is_empty() || name.bytes().any(|b| b <= b' ' || b == 0x7f) {
            return Err(Rejected(400, "a header name is not one"));
        }
        headers.push((name.to_ascii_lowercase(), value.trim().to_string()));
    }
    if headers.iter().any(|(n, _)| n == "transfer-encoding") {
        return Err(Rejected(501, "a chunked body is not taken"));
    }
    if headers.iter().filter(|(n, _)| n == "host").count() != 1 {
        return Err(Rejected(400, "a request has one Host"));
    }
    let lengths: Vec<_> = headers.iter().filter(|(n, _)| n == "content-length").collect();
    if lengths.len() > 1 {
        return Err(Rejected(400, "a request has one Content-Length"));
    }
    let length = match lengths.first() {
        Some((_, v)) => v.parse::<usize>().map_err(|_| Rejected(400, "the Content-Length is not a number"))?,
        None => 0,
    };
    if length > max_body {
        return Err(Rejected(413, "the body is too large"));
    }
    let mut body = buffer[end + 4..].to_vec();
    if body.len() > length {
        return Err(Rejected(400, "more was sent than the Content-Length says"));
    }
    while body.len() < length {
        let want = (length - body.len()).min(chunk.len());
        let n = stream.read(&mut chunk[..want]).map_err(|_| Rejected(400, "the body was not read"))?;
        if n == 0 {
            return Err(Rejected(400, "the body ended too soon"));
        }
        body.extend_from_slice(&chunk[..n]);
    }
    Ok(Request { method: method.to_string(), path, query: query.to_string(), headers, body })
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// A text with its percent-escapes decoded (and `+` a space in a query): None when the bytes are not UTF-8 or hold a NUL.
pub fn decode(text: &str, plus: bool) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hex = bytes.get(i + 1..i + 3)?;
                let value = u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
                out.push(value);
                i += 3;
            }
            b'+' if plus => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    let text = String::from_utf8(out).ok()?;
    (!text.contains('\0')).then_some(text)
}

/// A text as a part of a path: everything but the unreserved characters and `/` percent-encoded.
pub fn encode_path(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_escapes_decode_as_utf8_and_nothing_else() {
        assert_eq!(decode("a%20b%2Fc", false).as_deref(), Some("a b/c"));
        assert_eq!(decode("%E7%BD%91", false).as_deref(), Some("网"));
        assert_eq!(decode("a+b", true).as_deref(), Some("a b"));
        assert_eq!(decode("a+b", false).as_deref(), Some("a+b"));
        assert_eq!(decode("%zz", false), None);
        assert_eq!(decode("%2", false), None);
        assert_eq!(decode("%FF", false), None, "not UTF-8");
        assert_eq!(decode("a%00b", false), None, "a NUL");
    }

    #[test]
    fn a_path_is_encoded_so_that_it_decodes_to_itself() {
        for path in ["/home/u/a b/网表.sp", "/x/%41", "/a+b/c?d#e"] {
            assert_eq!(decode(&encode_path(path), false).as_deref(), Some(path));
        }
        assert_eq!(encode_path("/a b"), "/a%20b");
    }

    #[test]
    fn cookies_and_parameters_are_found_by_name() {
        let r = Request {
            method: "GET".into(),
            path: "/".into(),
            query: "dir=%2Fhome%2Fu&x=1&flag".into(),
            headers: vec![("cookie".into(), "a=1; rnl_session=abc; b=2".into())],
            body: Vec::new(),
        };
        assert_eq!(r.cookie("rnl_session"), Some("abc"));
        assert_eq!(r.cookie("c"), None);
        assert_eq!(r.param("dir").as_deref(), Some("/home/u"));
        assert_eq!(r.param("flag").as_deref(), Some(""));
        assert_eq!(r.param("nope"), None);
    }
}
