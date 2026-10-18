use rnl_solver::{exact_text, Exact, SolverOptions, DEFAULT_CONFIG};

#[test]
fn decimal_lexemes_remain_exact_through_the_effective_configuration() {
    let config = |n: &str| {
        SolverOptions::from_toml(&format!("[[level]]\ntolerance={n}\n[[level.term]]\nkind=\"wire_turns\"\nweight={n}"))
            .unwrap()
    };
    let a = config("0.1");
    assert_eq!(a.canonical(), config("0.10").canonical());
    assert_eq!(a.canonical(), config("1e-1").canonical());
    assert_ne!(a.canonical(), config("0.10000000000000001").canonical());
    assert_eq!(exact_text(&a.objectives.layers[0].items[0].weight), "1/10");
    assert_eq!(a.objectives.layers[0].tolerance, Exact::new(1.into(), 10.into()));
}

#[test]
fn the_builtin_and_shipped_configuration_are_the_same_source() {
    assert_eq!(SolverOptions::builtin(), SolverOptions::from_toml(DEFAULT_CONFIG).unwrap());
    assert_eq!(DEFAULT_CONFIG, include_str!("../../../config/rnl-solver.toml"));
}

#[test]
fn absent_levels_inherit_but_an_explicit_empty_list_disables_all_terms() {
    let partial = SolverOptions::from_toml("[rules]\nbeam=7").unwrap();
    assert_eq!(partial.beam, 7);
    assert_eq!(partial.route_beam, 7);
    assert_eq!(partial.objectives, SolverOptions::builtin().objectives);
    assert!(SolverOptions::from_toml("level=[]").unwrap().objectives.layers.is_empty());
    let disabled = "[[level]]\n[[level.term]]\nkind='wire_turns'\nenabled=false";
    assert!(SolverOptions::from_toml(disabled).unwrap().objectives.layers.is_empty());
}

#[test]
fn canonical_term_order_is_also_execution_order_and_distinct_parameters_coexist() {
    let a = "[[level]]\n[[level.term]]\nkind='current_direction'\ndirection='up'\n[[level.term]]\nkind='current_direction'\ndirection='down'";
    let b = a.replace("'up'", "'PLACEHOLDER'").replace("'down'", "'up'").replace("'PLACEHOLDER'", "'down'");
    let a = SolverOptions::from_toml(a).unwrap();
    let b = SolverOptions::from_toml(&b).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.canonical(), b.canonical());
    let duplicate =
        "[[level]]\n[[level.term]]\nkind='symbol_alignment'\n[[level.term]]\nkind='symbol_alignment'\nscope='group'";
    assert!(SolverOptions::from_toml(duplicate).unwrap_err().message.contains("identical parameters"));
}

#[test]
fn integer_classification_does_not_depend_on_fractional_weight() {
    let valid = "[[level]]\n[[level.term]]\nkind='wire_turns'\nweight=0.5\n[[level.term]]\nkind='symbol_overlaps'";
    assert!(SolverOptions::from_toml(valid).is_ok());
    let invalid = valid.replace("wire_turns", "area");
    let error = SolverOptions::from_toml(&invalid).unwrap_err();
    assert!(error.message.contains("non-integer"));
    assert!(error.line > 0 && error.column > 0);
}

#[test]
fn invalid_rules_and_terms_report_source_lines() {
    for (text, line) in [
        ("[rules]\ngrid=0", 2),
        ("[rules]\ntolerance=0", 2),
        ("[rules]\ntexts_at_anchor=false", 2),
        ("[rules]\nbeam=-1", 2),
        ("[rules]\nroute_clearance=-0.1", 2),
        ("[[level]]\n[[level.term]]\nkind='symbol_symmetry'", 3),
        ("[[level]]\n[[level.term]]\nkind='wire_turns'\nweight=nan", 4),
        ("[[level]]\n[[level.term]]\nkind='current_direction'\ndirection='diagonal'", 4),
    ] {
        let error = SolverOptions::from_toml(text).unwrap_err();
        assert_eq!(error.line, line, "{text}: {error}");
        assert!(error.to_string().contains("line "));
    }
    assert!(SolverOptions::from_toml("format='unknown'").is_err());
    assert!(SolverOptions::from_toml("[rules]\nunrecognized=1").is_err());
    assert!(SolverOptions::from_toml("[[level]]\n[[level.term]]\nkind='area'\ninclude_wires=true").is_err());
}

#[test]
fn scalar_overrides_cannot_restore_the_old_objective_configuration() {
    assert!(SolverOptions::builtin().with_overrides("objectives=none").is_err());
    let options = SolverOptions::builtin()
        .with_overrides("grid=none;separate_symbols=false;budget=astar_expansions:0,soft_expansions:0")
        .unwrap();
    assert_eq!(options.grid, None);
    assert!(!options.separate_symbols);
    assert_eq!(options.budget.soft_expansions, 0);
}
