//! Showing the page: which browser, and how.
//!
//! The page needs a browser of 2021 or later (top-level await, module Workers for the core, `Object.hasOwn`): Firefox 115 ESR
//! or newer, Chrome, Chromium or Edge 89 or newer. An older one is never opened onto a page that would stay blank; the
//! program says what to do instead. A browser of Chromium's family is opened as an application window (`--app`), with a
//! profile of its own; Firefox in a profile of its own, `--no-remote`, so that the user's own profile is never touched.
//! The address, with its one-time token, is in a file of the user's own (mode 0600, in a new folder of mode 0700 that no one
//! else could have made first) that the browser is told to open, never on the command line, where `ps` would show it to the
//! other users of the machine. Each run has a browser profile of its own, which is removed when the run ends.

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
        Kind::Firefox => {
            format!("Firefox {}", browser.major.map_or("(version unknown)".to_string(), |v| v.to_string()))
        }
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
    let mut child = Command::new(path)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
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
        let path = if name.contains('/') || name.contains('\\') {
            PathBuf::from(name)
        } else {
            on_path(name).ok_or_else(|| format!("{name} is not on the PATH"))?
        };
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

/// What a fresh Firefox profile is told, so that a profile made for one run starts on the page and on nothing else: no welcome
/// page, no question about being the default browser, and no update of a Firefox that comes with the program.
const FIREFOX_PREFS: &str = "\
user_pref(\"browser.shell.checkDefaultBrowser\", false);
user_pref(\"browser.startup.homepage_override.mstone\", \"ignore\");
user_pref(\"startup.homepage_welcome_url\", \"\");
user_pref(\"browser.aboutwelcome.enabled\", false);
user_pref(\"datareporting.policy.firstRunURL\", \"\");
user_pref(\"toolkit.telemetry.reportingpolicy.firstRun\", false);
user_pref(\"app.update.auto\", false);
user_pref(\"app.update.enabled\", false);
user_pref(\"browser.tabs.warnOnClose\", false);
";

/// Start the browser on `url`, with `profile`, a folder for this run alone: a profile that other runs share is one that is
/// locked by the one before it (Firefox then asks, in a window, whether to close it, and the page is never shown).
pub fn open(browser: &Browser, url: &str, profile: &Path) -> Result<Child, String> {
    if browser.kind == Kind::Firefox {
        std::fs::write(profile.join("user.js"), FIREFOX_PREFS).map_err(|e| format!("{}: {e}", profile.display()))?;
    }
    // (what the browser says is kept in a file of the profile, for the user to be told if it ends at once: a Firefox that lacks
    // a library of the system says so there, and nowhere else)
    let log = std::fs::File::create(profile.join(LOG)).map_err(|e| format!("{}: {e}", profile.display()))?;
    let log_again = log.try_clone().map_err(|e| e.to_string())?;
    Command::new(&browser.path)
        .args(arguments(browser, url, profile))
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(log_again)
        .spawn()
        .map_err(|e| format!("{} could not be started: {e}", browser.path.display()))
}

/// The name of the file of the profile that holds what the browser said.
pub const LOG: &str = "rnl-browser.log";

/// The words for a browser that ended as soon as it was started, which is not how one shows a page: what it said last, and what
/// to do (a Firefox that does not start on a system that has none of the libraries of a desktop is the usual cause).
pub fn ended_at_once(browser: &Browser, status: &std::process::ExitStatus, profile: &Path) -> String {
    let said = std::fs::read_to_string(profile.join(LOG)).unwrap_or_default();
    let tail: Vec<&str> = said.lines().filter(|l| !l.trim().is_empty()).collect();
    let tail = tail[tail.len().saturating_sub(8)..].join("\n  ");
    format!(
        "{} ended as soon as it was started ({status}), and showed nothing.{}\n\
         A browser that does not start on a computer with no desktop usually lacks libraries (Firefox needs GTK 3 and some X libraries: \
         `{} --version` and the same without `--headless` tell which). Install them, or start the program with --no-browser, forward its port with\n\
         ssh -L PORT:127.0.0.1:PORT THIS-HOST, and open the address it prints in a browser on your own computer.",
        browser.path.display(),
        if tail.is_empty() { String::new() } else { format!("\n  It said:\n  {tail}") },
        browser.path.display()
    )
}

/// The folder the program keeps its own files in (the browser profiles): `$XDG_CACHE_HOME/rnl-viewer-local`, else
/// `~/.cache/rnl-viewer-local`, else a folder of the user's own number in the temporary folder.
pub fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| crate::files::home().map(|h| h.join(".cache")))
        .map(|p| p.join("rnl-viewer-local"))
        .unwrap_or_else(|| {
            std::env::temp_dir()
                .join(format!("rnl-viewer-local-{}", euid().map_or_else(|| "user".to_string(), |u| u.to_string())))
        })
}

/// The user this process runs as (its effective user id): on Linux, what `/proc` says; else, and if `/proc` cannot be read, the
/// owner of a file this process has just made, which is the user the system makes files for.
fn euid() -> Option<u32> {
    static EUID: std::sync::OnceLock<Option<u32>> = std::sync::OnceLock::new();
    *EUID.get_or_init(|| {
        let from_proc = std::fs::read_to_string("/proc/self/status").ok().and_then(|status| {
            status.lines().find_map(|l| l.strip_prefix("Uid:")).and_then(|v| v.split_whitespace().nth(1)?.parse().ok())
        });
        from_proc.or_else(owner_of_a_new_file)
    })
}

#[cfg(unix)]
fn owner_of_a_new_file() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    let path = std::env::temp_dir().join(format!(
        "rnl-viewer-uid-{}-{}",
        std::process::id(),
        &crate::security::random_hex()[..16]
    ));
    let file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path).ok()?;
    let uid = file.metadata().ok().map(|m| m.uid());
    drop(file);
    let _ = std::fs::remove_file(&path);
    uid
}

#[cfg(not(unix))]
fn owner_of_a_new_file() -> Option<u32> {
    None
}

/// Whether a folder (not a link to one) belongs to this user. Where the user cannot be told, nothing is: the program keeps its
/// files in a folder it knows to be its user's, or not at all.
fn mine(meta: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        euid().is_some_and(|u| meta.uid() == u)
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        true
    }
}

/// Make a folder of this user's alone: created if it is not there (with what is above it), mode 0700 always, and refused
/// if it is somebody else's, which a folder in a shared place (a temporary folder, a cache that was shared out) may be.
pub fn private_dir(dir: &Path) -> Result<(), String> {
    let say = |e: std::io::Error| format!("{}: {e}", dir.display());
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir).map_err(say)?;
        // (not `metadata`, which follows a link: a link in the place of the folder is somebody's choice, not the user's)
        let meta = std::fs::symlink_metadata(dir).map_err(say)?;
        if !meta.is_dir() || !mine(&meta) {
            return Err(format!("{} is not a folder of yours: the program keeps nothing in it", dir.display()));
        }
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(say)?;
    }
    #[cfg(not(unix))]
    std::fs::create_dir_all(dir).map_err(say)?;
    Ok(())
}

/// A new folder below `parent`, of this run: a name nobody could have made before (the process and 64 random bits),
/// created by the call that fails if it is there, readable by this user alone, and checked to be a folder of its own, not
/// a link that somebody put there.
fn fresh_dir(parent: &Path, prefix: &str) -> Result<PathBuf, String> {
    for _ in 0..8 {
        let path = parent.join(format!("{prefix}-{}-{}", std::process::id(), &crate::security::random_hex()[..16]));
        let made = {
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                std::fs::DirBuilder::new().mode(0o700).create(&path)
            }
            #[cfg(not(unix))]
            {
                std::fs::create_dir(&path)
            }
        };
        match made {
            Ok(()) => {
                let meta = std::fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                if !meta.is_dir() || !mine(&meta) {
                    let _ = std::fs::remove_dir(&path);
                    return Err(format!("{} is not a folder of yours", path.display()));
                }
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
    }
    Err(format!("no new folder could be made in {}", parent.display()))
}

/// This computer's name, for the names of folders: letters, digits and dots, whatever else made an underscore (never a
/// hyphen, which separates the parts of the name).
fn host_name() -> String {
    let name = ["/proc/sys/kernel/hostname", "/etc/hostname"]
        .iter()
        .find_map(|f| std::fs::read_to_string(f).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "host".to_string());
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' { c } else { '_' }).collect()
}

/// The folder of a browser profile for this run, below the cache folder, which is made the user's own. Its name has this
/// computer's name and the process: the cache folder may be one home folder shared by several computers (a network file
/// system), where a process that is not here is not gone, only elsewhere. The folders of runs on this computer that ended
/// without removing theirs (Ctrl-C) are removed first.
pub fn profile_dir() -> Result<PathBuf, String> {
    let parent = cache_dir().join("profiles");
    private_dir(&parent)?;
    remove_stale(&parent, &host_name());
    fresh_dir(&parent, &format!("run-{}", host_name()))
}

/// Remove the `run-HOST-PID-…` folders of `parent` made on this `host` whose process is gone. Those of another computer are
/// never touched, nor are any where the system does not say which processes there are (no `/proc`).
fn remove_stale(parent: &Path, host: &str) {
    if !Path::new("/proc/self").exists() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(parent) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(rest) = name.strip_prefix("run-") else { continue };
        let mut parts = rest.rsplitn(3, '-');
        let (Some(_random), Some(pid), Some(from)) = (parts.next(), parts.next(), parts.next()) else { continue };
        let Ok(pid) = pid.parse::<u32>() else { continue };
        if from != host {
            continue;
        }
        let Ok(meta) = std::fs::symlink_metadata(entry.path()) else { continue };
        if meta.is_dir() && mine(&meta) && !Path::new(&format!("/proc/{pid}")).exists() {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Write the file the browser is told to open: a page that goes on to `url`, readable by this user alone. It is in a new
/// folder of its own (see `fresh_dir`) in `$XDG_RUNTIME_DIR` when there is one, else in the cache folder; the folder it is
/// in is not changed, only the new one is, so that nobody else's folder is made less than it was. The file is made by the
/// call that fails if it is there, and does not follow a link.
pub fn address_file(url: &str) -> Result<PathBuf, String> {
    let parent = match std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).filter(|p| p.is_absolute() && p.is_dir())
    {
        Some(runtime) => runtime,
        None => {
            let cache = cache_dir();
            private_dir(&cache)?;
            cache
        }
    };
    let dir = fresh_dir(&parent, "rnl-viewer")?;
    let path = dir.join("open.html");
    let page = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>RNL Viewer</title><meta http-equiv=\"refresh\" content=\"0;url={url}\"><p><a href=\"{url}\">Open RNL Viewer</a></p>\n"
    );
    let write = || -> std::io::Result<()> {
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(&path)?.write_all(page.as_bytes())
    };
    if let Err(e) = write() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(format!("{}: {e}", path.display()));
    }
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

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rnl-local-launch-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    fn mode(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn a_folder_of_a_run_is_new_private_and_never_the_same_twice() {
        let parent = scratch("fresh");
        let one = fresh_dir(&parent, "run").unwrap();
        let two = fresh_dir(&parent, "run").unwrap();
        assert_ne!(one, two);
        assert!(one.starts_with(&parent) && one.is_dir());
        assert!(one.file_name().unwrap().to_string_lossy().starts_with(&format!("run-{}-", std::process::id())));
        #[cfg(unix)]
        {
            assert_eq!(mode(&one), 0o700);
            // the folder it is made in is left as it was
            assert_ne!(mode(&parent), 0o700);
        }
    }

    #[cfg(unix)]
    #[test]
    fn the_file_with_the_address_is_private_in_a_folder_of_its_own_and_does_not_change_the_one_above() {
        let runtime = scratch("runtime");
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&runtime, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // (the environment is the process's: this is the only test that sets it)
        std::env::set_var("XDG_RUNTIME_DIR", &runtime);
        let file = address_file("http://127.0.0.1:1/__start?t=abc").unwrap();
        std::env::remove_var("XDG_RUNTIME_DIR");
        assert_eq!(mode(&file), 0o600);
        let folder = file.parent().unwrap();
        assert_eq!(mode(folder), 0o700);
        assert_eq!(folder.parent().unwrap(), runtime);
        assert_eq!(mode(&runtime), 0o755, "the runtime folder is not changed: it is not ours to change");
        assert!(std::fs::read_to_string(&file).unwrap().contains("__start?t=abc"));
        // another run is another folder, and a file that is there is never written over
        std::env::set_var("XDG_RUNTIME_DIR", &runtime);
        let other = address_file("http://127.0.0.1:1/__start?t=def").unwrap();
        std::env::remove_var("XDG_RUNTIME_DIR");
        assert_ne!(other.parent(), file.parent());
    }

    #[cfg(unix)]
    #[test]
    fn a_private_folder_is_made_private_each_time_and_a_link_in_its_place_is_not_followed_into() {
        let parent = scratch("private");
        let cache = parent.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&cache, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        private_dir(&cache).unwrap();
        assert_eq!(mode(&cache), 0o700);
        let deep = parent.join("a/b/c");
        private_dir(&deep).unwrap();
        assert_eq!(mode(&deep), 0o700);
        // a file in the place of the folder is no folder
        std::fs::write(parent.join("file"), "x").unwrap();
        assert!(private_dir(&parent.join("file")).is_err());
    }

    #[test]
    fn the_profiles_of_runs_that_ended_are_removed_and_those_of_a_live_one_are_not() {
        let parent = scratch("stale");
        let host = host_name();
        let live = parent.join(format!("run-{host}-{}-aaaa", std::process::id()));
        // (a process number no system has: above the largest it gives)
        let gone = parent.join(format!("run-{host}-4294967290-bbbb"));
        // the same number on another computer that shares the folder: a process that is there, not here
        let elsewhere = parent.join("run-another-computer-4294967290-cccc");
        let foreign = parent.join("keep-me");
        for dir in [&live, &gone, &elsewhere, &foreign] {
            std::fs::create_dir_all(dir.join("inside")).unwrap();
        }
        remove_stale(&parent, &host);
        assert!(live.exists() && foreign.exists() && elsewhere.exists(), "only this computer's own are removed");
        if Path::new("/proc/self").exists() {
            assert!(!gone.exists());
        } else {
            assert!(gone.exists(), "where the processes are not known, nothing is removed");
        }
        assert!(!host.contains('-') && !host.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_browser_that_ends_at_once_is_told_with_what_it_said() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("ended");
        let script = dir.join("firefox-that-lacks-a-library");
        std::fs::write(&script, "#!/bin/sh\necho 'XPCOMGlueLoad error for file libxul.so: libX11-xcb.so.1: cannot open shared object file' >&2\nexit 127\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let profile = dir.join("profile");
        std::fs::create_dir_all(&profile).unwrap();
        let browser = Browser { path: script, kind: Kind::Other, major: None };
        let mut child = open(&browser, "file:///x.html", &profile).unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success());
        let text = ended_at_once(&browser, &status, &profile);
        assert!(
            text.contains("ended as soon as it was started")
                && text.contains("libX11-xcb.so.1")
                && text.contains("--no-browser")
                && text.contains("ssh -L"),
            "{text}"
        );
    }

    #[test]
    fn a_path_is_an_address_that_a_browser_opens() {
        assert_eq!(file_url(Path::new("/run/user/1000/a b.html")), "file:///run/user/1000/a%20b.html");
    }
}
