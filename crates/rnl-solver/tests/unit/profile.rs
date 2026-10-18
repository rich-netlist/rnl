//! Opt-in, test-only timing. Nested sections overlap and must not be added together.
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::Instant;

thread_local! {
    static TIMINGS: RefCell<Option<BTreeMap<String, (u64, f64)>>> = const { RefCell::new(None) };
    static ORIENTATIONS: RefCell<Vec<serde_json::Value>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn oracle_enabled() -> bool {
    std::env::var_os("RNL_PROFILE_ORACLE").is_some()
}
pub(crate) fn orientations(
    members: &[(usize, rnl_runtime::geom::Orient)],
    values: &[rnl_runtime::geom::Orient],
    scores: &[Vec<crate::rational::Q>],
    rank: &[usize],
) {
    if TIMINGS.with(|t| t.borrow().is_none()) {
        return;
    }
    ORIENTATIONS.with(|events|events.borrow_mut().push(serde_json::json!({"members":members.iter().map(|(k,o)|serde_json::json!({"object":k,"label":format!("{o:?}")})).collect::<Vec<_>>(),"values":values.iter().map(|o|format!("{o:?}")).collect::<Vec<_>>(),"scores":scores.iter().map(|xs|xs.iter().map(crate::rational::exact_text).collect::<Vec<_>>()).collect::<Vec<_>>(),"rank":rank,"retained":rank.iter().take(2).copied().collect::<Vec<_>>()})));
}

pub(crate) struct Section<'a> {
    name: &'a str,
    start: Option<Instant>,
}

pub(crate) fn section(name: &str) -> Section<'_> {
    let enabled = TIMINGS.with(|t| t.borrow().is_some());
    Section { name, start: enabled.then(Instant::now) }
}

impl Drop for Section<'_> {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            let seconds = start.elapsed().as_secs_f64();
            TIMINGS.with(|t| {
                if let Some(values) = t.borrow_mut().as_mut() {
                    let entry = values.entry(self.name.to_string()).or_default();
                    entry.0 += 1;
                    entry.1 += seconds;
                }
            });
        }
    }
}

pub(crate) fn reset() {
    TIMINGS.with(|t| *t.borrow_mut() = Some(BTreeMap::new()));
    ORIENTATIONS.with(|t| t.borrow_mut().clear());
}

pub(crate) fn take() -> serde_json::Value {
    let values = TIMINGS.with(|t| t.borrow_mut().take().unwrap());
    serde_json::json!(values
        .into_iter()
        .map(|(name, (calls, seconds))| (name, serde_json::json!({"calls":calls,"seconds":seconds})))
        .collect::<BTreeMap<_, _>>())
}

