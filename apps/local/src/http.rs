//! The small piece of HTTP/1.1 the page needs, over `std::net`, with no crate: one request and one response on a
//! connection, which is then closed. The server is for one user's browser on the loopback address, so what it takes
//! is strict and small: a request line and headers of at most `MAX_HEAD` bytes, a body with a `Content-Length` of at
//! most `max_body` bytes, and nothing else (no chunked bodies, no upgrades, no pipelining).

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

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

/// What writes the body of a streamed response, to the connection, after its head: a solve tells its events as it goes.
type Stream = Box<dyn FnOnce(&mut TcpStream) -> io::Result<()>>;

/// A response: its status, headers and body, or, for a [`Response::streamed`] one, what writes the body as it is made.
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    streamed: std::cell::RefCell<Option<Stream>>,
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response").field("status", &self.status).field("headers", &self.headers).field("body", &self.body.len()).finish()
    }
}

impl Response {
    pub fn new(status: u16, content_type: &str, body: impl Into<Vec<u8>>) -> Response {
        Response {
            status,
            headers: vec![("Content-Type".into(), content_type.into())],
            body: body.into(),
            streamed: std::cell::RefCell::new(None),
        }
    }

    /// A 200 whose body is not known when its head is written: `write` writes it to the connection, which the end of ends
    /// (no `Content-Length`; the connection is closed after every response).
    pub fn streamed(content_type: &str, write: impl FnOnce(&mut TcpStream) -> io::Result<()> + 'static) -> Response {
        let response = Response::new(200, content_type, Vec::new());
        *response.streamed.borrow_mut() = Some(Box::new(write));
        response
    }

    /// Whether the body is written as it is made.
    fn is_streamed(&self) -> bool {
        self.streamed.borrow().is_some()
    }

    /// Write the body of a streamed response, if it is one, once.
    fn stream_to(&self, stream: &mut TcpStream) -> io::Result<()> {
        match self.streamed.borrow_mut().take() {
            Some(write) => write(stream),
            None => Ok(()),
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
        408 => "Request Timeout",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "Status",
    }
}

/// The head of a response. Every response closes the connection and says it is never cached or sniffed.
fn head_of(response: &Response) -> String {
    let mut head = format!("HTTP/1.1 {} {}\r\n", response.status, reason(response.status));
    for (name, value) in &response.headers {
        // (a header never holds a line end: the values are the program's own, but a path may reach one)
        if name.contains(['\r', '\n']) || value.contains(['\r', '\n']) {
            continue;
        }
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    // (a streamed body is as long as it is: the end of the connection is its end)
    if !response.is_streamed() {
        head.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    }
    head.push_str("Connection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\n");
    // (no page of another site may frame the program's: it would draw over what the user sees)
    head.push_str("Content-Security-Policy: frame-ancestors 'none'\r\nX-Frame-Options: DENY\r\n\r\n");
    head
}

/// Write a response and flush it. Each write may take as long as the stream's write timeout.
pub fn write_response(stream: &mut TcpStream, response: &Response, head_only: bool) -> io::Result<()> {
    stream.write_all(head_of(response).as_bytes())?;
    stream.flush()?;
    if !head_only {
        stream.write_all(&response.body)?;
        response.stream_to(stream)?;
    }
    stream.flush()
}

/// Write a response and flush it, all of it in `total` at most: for a client that is not of the session, whose answer is
/// not worth a place for as long as a client likes to take. A reader that takes a byte now and then holds a place for as
/// long as the write timeout of each write, which a response of megabytes is made of many; this is for all of them.
pub fn write_response_within(stream: &mut TcpStream, response: &Response, head_only: bool, total: Duration) -> io::Result<()> {
    let deadline = Instant::now() + total;
    let mut send = |bytes: &[u8]| -> io::Result<()> {
        for piece in bytes.chunks(16 * 1024) {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "the response was taken too slowly"));
            }
            stream.set_write_timeout(Some(left))?;
            stream.write_all(piece)?;
        }
        Ok(())
    };
    send(head_of(response).as_bytes())?;
    if !head_only {
        send(&response.body)?;
    }
    // (a streamed body is of the session's own requests alone, which are not written within a time: it is not written here)
    stream.flush()
}

/// How long a request's head may take to arrive, all of it: a client that sends a byte at a time, to keep a connection (and
/// one of the program's few places for them) for as long as it can, is not served.
pub const HEAD_TIME: Duration = Duration::from_secs(10);

/// How fast a body must come, which is not how soon it must be done: a save or a solve of hundreds of megabytes over an `ssh -L`
/// is slow and no fault. A read that has nothing for `idle` ends it, and in each `window` at least `min_rate` bytes a second
/// must have come on average; a client that sends a byte now and then, to keep a place, has neither.
#[derive(Clone, Copy, Debug)]
pub struct Pace {
    pub idle: Duration,
    pub window: Duration,
    pub min_rate: u64,
}

