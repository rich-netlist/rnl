//! The thread that reads the documents. The reading the page shows keeps the drawings of the objects it lists without
//! a position ([`rnl_view::build_file_drawn`]) for the page to ask for as it scrolls; they hold what the model does.
//! Readings and requests for drawings run in order. Nothing else is kept: the page keeps the readings
//! (apps/viewer-ui/file-reading.js).

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use rnl_view::{Drawings, UnplacedDrawing};

/// The drawings of the reading shown, kept from the last reading that asked to keep them.
#[derive(Default)]
pub struct Drawn {
    active: Option<Drawings>,
}

impl Drawn {
    /// Keep `drawings`, in place of those kept before.
    pub fn keep(&mut self, drawings: Drawings) {
        self.active = Some(drawings);
    }

    /// The drawings of the objects at `items` of `levels[level].unplaced` of the reading kept, which `names` must name.
    pub fn unplaced(&self, level: usize, items: &[usize], names: &[String]) -> Result<Vec<UnplacedDrawing>, String> {
        self.active.as_ref().ok_or_else(|| "no document has kept drawings".to_string())?.unplaced(level, items, names)
    }
}

/// What the core thread runs, with the drawings of the reading shown.
type Job = Box<dyn FnOnce(&mut Drawn) + Send>;

/// The core thread, as a handle that can be cloned and sent to the threads of the commands.
#[derive(Clone)]
pub struct Core {
    jobs: Arc<Mutex<Sender<Job>>>,
}

impl Core {
    /// Start the thread; it ends when the last handle is dropped.
    pub fn new() -> Core {
        let (jobs, queue) = channel::<Job>();
        std::thread::Builder::new()
            .name("rnl-core".into())
            .spawn(move || {
                let mut drawn = Drawn::default();
                for job in queue {
                    job(&mut drawn);
                }
            })
            .expect("the core thread could not be started");
        Core { jobs: Arc::new(Mutex::new(jobs)) }
    }

    /// Run `job` in the core thread, with the drawings kept, and wait for its result: call it from a thread that may
    /// block, such as a command's `spawn_blocking`. A job that panics is an error, and the thread goes on.
    pub fn run<T: Send + 'static>(&self, job: impl FnOnce(&mut Drawn) -> T + Send + 'static) -> Result<T, String> {
        let (reply, result) = channel();
        let job: Job = Box::new(move |drawn| {
            let _ = reply.send(catch_unwind(AssertUnwindSafe(|| job(drawn))));
        });
        let stopped = || "the core stopped".to_string();
        self.jobs.lock().map_err(|_| stopped())?.send(job).map_err(|_| stopped())?;
        result.recv().map_err(|_| stopped())?.map_err(|_| "the core failed on this request".to_string())
    }
}

#[cfg(test)]
mod tests {
    use rnl_view::MemoryProvider;

    use super::*;

    const TEXT: &str = "* some left\nR1 a b 1k\nC1 b 0 1p\n* @RNL { #R1 @PLACE(0, 0) }\n.end\n";

    fn read(core: &Core) -> Vec<String> {
        core.run(|drawn| {
            let files = MemoryProvider::new().with("/a.sp", TEXT);
            let (result, drawings) = rnl_view::build_file_drawn("/a.sp", "a.sp", &files, &Default::default()).unwrap();
            drawn.keep(drawings);
            result.document.levels[0].unplaced.iter().map(|u| u.name.clone()).collect()
        })
        .unwrap()
    }

    #[test]
    fn the_drawings_of_a_reading_are_asked_for_from_another_thread() {
        let core = Core::new();
        let asked = |core: &Core, names: Vec<String>| {
            core.run(move |drawn| {
                let items: Vec<usize> = (0..names.len()).collect();
                drawn.unplaced(0, &items, &names)
            })
            .unwrap()
        };
        // none before a reading kept them
        assert!(asked(&core, vec![]).is_err());
        let names = read(&core);
        assert!(!names.is_empty());
        // asked for by a thread of another handle, as the commands do
        let other = core.clone();
        let (n, drawings) = std::thread::spawn(move || {
            let drawings = asked(&other, names.clone()).unwrap();
            (names, drawings)
        })
        .join()
        .unwrap();
        assert_eq!(drawings.len(), n.len());
        assert!(drawings.iter().all(|d| d.svg.is_some()));
        // names of another reading are refused
        let wrong = vec!["R9".to_string(); n.len()];
        let e = asked(&core, wrong).unwrap_err();
        assert!(e.contains("built again"), "{e}");
    }

    #[test]
    fn a_job_that_panics_is_an_error_and_the_thread_goes_on() {
        let core = Core::new();
        let e = core.run(|_| -> u8 { panic!("on purpose") }).unwrap_err();
        assert!(e.contains("failed"), "{e}");
        assert_eq!(core.run(|_| 7).unwrap(), 7);
    }
}
