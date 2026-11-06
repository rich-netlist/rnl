//! Showing the page: which browser, and how.
//!
//! The page needs a browser of 2021 or later (top-level await, module Workers for the core, `Object.hasOwn`): Firefox 115 ESR
//! or newer, Chrome, Chromium or Edge 89 or newer. An older one is never opened onto a page that would stay blank; the
//! program says what to do instead. A browser of Chromium's family is opened as an application window (`--app`), with a
//! profile of its own; Firefox in a profile of its own, `--no-remote`, so that the user's own profile is never touched.
//! The address, with its one-time token, is in a file of the user's own (mode 0600) that the browser is told to open,
//! never on the command line, where `ps` would show it to the other users of the machine.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Firefox,
    /// Chrome, Chromium, Edge, and what is built on them.
    Chromium,
    /// Named by the user with `--browser`: opened as a Chromium-family browser would be only if it says so; not checked.
    Other,
}

#[derive(Clone, Debug)]
pub struct Browser {
    pub path: PathBuf,
    pub kind: Kind,
    /// The major version, when the browser said one.
    pub major: Option<u32>,
}

/// The oldest major versions the page runs on.
pub const MIN_FIREFOX: u32 = 115;
pub const MIN_CHROMIUM: u32 = 89;

/// What kind of browser a program is, by its name.
pub fn kind_of(path: &Path) -> Kind {
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if name.contains("firefox") {
        Kind::Firefox
    } else if ["chrome", "chromium", "edge", "brave"].iter().any(|k| name.contains(k)) {
        Kind::Chromium
    } else {
        Kind::Other
    }
}

/// The major version in what `--version` printed: `Mozilla Firefox 115.9.0esr`, `Google Chrome 120.0.6099.109`.
pub fn parse_major(output: &str) -> Option<u32> {
    output.split_whitespace().find_map(|word| {
        let digits: String = word.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            return None;
        }
        let rest = &word[digits.len()..];
        (rest.is_empty() || rest.starts_with('.')).then(|| digits.parse().ok()).flatten()
    })
}

/// Whether the page runs on a browser of this kind and version: an unknown kind or version is let through, and said so.
pub fn acceptable(kind: Kind, major: Option<u32>) -> bool {
    match (kind, major) {
        (Kind::Firefox, Some(v)) => v >= MIN_FIREFOX,
        (Kind::Chromium, Some(v)) => v >= MIN_CHROMIUM,
        _ => true,
    }
}

/// The words for a browser that is too old: what it is, what the page needs, and the two ways out.
pub fn too_old(browser: &Browser) -> String {
    let what = match browser.kind {
        Kind::Firefox => format!("Firefox {}", browser.major.map_or("(version unknown)".to_string(), |v| v.to_string())),
        _ => format!("{} {}", browser.path.display(), browser.major.map_or(String::new(), |v| v.to_string())),
    };
    format!(
        "{what} is too old for RNL Viewer: the page needs Firefox {MIN_FIREFOX} ESR or newer, or Chrome, Chromium or Edge {MIN_CHROMIUM} or newer.\n\
         Either use the package of the program that comes with its own Firefox, or start the program with --no-browser, forward its port with\n\
         ssh -L PORT:127.0.0.1:PORT THIS-HOST, and open the address it prints in a browser on your own computer."
    )
}

/// Run `path --version`, at most five seconds: what it printed.
fn version_of(path: &Path) -> Option<String> {
    let mut child = Command::new(path).arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(_) => break,
            None if start.elapsed() > Duration::from_secs(5) => {
                let _ = child.kill();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    Some(out)
}

/// A program by name on the PATH.
fn on_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).map(|d| d.join(name)).find(|p| p.is_file())
}

/// The browser to show the page in: the one named (`--browser` or $RNL_BROWSER), else the Firefox that comes with the program
/// (`firefox/firefox` beside it), else the first of Chrome, Chromium, Edge and Firefox that is on the PATH and is new enough
/// (a newer one is preferred to an older, by the order of the list). None when there is none: `Err` says why.
pub fn find(named: Option<&str>, beside: Option<&Path>) -> Result<Browser, String> {
    let describe = |path: PathBuf| -> Browser {
        let major = version_of(&path).and_then(|o| parse_major(&o));
        Browser { kind: kind_of(&path), path, major }
    };
    if let Some(name) = named {
        let path = if name.contains('/') || name.contains('\\') { PathBuf::from(name) } else { on_path(name).ok_or_else(|| format!("{name} is not on the PATH"))? };
        if !path.is_file() {
            return Err(format!("{} is not a program", path.display()));
        }
        return Ok(describe(path));
    }
    if let Some(dir) = beside {
        let bundled = dir.join("firefox").join("firefox");
        if bundled.is_file() {
            return Ok(describe(bundled));
        }
    }
    let mut too_old_one: Option<Browser> = None;
    for name in ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "microsoft-edge", "firefox"] {
        if let Some(path) = on_path(name) {
            let browser = describe(path);
            if acceptable(browser.kind, browser.major) {
                return Ok(browser);
            }
            too_old_one.get_or_insert(browser);
        }
    }
    match too_old_one {
        Some(old) => Err(too_old(&old)),
        None => Err("no browser was found on the PATH (Chrome, Chromium, Edge or Firefox)".into()),
    }
}