/// What the program asks of the body of a request that is of the session.
pub const BODY_PACE: Pace = Pace { idle: Duration::from_secs(30), window: Duration::from_secs(30), min_rate: 64 * 1024 };

/// A request's head, read: the request without its body, the length of the body it says it has, and the first bytes of that
/// body, which came with the head.
#[derive(Debug)]
pub struct Head {
    pub request: Request,
    pub length: usize,
    early: Vec<u8>,
}

/// Read some bytes of the stream, no later than `deadline`.
fn read_some(stream: &mut TcpStream, buffer: &mut [u8], deadline: Instant, what: &'static str) -> Result<usize, Rejected> {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        return Err(Rejected(408, "the request came too slowly"));
    }
    stream.set_read_timeout(Some(left)).map_err(|_| Rejected(400, what))?;
    match stream.read(buffer) {
        Ok(n) => Ok(n),
        Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => Err(Rejected(408, "the request came too slowly")),
        Err(_) => Err(Rejected(400, what)),
    }
}

/// Read the head of a request from the stream, in `HEAD_TIME` at most. What the body is to be is for the caller to say, and
/// the body is not read: a request that is not let in is answered without it.
pub fn read_head(stream: &mut TcpStream) -> Result<Head, Rejected> {
    read_head_within(stream, HEAD_TIME)
}

