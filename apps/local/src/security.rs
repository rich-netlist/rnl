//! Who may talk to the program. It listens on the loopback address only, but that address is shared by everyone who
//! has an account on the machine, and by every page the user's browser has open, so a request is served only if:
//!
//! * its `Host` is this machine's own name for itself (`127.0.0.1`, `localhost`, `[::1]`), which keeps a page of another
//!   site, reaching the program through a DNS name it controls, out;
//! * it carries the session cookie, which the page gets once, by opening the address the program printed, whose token
//!   works one time and is never on a command line, where `ps` shows it to the other users;
//! * and a request that changes something also carries the header `X-RNL: 1`, which a page of another origin cannot
//!   add without a preflight that nothing here allows, comes from this origin, and is not cross-site.
//!
//! The cookie is `HttpOnly` and `SameSite=Strict`; neither is enough alone, since another local port is the same site.

use std::io::Read;
use std::sync::Mutex;

use crate::http::Request;

/// The name of the session cookie.
pub const COOKIE: &str = "rnl_session";

/// 256 bits from the system: `/dev/urandom`, else the standard library's own seed, which it takes from the system too.
pub fn random_hex() -> String {
    let mut bytes = [0u8; 32];
    let from_system = std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut bytes)).is_ok();
    if !from_system {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        for chunk in bytes.chunks_mut(8) {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
            chunk.copy_from_slice(&hasher.finish().to_le_bytes()[..chunk.len()]);
        }
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Whether two texts are the same, taking as long to say no as to say yes.
pub fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    diff == 0
}

/// Whether a `Host` header names this machine by its own loopback name, with any port: a port is what `ssh -L` changes.
pub fn host_ok(host: &str) -> bool {
    let (name, port) = match host.strip_prefix('[') {
        Some(rest) => match rest.split_once(']') {
            Some((name, tail)) => (format!("[{name}]"), tail.strip_prefix(':')),
            None => return false,
        },
        None => match host.split_once(':') {
            Some((name, port)) => (name.to_string(), Some(port)),
            None => (host.to_string(), None),
        },
    };
    let name_ok = matches!(name.to_ascii_lowercase().as_str(), "127.0.0.1" | "localhost" | "[::1]");
    let port_ok = port.is_none_or(|p| !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit()));
    name_ok && port_ok
}

/// The one-time token and the session cookie.
pub struct Auth {
    session: String,
    token: Mutex<Option<String>>,
}

impl Auth {
    /// A new session, and the token that opens it once.
    pub fn new() -> (Auth, String) {
        let token = random_hex();
        (Auth { session: random_hex(), token: Mutex::new(Some(token.clone())) }, token)
    }

    /// Exchange the token for the cookie: the token is spent whether it was right or not, so that it cannot be guessed at.
    pub fn exchange(&self, presented: &str) -> Option<String> {
        let mut token = self.token.lock().ok()?;
        let expected = token.take()?;
        same(&expected, presented).then(|| self.session.clone())
    }

    /// The `Set-Cookie` value of the session.
    pub fn cookie(&self) -> String {
        format!("{COOKIE}={}; HttpOnly; SameSite=Strict; Path=/", self.session)
    }

    /// Whether the request is of the session.
    pub fn signed_in(&self, request: &Request) -> bool {
        request.cookie(COOKIE).is_some_and(|c| same(c, &self.session))
    }
}

/// Whether a request that changes something comes from the page of this server: the custom header, and an origin that is the
/// server's own, when the browser says one, and a fetch that is not cross-site, when it says.
pub fn from_the_page(request: &Request) -> bool {
    if request.header("x-rnl") != Some("1") {
        return false;
    }
    let host = request.header("host").unwrap_or("");
    if let Some(origin) = request.header("origin") {
        if origin != format!("http://{host}") {
            return false;
        }
    }
    request.header("sec-fetch-site").is_none_or(|s| s == "same-origin")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(headers: &[(&str, &str)]) -> Request {
        Request {
            method: "POST".into(),
            path: "/api/x".into(),
            query: String::new(),
            headers: headers.iter().map(|(n, v)| (n.to_string(), v.to_string())).collect(),
            body: Vec::new(),
        }
    }

    #[test]
    fn only_the_machines_own_names_are_served_with_any_port() {
        for ok in ["127.0.0.1:8765", "localhost:1", "LOCALHOST:65535", "[::1]:8000", "127.0.0.1", "localhost"] {
            assert!(host_ok(ok), "{ok}");
        }
        for bad in ["example.com", "example.com:8765", "127.0.0.1.example.com:80", "evil.localhost.example:80", "127.0.0.1:", "127.0.0.1:80a", "[::1", "0.0.0.0:80", "localhost:123456", ""] {
            assert!(!host_ok(bad), "{bad}");
        }
    }

    #[test]
    fn a_token_opens_a_session_once_and_a_wrong_one_spends_it() {
        let (auth, token) = Auth::new();
        assert_eq!(token.len(), 64);
        assert!(auth.exchange("nope").is_none());
        assert!(auth.exchange(&token).is_none(), "spent by the wrong one");
        let (auth, token) = Auth::new();
        let cookie = auth.exchange(&token).unwrap();
        assert!(auth.exchange(&token).is_none(), "the second time");
        let signed = request(&[("cookie", &format!("{COOKIE}={cookie}"))]);
        assert!(auth.signed_in(&signed));
        assert!(!auth.signed_in(&request(&[("cookie", &format!("{COOKIE}=wrong"))])));
        assert!(!auth.signed_in(&request(&[])));
        assert!(auth.cookie().contains("HttpOnly") && auth.cookie().contains("SameSite=Strict"));
    }

    #[test]
    fn tokens_are_not_the_same_twice() {
        assert_ne!(random_hex(), random_hex());
        assert!(same("abc", "abc") && !same("abc", "abd") && !same("abc", "ab"));
    }

    #[test]
    fn a_change_must_come_from_the_page_of_this_server() {
        let page = [("host", "127.0.0.1:8765"), ("x-rnl", "1"), ("origin", "http://127.0.0.1:8765"), ("sec-fetch-site", "same-origin")];
        assert!(from_the_page(&request(&page)));
        // no header: a form of another page could send it
        assert!(!from_the_page(&request(&[("host", "127.0.0.1:8765")])));
        // another origin, though it knows the header
        assert!(!from_the_page(&request(&[("host", "127.0.0.1:8765"), ("x-rnl", "1"), ("origin", "http://127.0.0.1:9999")])));
        assert!(!from_the_page(&request(&[("host", "127.0.0.1:8765"), ("x-rnl", "1"), ("sec-fetch-site", "cross-site")])));
        // a client that is no browser says neither origin nor site
        assert!(from_the_page(&request(&[("host", "127.0.0.1:8765"), ("x-rnl", "1")])));
    }
}