/// The arguments that open `url` in the browser, with the profile folder it is to use.
pub fn arguments(browser: &Browser, url: &str, profile: &Path) -> Vec<String> {
    match browser.kind {
        Kind::Chromium => vec![
            format!("--app={url}"),
            format!("--user-data-dir={}", profile.display()),
            "--no-first-run".into(),
            "--no-default-browser-check".into(),
            "--window-size=1320,880".into(),
        ],
        Kind::Firefox => vec!["--no-remote".into(), "--profile".into(), profile.display().to_string(), url.into()],
        Kind::Other => vec![url.into()],
    }
}

/// Start the browser on `url`, with its profile below `profiles`.
pub fn open(browser: &Browser, url: &str, profiles: &Path) -> Result<Child, String> {
    let profile = profiles.join(match browser.kind {
        Kind::Firefox => "profile-firefox",
        _ => "profile-chromium",
    });
    std::fs::create_dir_all(&profile).map_err(|e| format!("{}: {e}", profile.display()))?;
    Command::new(&browser.path)
        .args(arguments(browser, url, &profile))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("{} could not be started: {e}", browser.path.display()))
}

/// The folder the program keeps its own files in (the browser profiles, the address file): `$XDG_CACHE_HOME/rnl-viewer-local`,
/// else `~/.cache/rnl-viewer-local`.
pub fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| crate::files::home().map(|h| h.join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("rnl-viewer-local")
}

/// Write the file the browser is told to open: a page that goes on to `url`, readable by this user alone. In
/// `$XDG_RUNTIME_DIR` when there is one, else the cache folder, whose mode is set to the user's alone as well.
pub fn address_file(url: &str) -> Result<PathBuf, String> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).filter(|p| p.is_dir()).unwrap_or_else(cache_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    let path = dir.join(format!("rnl-viewer-{}.html", std::process::id()));
    let page = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>RNL Viewer</title><meta http-equiv=\"refresh\" content=\"0;url={url}\"><p><a href=\"{url}\">Open RNL Viewer</a></p>\n"
    );
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        file.write_all(page.as_bytes()).map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    std::fs::write(&path, page).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// The `file://` address of a path.
pub fn file_url(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    format!("file://{}{}", if text.starts_with('/') { "" } else { "/" }, crate::http::encode_path(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_major_version_is_read_from_what_each_browser_prints() {
        assert_eq!(parse_major("Mozilla Firefox 115.9.0esr\n"), Some(115));
        assert_eq!(parse_major("Mozilla Firefox 68.5.0\n"), Some(68));
        assert_eq!(parse_major("Google Chrome 120.0.6099.109 \n"), Some(120));
        assert_eq!(parse_major("Chromium 91.0.4472.101 built on Debian\n"), Some(91));
        assert_eq!(parse_major("Microsoft Edge 120.0.2210.91\n"), Some(120));
        assert_eq!(parse_major("no number here"), None);
        assert_eq!(parse_major(""), None);
    }

    #[test]
    fn a_browser_below_what_the_page_needs_is_refused_and_an_unknown_one_is_let_through() {
        assert!(!acceptable(Kind::Firefox, Some(68)));
        assert!(!acceptable(Kind::Firefox, Some(102)));
        assert!(acceptable(Kind::Firefox, Some(115)));
        assert!(!acceptable(Kind::Chromium, Some(88)));
        assert!(acceptable(Kind::Chromium, Some(89)));
        assert!(acceptable(Kind::Firefox, None));
        assert!(acceptable(Kind::Other, Some(1)));
    }

    #[test]
    fn the_words_for_an_old_browser_name_both_ways_out() {
        let old = Browser { path: PathBuf::from("/usr/bin/firefox"), kind: Kind::Firefox, major: Some(68) };
        let text = too_old(&old);
        assert!(text.contains("Firefox 68 is too old") && text.contains("115 ESR"), "{text}");
        assert!(text.contains("its own Firefox") && text.contains("--no-browser") && text.contains("ssh -L"), "{text}");
    }

    #[test]
    fn each_kind_of_browser_gets_a_profile_of_its_own_and_the_address_last() {
        let url = "file:///run/user/1/rnl-viewer-1.html";
        let profile = Path::new("/home/u/.cache/rnl-viewer-local/profile-x");
        let firefox = Browser { path: "firefox".into(), kind: Kind::Firefox, major: Some(128) };
        let args = arguments(&firefox, url, profile);
        assert_eq!(args.first().map(String::as_str), Some("--no-remote"));
        assert_eq!(args.last().map(String::as_str), Some(url));
        assert!(args.iter().any(|a| a == "--profile") && args.iter().any(|a| a.ends_with("profile-x")));
        let chrome = Browser { path: "google-chrome".into(), kind: Kind::Chromium, major: Some(120) };
        let args = arguments(&chrome, url, profile);
        assert!(args.contains(&format!("--app={url}")) && args.iter().any(|a| a.starts_with("--user-data-dir=")));
        assert_eq!(kind_of(Path::new("/opt/firefox/firefox")), Kind::Firefox);
        assert_eq!(kind_of(Path::new("/usr/bin/google-chrome-stable")), Kind::Chromium);
        assert_eq!(kind_of(Path::new("/usr/bin/surf")), Kind::Other);
    }

    #[test]
    fn a_path_is_an_address_that_a_browser_opens() {
        assert_eq!(file_url(Path::new("/run/user/1000/a b.html")), "file:///run/user/1000/a%20b.html");
    }
}