/// Read the head of a request from the stream, in `time` at most.
pub fn read_head_within(stream: &mut TcpStream, time: Duration) -> Result<Head, Rejected> {
    let deadline = Instant::now() + time;
    let mut buffer = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    let end = loop {
        if let Some(at) = find(&buffer, b"\r\n\r\n") {
            break at;
        }
        if buffer.len() > MAX_HEAD {
            return Err(Rejected(431, "the headers are too long"));
        }
        let n = read_some(stream, &mut chunk, deadline, "the request was not read")?;
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
    let early = buffer[end + 4..].to_vec();
    if early.len() > length {
        return Err(Rejected(400, "more was sent than the Content-Length says"));
    }
    Ok(Head { request: Request { method: method.to_string(), path, query: query.to_string(), headers, body: Vec::new() }, length, early })
}

/// Read the body the head says it has, at the pace `BODY_PACE` asks, and give the request whole.
pub fn read_body(stream: &mut TcpStream, head: Head) -> Result<Request, Rejected> {
    read_body_paced(stream, head, BODY_PACE)
}

/// Read the body the head says it has, at the pace given.
pub fn read_body_paced(stream: &mut TcpStream, head: Head, pace: Pace) -> Result<Request, Rejected> {
    let Head { mut request, length, early } = head;
    let mut body = early;
    body.reserve(length - body.len());
    let mut chunk = [0u8; 16 * 1024];
    let (mut window_start, mut window_from) = (Instant::now(), body.len());
    while body.len() < length {
        let want = (length - body.len()).min(chunk.len());
        let n = read_some(stream, &mut chunk[..want], Instant::now() + pace.idle, "the body was not read")?;
        if n == 0 {
            return Err(Rejected(400, "the body ended too soon"));
        }
        body.extend_from_slice(&chunk[..n]);
        let seconds = window_start.elapsed();
        if seconds >= pace.window {
            if ((body.len() - window_from) as f64) < pace.min_rate as f64 * seconds.as_secs_f64() {
                return Err(Rejected(408, "the body came too slowly"));
            }
            (window_start, window_from) = (Instant::now(), body.len());
        }
    }
    request.body = body;
    Ok(request)
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

    /// A client and the server's end of its connection.
    fn pair() -> (TcpStream, TcpStream) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        (client, server)
    }

    #[test]
    fn a_head_that_comes_a_byte_at_a_time_is_cut_off_as_a_whole_not_read_by_read() {
        let (mut client, mut server) = pair();
        let slow = std::thread::spawn(move || {
            // each byte comes well inside any time for a single read, and the head never ends
            for b in b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nX-A: ".iter().cycle().take(200) {
                if client.write_all(&[*b]).is_err() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let started = Instant::now();
        let Err(Rejected(status, _)) = read_head_within(&mut server, Duration::from_millis(400)) else { panic!("the head was taken") };
        assert_eq!(status, 408);
        assert!(started.elapsed() < Duration::from_secs(2), "{:?}", started.elapsed());
        drop(server);
        slow.join().unwrap();
    }

    #[test]
    fn a_body_that_comes_slowly_is_taken_while_it_comes_and_not_while_it_stops_or_trickles() {
        let pace = Pace { idle: Duration::from_millis(300), window: Duration::from_millis(200), min_rate: 20_000 };
        let head = |client: &mut TcpStream, server: &mut TcpStream, length: usize| {
            client.write_all(format!("POST /x HTTP/1.1\r\nHost: h\r\nContent-Length: {length}\r\n\r\n").as_bytes()).unwrap();
            read_head(server).unwrap()
        };
        // a body of 100 KB that takes a second (100 KB/s: slow, and far above the least): taken, though it takes longer than a fixed time would give
        let (mut client, mut server) = pair();
        let h = head(&mut client, &mut server, 100_000);
        let sender = std::thread::spawn(move || {
            for _ in 0..10 {
                client.write_all(&[7u8; 10_000]).unwrap();
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        assert_eq!(read_body_paced(&mut server, h, pace).unwrap().body.len(), 100_000);
        sender.join().unwrap();
        // a byte every 50 ms (20 B/s): it is not a body that is coming
        let (mut client, mut server) = pair();
        let h = head(&mut client, &mut server, 100_000);
        let sender = std::thread::spawn(move || {
            for _ in 0..100 {
                if client.write_all(&[7u8]).is_err() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        });
        let started = Instant::now();
        assert!(matches!(read_body_paced(&mut server, h, pace), Err(Rejected(408, _))));
        assert!(started.elapsed() < Duration::from_secs(2));
        drop(server);
        sender.join().unwrap();
        // nothing at all for longer than the idle time
        let (mut client, mut server) = pair();
        let h = head(&mut client, &mut server, 10);
        assert!(matches!(read_body_paced(&mut server, h, pace), Err(Rejected(408, _))));
        drop(client);
    }

    #[test]
    fn a_response_that_is_not_read_is_given_up_whole_in_the_time_it_has() {
        let (client, mut server) = pair();
        let big = Response::new(200, "application/octet-stream", vec![0u8; 64 * 1024 * 1024]);
        let started = Instant::now();
        // (the client never reads: the buffers of the two ends fill, and what is left waits for a reader that is not there)
        assert!(write_response_within(&mut server, &big, false, Duration::from_millis(500)).is_err());
        assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
        drop(client);
        // one that is read is written whole
        let (mut client, mut server) = pair();
        let small = Response::text(200, "hello");
        write_response_within(&mut server, &small, false, Duration::from_secs(5)).unwrap();
        drop(server);
        let mut text = String::new();
        client.read_to_string(&mut text).unwrap();
        assert!(text.starts_with("HTTP/1.1 200 OK") && text.ends_with("hello"), "{text}");
    }

    #[test]
    fn a_streamed_response_has_no_length_and_its_body_is_written_after_its_head() {
        let (mut client, mut server) = pair();
        let streamed = Response::streamed("application/x-ndjson", |out| {
            out.write_all(b"{\"a\":1}\n")?;
            out.write_all(b"{\"b\":2}\n")
        });
        write_response(&mut server, &streamed, false).unwrap();
        drop(server);
        let mut text = String::new();
        client.read_to_string(&mut text).unwrap();
        let (head, body) = text.split_once("\r\n\r\n").unwrap();
        assert!(head.starts_with("HTTP/1.1 200 OK") && head.contains("application/x-ndjson") && !head.contains("Content-Length"), "{head}");
        assert_eq!(body, "{\"a\":1}\n{\"b\":2}\n");
    }

    #[test]
    fn the_body_is_read_after_the_head_and_only_what_the_head_says() {
        let (mut client, mut server) = pair();
        client.write_all(b"POST /x HTTP/1.1\r\nHost: h\r\nContent-Length: 5\r\n\r\nab").unwrap();
        let head = read_head(&mut server).unwrap();
        assert_eq!((head.length, head.request.body.len()), (5, 0), "the head is read without the body");
        client.write_all(b"cde").unwrap();
        let request = read_body(&mut server, head).unwrap();
        assert_eq!(request.body, b"abcde");
        // more than the head says is not taken
        let (mut client, mut server) = pair();
        client.write_all(b"POST /x HTTP/1.1\r\nHost: h\r\nContent-Length: 1\r\n\r\nabc").unwrap();
        assert!(matches!(read_head(&mut server), Err(Rejected(400, _))));
    }

    #[test]
    fn parameters_are_found_by_name() {
        let r = Request {
            method: "GET".into(),
            path: "/".into(),
            query: "dir=%2Fhome%2Fu&x=1&flag".into(),
            headers: vec![("x-a".into(), "1".into())],
            body: Vec::new(),
        };
        assert_eq!(r.header("x-a"), Some("1"));
        assert_eq!(r.param("dir").as_deref(), Some("/home/u"));
        assert_eq!(r.param("flag").as_deref(), Some(""));
        assert_eq!(r.param("nope"), None);
    }
}
