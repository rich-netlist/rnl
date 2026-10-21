//! One speculative compiler per window, independent of the foreground core's job queue.
//! Cancellation discards the result; a compiler already interpreting can finish at low priority.
use std::cell::Cell;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use rnl_cli::FsProvider;
use rnl_runtime::source::{FilePool, LibraryLookup, Provider};
use tauri::State;

use crate::{build_cache::Prepared, core_thread::Core, library::Library, settings::Settings};

#[derive(Default)]
pub struct Control {
    generation: AtomicU64,
    working: AtomicBool,
}
impl Control {
    pub fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
    fn current(&self, generation: u64) -> bool {
        self.generation.load(Ordering::SeqCst) == generation
    }
    fn ticket(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
    fn start(self: &Arc<Self>, ticket: u64) -> Result<Running, &'static str> {
        if !self.current(ticket) {
            return Err("cancelled");
        }
        self.working.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).map_err(|_| "busy")?;
        Ok(Running { control: self.clone(), generation: ticket })
    }
}
struct Running {
    control: Arc<Control>,
    generation: u64,
}
impl Drop for Running {
    fn drop(&mut self) {
        self.control.working.store(false, Ordering::SeqCst);
    }
}

#[tauri::command]
pub fn cancel_preload(core: State<'_, Core>) {
    core.preload.cancel();
}

/// Obtain before sending the async compilation IPC. A late-starting async command must keep
/// this ticket, rather than accidentally adopting a newer generation after cancellation.
#[tauri::command]
pub fn preload_ticket(core: State<'_, Core>) -> u64 {
    core.preload.ticket()
}

/// A read quota bounds speculative input, not arbitrary compiler working memory. Oversized
/// entries still open normally in the foreground. Failed/partial quota builds never enter cache.
struct Limited<'a> {
    files: &'a dyn Provider,
    running: &'a Running,
    remaining: Cell<usize>,
    exceeded: Cell<bool>,
}
impl Provider for Limited<'_> {
    fn read(&self, path: &str) -> Option<String> {
        if !self.running.control.current(self.running.generation) {
            return None;
        }
        let file = std::fs::File::open(path).ok()?;
        let metadata = file.metadata().ok()?;
        if !metadata.is_file() {
            return None;
        }
        if metadata.len() > self.remaining.get() as u64 {
            self.exceeded.set(true);
            return None;
        }
        // Take also bounds a file that grows after metadata was read. Decode only the bounded
        // bytes, then check UTF-8 expansion of Latin-1 against the same quota.
        let Some(text) = read_bounded(file, self.remaining.get()).ok()? else {
            self.exceeded.set(true);
            return None;
        };
        self.remaining.set(self.remaining.get() - text.len());
        Some(text)
    }
    fn package_files(&self) -> Vec<String> {
        self.files.package_files()
    }
    fn package_candidates(&self, name: &str) -> Vec<String> {
        self.files.package_candidates(name)
    }
    fn include_candidates(&self, name: &str) -> Vec<String> {
        self.files.include_candidates(name)
    }
    fn auto_include(&self) -> Option<&dyn LibraryLookup> {
        self.files.auto_include()
    }
}

fn read_bounded(reader: impl Read, limit: usize) -> std::io::Result<Option<String>> {
    let mut bytes = Vec::new();
    reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Ok(None);
    }
    let text = crate::files::decode(&bytes);
    Ok((text.len() <= limit).then_some(text))
}

