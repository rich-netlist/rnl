//! A build takes a reading of the document made with options that read it alike
//! (`Options::reads_as`) instead of reading it again, and only such a one: the tool's settings for
//! the solver change no reading but `junction` (13.2). The Document is the one reading again gives.

use std::cell::Cell;
use std::collections::BTreeMap;

use rnl_runtime::solver_settings::parse_overrides;
use rnl_runtime::source::{MemoryProvider, Provider};
use rnl_runtime::Options;
use rnl_view::{build, build_taking, read_noting, BuildOptions};

/// The files, counting how often a reading looks for the one that is not there.
struct Counting {
    files: MemoryProvider,
    absent: Cell<usize>,
}

impl Provider for Counting {
    fn read(&self, path: &str) -> Option<String> {
        if path.ends_with("absent.inc") {
            self.absent.set(self.absent.get() + 1);
        }
        self.files.read(path)
    }

    fn path_key(&self, path: &str) -> String {
        self.files.path_key(path)
    }
}

const NET: &str = "* taking\n\
    * @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }\n\
    .include \"absent.inc\"\nR1 a b 1k\nR2 b 0 1k\n\
    * @RNL { #R1 @PLACE(0, 0) }\n.end\n";

#[test]
fn a_build_takes_a_reading_whose_options_read_alike_and_only_that() {
    let files = Counting { files: MemoryProvider::new().with("/w/t.sp", NET), absent: Cell::new(0) };
    let build_options = BuildOptions::default();
    let again = build("/w/t.sp", &files, &build_options).unwrap();
    let json = serde_json::to_string(&again).unwrap();
    assert!(again.missing.iter().any(|m| m.ends_with("absent.inc")), "{:?}", again.missing);
    // the build's reading with a tool setting: one the solver alone takes, and `junction`
    for (setting, taken) in [("reroute_nets=true", true), ("junction=\"$JUNCTION\"", false)] {
        let tool = Options { solver_config: parse_overrides(setting).unwrap(), ..build_options.runtime.clone() };
        assert_eq!(build_options.runtime.reads_as(&tool), taken, "{setting}");
        let reading = read_noting("/w/t.sp", None, &BTreeMap::new(), &files, &tool).unwrap();
        files.absent.set(0);
        let built = build_taking("/w/t.sp", &files, &build_options, &[&reading]).unwrap();
        assert_eq!(files.absent.get() == 0, taken, "{setting}: read again only when not taken");
        assert_eq!(serde_json::to_string(&built).unwrap(), json, "{setting}");
    }
}