fn digest(bytes: &[u8]) -> String {
    rnl_runtime::numbering::sha256(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Mirror only the production driver's conversion of its chosen geometry to the writer input.
/// Comparing the complete writer output below catches drift in this test-only conversion.
fn chosen_solution(
    problem: &rnl_runtime::problem::LevelProblem,
    s: &crate::structure::Structure,
    placed: &crate::coords::Placed,
    routing: &crate::route::Routing,
) -> rnl_runtime::solution::Solution {
    use crate::rational::to_f64;
    use rnl_runtime::problem::Position;
    use rnl_runtime::solution::{Placement, Solution};
    let mut placements = Vec::new();
    for (k, object) in problem.objects.iter().enumerate() {
        let orient = object.orient.is_some().then(|| placed.geometry.orient[k]);
        match &object.position {
            Position::Own { .. } if crate::driver::left_free(s, k) => {}
            Position::Own { .. } => {
                let [x, y] = &placed.geometry.origin[k];
                placements.push(Placement { object: k, x: Some(to_f64(x)), y: Some(to_f64(y)), orient });
            }
            Position::At(_) if orient.is_some() => {
                placements.push(Placement { object: k, x: None, y: None, orient });
            }
            Position::InSymbol { replaced: Some(_), .. } => {
                let [x, y] = &placed.origins[k];
                placements.push(Placement { object: k, x: Some(to_f64(x)), y: Some(to_f64(y)), orient: None });
            }
            Position::At(_) | Position::InSymbol { .. } => {}
        }
    }
    Solution { placements, branches: routing.branches.clone(), wires: routing.wires.clone(), wire_routes: Vec::new() }
}

#[test]
#[ignore = "requires explicit input/config/output paths for isolated profiling"]
fn isolated_stage_profile() {
    use crate::score::{drawn, layers, order, Entry};
    struct Files;
    impl rnl_runtime::source::Provider for Files {
        fn read(&self, path: &str) -> Option<String> {
            std::fs::read_to_string(path).ok()
        }
    }
    let input = std::env::var("RNL_PROFILE_INPUT").unwrap();
    let output = std::env::var("RNL_PROFILE_OUTPUT").unwrap();
    let config = std::env::var("RNL_PROFILE_CONFIG").unwrap();
    let config_text = std::fs::read_to_string(&config).unwrap();
    let mut options = crate::SolverOptions::from_toml(&config_text).unwrap();
    options.time_limit = None;
    let cli_report = std::env::var("RNL_PROFILE_CLI_REPORT").ok().map(|value| value.parse::<bool>().unwrap());
    if let Some(report) = cli_report {
        // rnl-cli main::solve sets this host option from --report after reading TOML.
        // It controls additional low-priority conflict-group enumeration, never a work offset.
        options.hints_for_low = report;
    }
    let runtime = rnl_runtime::Options::solver();
    let original = rnl_view::interpret(&input, None, &Files, &runtime).unwrap();
    let original_bytes = std::fs::read(&input).unwrap();
    let original_text = String::from_utf8(original_bytes.clone()).unwrap();
    let canonical = options.canonical();
    let first_problems: Vec<_> = original
        .problems
        .iter()
        .map(|p| {
            serde_json::json!({
                "level":p.level,"title":p.title,"version":p.version,
                "hash":rnl_runtime::problem::sig3(p,&canonical),
            })
        })
        .collect();
    let target_title = std::env::var("RNL_PROFILE_LEVEL").ok();
    if let Some(title) = &target_title {
        assert_eq!(
            original.problems.iter().filter(|p| !p.objects.is_empty() && p.title == *title).count(),
            1,
            "requested profile level must identify exactly one original nonempty level"
        );
    }
    let mut memory = rnl_runtime::source::MemoryProvider::new();
    let mut raised_text = None;
    let raise_file_level = std::env::var("RNL_PROFILE_RAISE_FILE_LEVEL").ok().map(|v| v.parse::<usize>().unwrap());
    if let Some(level) = raise_file_level {
        // A narrow T14 final-pass reproducer, not a replacement for CLI wants discovery. The
        // invocation must identify the sole self-contained file level that the saved real CLI
        // report confirms was written and raised. Do not infer wants from a solved record.
        let nonempty: Vec<_> = original.problems.iter().filter(|p| !p.objects.is_empty()).collect();
        assert!(!nonempty.is_empty());
        if target_title.is_none() {
            assert_eq!(nonempty.len(), 1);
        }
        // Multiple target levels are allowed only inside this same confirmed self-contained
        // file. Only the explicitly selected level will be solved/profiled below.
        assert!(nonempty.iter().all(|p| original.doc.file_level(p.level) == level && p.version == "0.2"));
        assert_eq!(
            rnl_runtime::source::normalize(&original.doc.levels[level].path),
            rnl_runtime::source::normalize(&input)
        );
        assert!(original.doc.levels[level].section.is_none());
        assert_eq!(original.doc.levels[level].kind, rnl_runtime::host::LevelKind::File);
        let pin = !rnl_runtime::writeback::pins_std(&original.doc, level);
        let (text, done) = rnl_runtime::writeback::raise_version(&original.doc, &original_text, &[(level, pin)]);
        assert_eq!(done, vec![true], "the confirmed CLI wants file must be raised");
        assert_ne!(text, original_text);
        std::fs::write(format!("{output}.raised-input.sp"), &text).unwrap();
        memory.insert(&input, &text);
        raised_text = Some(text);
    }
    let files = rnl_runtime::source::Layered { layers: vec![&memory, &Files] };
    let document = rnl_view::interpret(&input, None, &files, &runtime).unwrap();
    assert!(!document.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error));
    let pass_binding = serde_json::json!({
        "source_revision":std::env::var("RNL_PROFILE_SOURCE").ok(),
        "profile_source_sha256":digest(include_bytes!("profile.rs")),
        "original_input":input,"original_raw_sha256":digest(&original_bytes),
        "original_decoded_sha256":digest(original_text.as_bytes()),
        "raised_overlay_sha256":raised_text.as_ref().map(|t|digest(t.as_bytes())),
        "raised_overlay_path":raised_text.as_ref().map(|_|format!("{output}.raised-input.sp")),
        "confirmed_cli_wants_file_level":raise_file_level,
        "first_pass_problems":first_problems,"first_solve_reexecuted":false,
        "profiled_pass":if raised_text.is_some(){"final raised pass"}else{"single input pass"},
        "config_input":config,"config_sha256":digest(config_text.as_bytes()),
        "effective_canonical_config":canonical,"time_limit":null,"caps":"options::CAPS",
        "cli_report_flag":cli_report,"effective_hints_for_low":options.hints_for_low,
        "target_title":target_title,"other_levels_reexecuted":false,
        "saved_other_level_records_used_only_as_output_carrier":target_title.is_some(),
    });
    let mut reports = Vec::new();
    for problem in document
        .problems
        .iter()
        .filter(|p| !p.objects.is_empty() && target_title.as_ref().is_none_or(|title| p.title == *title))
    {
        let total = Instant::now();
        let mut stages = BTreeMap::new();
        let start = Instant::now();
        let resolved = crate::resolve(problem, &options);
        stages.insert("resolve", start.elapsed().as_secs_f64());
        let mut work = resolved.work;
        let start = Instant::now();
        let structure = crate::structure::analyse(problem, &resolved, &options, crate::options::CAPS, &mut work);
        stages.insert("analyse", start.elapsed().as_secs_f64());
        reset();
        let start = Instant::now();
        let candidates = crate::search::search(problem, &structure, &options, &mut work);
        stages.insert("search", start.elapsed().as_secs_f64());
        let search_sections = take();
        let orientation_trace = ORIENTATIONS.with(|t| t.borrow_mut().drain(..).collect::<Vec<_>>());
        println!(
            "stage-profile {} search_seconds={:.6} candidates={}",
            problem.title,
            stages["search"],
            candidates.len()
        );
        let mut placing = Vec::new();
        let mut coords_seconds = 0.0;
        let mut estimate_seconds = 0.0;
        for candidate in candidates {
            let start = Instant::now();
            let placed = crate::coords::place(problem, &structure, &candidate, &options, &mut work);
            coords_seconds += start.elapsed().as_secs_f64();
            let start = Instant::now();
            let entry = Entry {
                layers: layers(&structure, &placed.origins, &placed.geometry, None),
                code: candidate.code(&structure),
            };
            estimate_seconds += start.elapsed().as_secs_f64();
            placing.push((candidate, placed, entry));
        }
        stages.insert("coords", coords_seconds);
        stages.insert("placed_estimates", estimate_seconds);
        let estimates: Vec<_> = placing.iter().map(|(_, _, e)| e.clone()).collect();
        let ranked = order(&structure.active, &estimates);
        let mut finals = Vec::new();
        let mut route_seconds = 0.0;
        let mut score_seconds = 0.0;
        for &i in ranked.iter().take(options.route_beam.max(1)) {
            let (candidate, placed, _) = &placing[i];
            let start = Instant::now();
            let routing =
                crate::route::route(problem, &structure, &placed.geometry, &placed.origins, &options, &mut work);
            let seconds = start.elapsed().as_secs_f64();
            route_seconds += seconds;
            let start = Instant::now();
            let wires = drawn(problem, &structure, &placed.geometry, &routing);
            let entry = Entry {
                layers: layers(&structure, &placed.origins, &placed.geometry, Some(&wires)),
                code: candidate.code(&structure),
            };
            score_seconds += start.elapsed().as_secs_f64();
            println!(
                "stage-profile routed candidate={i} origin={} route_seconds={seconds:.6} expansions={}",
                candidate.origin, routing.expansions
            );
            finals.push((i, routing, entry));
        }
        stages.insert("route", route_seconds);
        stages.insert("final_score", score_seconds);
        let entries: Vec<_> = finals.iter().map(|(_, _, e)| e.clone()).collect();
        let best = order(&structure.active, &entries)[0];
        let comparison = std::env::var("RNL_PROFILE_EXPECTED").ok().map(|expected| {
            // With no explicit target retain the original single-level whole-file proof.
            // A target-level proof never claims to have reexecuted the other levels.
            if target_title.is_none() {
                assert_eq!(document.problems.iter().filter(|p| !p.objects.is_empty()).count(), 1);
            }
            let hash = rnl_runtime::problem::sig3(problem, &canonical);
            let solution = chosen_solution(problem, &structure, &placing[finals[best].0].1, &finals[best].1);
            let written = rnl_runtime::solution::write_record(problem, &solution, crate::AUTHOR, &hash);
            assert!(written.unsolved.is_empty(), "chosen decisions must all be representable");
            let record = rnl_view::finish_record(&document.doc, &files, &runtime, problem, &written.record);
            let expected_bytes = std::fs::read(&expected).unwrap();
            let with = rnl_view::interpret(&expected, None, &Files, &rnl_runtime::Options { problems:true,..Default::default() }).unwrap();
            assert!(!with.diagnostics().iter().any(|d|d.severity==rnl_runtime::diag::Severity::Error));
            // This carrier is used strictly after search and routing have completed from the
            // original/raised entry. For a selected sublevel it preserves the saved records of
            // other levels, and replaces only this actual target record. Equality proves this
            // target's entire identity-bearing output; it is not a rerun of those other levels.
            let actual_text = if target_title.is_some() {
                let carrier = std::str::from_utf8(&expected_bytes).unwrap();
                rnl_runtime::writeback::replace_generated_with_header(&with.doc, problem.level, carrier, &record, &problem.version)
            } else {
                rnl_runtime::writeback::replace_generated_with_header(&document.doc, problem.level,
                    raised_text.as_deref().unwrap_or(&original_text), &record, &problem.version)
            };
            let previous = rnl_runtime::previous::previous(problem, &with).expect("saved CLI record maps to final problem");
            assert_eq!(previous.hash.as_deref(),Some(hash.as_str()),"final problem/config hash differs from T14");
            let expected_records: Vec<_> = with.doc.levels[problem.level].records.iter().filter(|r|
                rnl_runtime::writeback::record_type(r)=="generated" && rnl_runtime::writeback::field(r,"author").as_deref()==Some(crate::AUTHOR)
            ).collect();
            assert_eq!(expected_records.len(),1);
            let expected_record = &expected_records[0].text;
            // Compare the same host-read representation on both sides. SPICE removes '*' but
            // retains its following carrier space on continuation lines, so bare writer text
            // is not byte-identical to a parsed Record.text even for an identical entire file.
            // Keep the independent whole-file byte assertion below; do not trim either record.
            let actual_memory = rnl_runtime::source::MemoryProvider::new().with(&input, &actual_text);
            let actual_files = rnl_runtime::source::Layered { layers: vec![&actual_memory, &Files] };
            let actual_with = rnl_view::interpret(&input, None, &actual_files, &rnl_runtime::Options { problems:true,..Default::default() }).unwrap();
            assert!(!actual_with.diagnostics().iter().any(|d|d.severity==rnl_runtime::diag::Severity::Error));
            let actual_records: Vec<_> = actual_with.doc.levels[problem.level].records.iter().filter(|r|
                rnl_runtime::writeback::record_type(r)=="generated" && rnl_runtime::writeback::field(r,"author").as_deref()==Some(crate::AUTHOR)
            ).collect();
            assert_eq!(actual_records.len(),1);
            let actual_record = &actual_records[0].text;
            let wires = drawn(problem,&structure,&placing[finals[best].0].1.geometry,&finals[best].1);
            let scoring = structure.active.report(&crate::objectives::View {s:&structure,at:&placing[finals[best].0].1.origins,orient:&placing[finals[best].0].1.geometry.orient},Some(&wires),&options);
            let report_path=std::env::var("RNL_PROFILE_EXPECTED_REPORT").expect("matching saved CLI report required");
            let report_text=std::fs::read_to_string(&report_path).unwrap();
            let lines: Vec<_> = report_text.lines().collect();
            let header = format!("level {} (", problem.title);
            let starts: Vec<_> = lines.iter().enumerate().filter_map(|(i,line)|line.starts_with(&header).then_some(i)).collect();
            assert_eq!(starts.len(), 1, "saved CLI report must identify the exact target level");
            let start = starts[0];
            let end = (start+1..lines.len()).find(|&i|lines[i].starts_with("level ")).unwrap_or(lines.len());
            let target_report = &lines[start..end];
            assert!(target_report.iter().any(|line| *line == format!("  hash: {hash}")), "target report hash mismatch");
            let scores:Vec<serde_json::Value>=target_report.iter().filter_map(|l|l.strip_prefix("  scoring: ")).map(|l|serde_json::from_str(l).unwrap()).collect();
            assert_eq!(scores.len(),1);
            let work_line=format!("  work: {} resolve nodes, {} pivots, {} candidates, {} A* expansions",work.resolve_nodes,work.pivots,work.candidates,work.astar_expansions);
            let binding=serde_json::json!({
                "expected":expected,"expected_report":report_path,"problem_hash":hash,
                "full_file_bytes_equal":actual_text.as_bytes()==expected_bytes,
                "record_bytes_equal":actual_record==expected_record,
                "scoring_equal":serde_json::to_value(&scoring).unwrap()==scores[0],
                "work_equal":target_report.iter().any(|line|*line==work_line),
                "target_title":problem.title,"target_level":problem.level,
                "whole_file_comparison_scope":if target_title.is_some(){"only-target-record-replaced-in-saved-output-carrier"}else{"all-nonempty-levels-reexecuted"},
                "other_levels_reexecuted":false,
                "actual_file_sha256":digest(actual_text.as_bytes()),"expected_file_sha256":digest(&expected_bytes),
                "actual_record_sha256":digest(actual_record.as_bytes()),"expected_record_sha256":digest(expected_record.as_bytes()),
                "bare_writer_record_sha256":digest(record.as_bytes()),
                "scoring":scoring,"work_line":work_line,
            });
            std::fs::write(format!("{output}.chosen-record.rnl"),&record).unwrap();
            std::fs::write(format!("{output}.chosen-host-record.rnl"),actual_record).unwrap();
            std::fs::write(format!("{output}.chosen-file.sp"),&actual_text).unwrap();
            std::fs::write(format!("{output}.binding.json"),serde_json::to_vec_pretty(&binding).unwrap()).unwrap();
            assert_eq!(binding["record_bytes_equal"],true,"chosen complete identity-bearing record differs from T14");
            assert_eq!(binding["full_file_bytes_equal"],true,"chosen file bytes differ from T14");
            assert_eq!(binding["scoring_equal"],true,"chosen full scoring report differs from T14");
            assert_eq!(binding["work_equal"],true,"final-pass work differs from T14");
            binding
        });
        let exact_layers = |xs: &[crate::rational::Q]| xs.iter().map(crate::rational::exact_text).collect::<Vec<_>>();
        let estimated = ranked.iter().map(|&i| serde_json::json!({
            "candidate":i,"origin":placing[i].0.origin,"code":placing[i].2.code,"layers":exact_layers(&placing[i].2.layers),
            "rows":placing[i].0.rows,
            "placed_origins":placing[i].1.origins.iter().map(|p|exact_layers(p)).collect::<Vec<_>>(),
            "placed_orientations":placing[i].1.geometry.orient.iter().map(|o|format!("{o:?}")).collect::<Vec<_>>(),
            "coordinate_lp_optimised":placing[i].1.optimised,"placement_undecided":placing[i].1.undecided,
            "dropped_own_relations":format!("{:?}",placing[i].1.dropped),
        })).collect::<Vec<_>>();
        let routed = finals.iter().map(|(i,r,e)| serde_json::json!({
            "candidate":i,"origin":placing[*i].0.origin,"code":e.code,"layers":exact_layers(&e.layers),
            "expansions":r.expansions,"identity_geometry":format!("{:?}",r.geometry),
            "writer_wires":format!("{:?}",r.wires),"writer_branches":format!("{:?}",r.branches),
            "fallbacks":r.fallbacks.iter().map(|f|serde_json::json!({"route":f.route,"net":f.net,"phase":f.phase,"reason":f.reason})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>();
        let old = std::env::var("RNL_QUALITY_OLD").ok().map(|old|{
            use rnl_runtime::problem::{Axis,FactKind,PointRef};
            let with=rnl_view::interpret(&old,None,&Files,&rnl_runtime::Options{problems:true,..Default::default()}).unwrap();
            let previous=rnl_runtime::previous::previous(problem,&with).expect("old record maps to current problem");
            let mut pinned=problem.clone();
            let original=pinned.facts.len();
            for p in &previous.placements {
                for (axis,value) in [(Axis::X,p.x),(Axis::Y,p.y)]{if let Some(value)=value{pinned.facts.push(crate::resolve::tests::fact(FactKind::Place{at:PointRef::Origin(p.object),axis,value}));}}
                if let Some(code)=p.orient{pinned.facts.push(crate::resolve::tests::fact(FactKind::Orient{object:p.object,code}));}
            }
            let checked=crate::resolve(&pinned,&options);
            assert_eq!(checked.kept.iter().copied().filter(|&f|f<original).collect::<Vec<_>>(),resolved.kept,"old record changed authored fact retention");
            assert!((original..pinned.facts.len()).all(|f|checked.kept.contains(&f)),"old record contradicts retained facts");
            serde_json::json!({"input":old,"original_kept":resolved.kept,"added_frame_facts":pinned.facts.len()-original,"all_frame_facts_kept":true,"orientations":previous.placements.iter().filter_map(|p|Some(serde_json::json!({"object":p.object,"label":problem.objects[p.object].label,"orientation":format!("{:?}",p.orient?)}))).collect::<Vec<_>>()})
        });
        reports.push(serde_json::json!({"title":problem.title,"input":input,"pass_binding":pass_binding,"t14_comparison":comparison,"total_seconds":total.elapsed().as_secs_f64(),"stage_seconds":stages,"search_sections_nested":search_sections,"orientation_trace":orientation_trace,"old_record":old,"estimated_rank":estimated,"routed":routed,"chosen_candidate":finals[best].0,"work":{"resolve_nodes":work.resolve_nodes,"pivots":work.pivots,"candidates":work.candidates,"astar_expansions":work.astar_expansions}}));
    }
    std::fs::write(output, serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
}