#[tauri::command]
pub async fn preload_document(
    core: State<'_, Core>,
    settings: State<'_, Settings>,
    library: State<'_, Library>,
    path: String,
    pool: Vec<String>,
    auto_include: bool,
    ticket: u64,
) -> Result<String, String> {
    if !settings.preload_enabled()? {
        return Ok("disabled".into());
    }
    let running = match core.preload.start(ticket) {
        Ok(running) => running,
        Err(status) => return Ok(status.into()),
    };
    let core = core.inner().clone();
    let paths = settings.paths()?;
    let limit = settings.cache_limit()?;
    let library = library.snapshot()?;
    tauri::async_runtime::spawn_blocking(move || {
        if !enough_memory() {
            return Ok("paused".into());
        }
        let entry = std::path::absolute(&path).map_err(|e| e.to_string())?.to_string_lossy().into_owned();
        let key = entry.clone();
        // A speculative snapshot must never restore an older global cache budget.
        let (cached, room) = core.run(move |cache| (cache.contains(&key, auto_include), cache.room().min(limit)))?;
        if !running.control.current(running.generation) {
            return Ok("cancelled".into());
        }
        if cached {
            return Ok("cached".into());
        }
        if room < 1024 * 1024 {
            return Ok("full".into());
        }
        let priority = BackgroundPriority::new();
        let disk = FsProvider::new(&paths).with_search_paths(&paths);
        let disk = FilePool { files: &disk, paths: &pool };
        let held = rnl_view::MemoryProvider::new();
        let files = crate::files::layered(&held, &disk);
        let files = rnl_view::LibraryProvider { files: &files, index: &library, enabled: auto_include };
        let limited = Limited {
            files: &files,
            running: &running,
            remaining: Cell::new((room / 8).min(8 * 1024 * 1024)),
            exceeded: Cell::new(false),
        };
        let prepared = Prepared::build(&entry, &limited);
        let exceeded = limited.exceeded.get();
        drop(priority); // Restore the async runtime's pool thread before it accepts another job.
        if !running.control.current(running.generation) {
            return Ok("cancelled".into());
        }
        if exceeded || prepared.is_err() {
            return Ok("skipped".into());
        }
        if !enough_memory() {
            return Ok("paused".into());
        }
        core.run(move |cache| {
            if !running.control.current(running.generation) {
                return "cancelled".into();
            }
            if cache.offer(prepared.unwrap()) {
                "ready".into()
            } else {
                "full".into()
            }
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(windows)]
fn enough_memory() -> bool {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut memory: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    memory.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    unsafe { GlobalMemoryStatusEx(&mut memory) != 0 && memory.ullAvailPhys >= 1024 * 1024 * 1024 }
}
#[cfg(not(windows))]
fn enough_memory() -> bool {
    true
}

struct BackgroundPriority {
    #[cfg(windows)]
    active: bool,
}
impl BackgroundPriority {
    fn new() -> Self {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Threading::{
                GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN,
            };
            Self { active: unsafe { SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN) != 0 } }
        }
        #[cfg(not(windows))]
        Self {}
    }
}
impl Drop for BackgroundPriority {
    fn drop(&mut self) {
        #[cfg(windows)]
        if self.active {
            use windows_sys::Win32::System::Threading::{
                GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_END,
            };
            unsafe {
                SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_END);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disk_reads_stop_at_quota_even_if_the_file_grew() {
        let mut reader = std::io::Cursor::new(vec![b'a'; 100_000]);
        assert!(read_bounded(&mut reader, 10).unwrap().is_none());
        assert_eq!(reader.position(), 11);
        assert!(read_bounded(std::io::Cursor::new([0xff; 10]), 10).unwrap().is_none());
        assert_eq!(read_bounded(std::io::Cursor::new(b"\xef\xbb\xbfhello"), 8).unwrap().unwrap(), "hello");
    }
    #[test]
    fn single_compiler_and_generation_cancellation() {
        let control = Arc::new(Control::default());
        let ticket = control.ticket();
        let first = control.start(ticket).unwrap();
        assert!(matches!(control.start(ticket), Err("busy")));
        control.cancel();
        assert!(!control.current(first.generation));
        assert!(matches!(control.start(control.ticket()), Err("busy"))); // no concurrent compilers
        drop(first);
        assert!(matches!(control.start(ticket), Err("cancelled"))); // an old IPC starts late
        assert!(control.start(control.ticket()).is_ok());
    }
}
