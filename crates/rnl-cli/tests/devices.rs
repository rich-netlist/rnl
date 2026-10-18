//! The symbols that the devices of a SPICE netlist get through the rule package of SPICE (16.3),
//! once the adapter has told their nodes from their models: a device of the P type is drawn as
//! one, whatever its model card is called and however many nodes the line has, and the lines of
//! HSPICE that are transmission lines are drawn as such. The same for a Spectre netlist and the
//! rule package of Spectre, also where it holds lines of SPICE.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(net: &str) -> Interpretation {
    read_as("/w/t.sp", net)
}

fn read_as(path: &str, net: &str) -> Interpretation {
    let files = MemoryProvider::new().with(path, net);
    interpret_file(Path::new(path), None, &files, &Options::default()).unwrap()
}

/// The symbol of each instance of the main level, by the name of the instance.
fn symbols(r: &Interpretation) -> Vec<(String, String)> {
    let level = r.model.levels.iter().find(|l| l.title == "t.sp" || l.title == "t.scs").unwrap();
    level
        .objects
        .iter()
        .filter(|o| o.kind == "instance")
        .map(|o| (o.id.trim_start_matches('#').to_string(), o.symbol.as_ref().map_or("-".into(), |s| s.name.clone())))
        .collect()
}

fn symbol(r: &Interpretation, name: &str) -> String {
    symbols(r).into_iter().find(|(n, _)| n == name).unwrap_or_else(|| panic!("no {name}")).1
}

fn messages(r: &Interpretation, severity: Severity) -> Vec<String> {
    r.diagnostics().iter().filter(|d| d.severity == severity).map(|d| r.sink.render(d)).collect()
}

#[test]
fn a_device_of_the_p_type_is_drawn_as_one_by_whatever_its_model_is_called() {
    let net = "* t\n\
        .model pmod_3t pmos level=72\n.model nsoi_5t nmos level=57\n.model psoi_bin.1 pmos level=57\n\
        .model pnp_bin.1 pnp\n.model vbic_pnp pnp level=4\n.model qlat lpnp\n.model mp vdmos pchan Vto=-1.5\n.model mn vdmos\n\
        .model pch.1 pmos\n.model nch.1 nmos\n.model npn_sub npn\n\
        M3p d g s pmod_3t w=1u\nM5n d g s e b nsoi_5t\nM5p d g s e b psoi_bin\nMpv d g s s mp\nMnv d g s mn\n\
        MP_bin d g s b pch\nMN_bin d g s b nch\nMP_bin1 d g s b pch.1\n\
        Q_lpnp c b e qlat\nQ5p c b e s dt vbic_pnp\nQ3area c b e pnp_bin 2.0\nQ4n c b e s npn_sub\n.end\n";
    let r = read(net);
    let want = [
        ("M3p", "$PMOS_4TERM"),
        ("M5n", "$NMOS_4TERM"),
        ("M5p", "$PMOS_4TERM"),
        ("Mpv", "$PMOS_4TERM"),
        ("Mnv", "$NMOS_4TERM"),
        ("MP_bin", "$PMOS_4TERM"),
        ("MN_bin", "$NMOS_4TERM"),
        ("MP_bin1", "$PMOS_4TERM"),
        ("Q_lpnp", "$PNP"),
        ("Q5p", "$PNP"),
        ("Q3area", "$PNP"),
        ("Q4n", "$NPN"),
    ];
    for (name, symbol_name) in want {
        assert_eq!(symbol(&r, name), symbol_name, "{name}");
    }
    assert!(messages(&r, Severity::Error).is_empty(), "{:?}", messages(&r, Severity::Error));
}

#[test]
fn the_lines_of_hspice_that_are_transmission_lines_are_drawn_as_such() {
    let net = "* t\nW1 a b Vc swm\nW2 in1 ref1 out1 ref2 RLGCMODEL=wmod N=1 L=10m\nU1 a b ref urc L=1u\nU2 in1 ref1 out1 ref2 umod L=10m\n\
               T1 a b c d Z0=50 TD=1n\n.end\n";
    let r = read(net);
    let got: Vec<String> = ["W1", "W2", "U1", "U2", "T1"].iter().map(|n| symbol(&r, n)).collect();
    assert_eq!(got, ["$SWITCH", "$TRANSMISSION_LINE", "$RC_LINE", "$TRANSMISSION_LINE", "$TRANSMISSION_LINE"]);
}

#[test]
fn the_terminals_of_w_t_o_and_u_are_named_as_6_12_says() {
    // W has p and n alone, T and O have p1 n1 p2 n2, U has a b ref
    let net = "* t\nW1 a b Vc swm\nT1 a b c d Z0=50\nO1 a b c d lmod\nU1 a b c urc L=1u\nVc c 0 1\n\
               * @RNL { &a {#W1:p #T1:p1 #O1:n2 #U1:ref} @WIRE }\n.end\n";
    let r = read(net);
    let e = messages(&r, Severity::Error);
    assert!(e.iter().all(|m| !m.contains("terminal")), "{e:?}");
    let net = "* t\nW1 a b Vc swm\nVc c 0 1\n* @RNL { &a {#W1:cp} @WIRE }\n.end\n";
    let e = messages(&read(net), Severity::Error);
    assert!(e.iter().any(|m| m.contains("#W1 has no terminal cp")), "{e:?}");
}

#[test]
fn a_value_written_with_a_capital_m_is_hinted_as_milli() {
    let r = read("* t\nR1 a b 10M\nR2 a b 10meg\n.end\n");
    let hints = messages(&r, Severity::Hint);
    let milli: Vec<&String> = hints.iter().filter(|m| m.contains("10M")).collect();
    assert_eq!(milli.len(), 1, "{hints:?}");
    assert!(milli[0].contains("t.sp:2") && milli[0].contains("milli in SPICE"), "{}", milli[0]);
}

#[test]
fn the_devices_of_a_spectre_netlist_are_drawn_by_their_model_whatever_its_family_or_spelling() {
    // `type = p` with spaces, a binned model by its full name and its base name, models of
    // families the table of 6.12 does not list yet, a transmission line, and a coupling of two
    // inductors, which is no instance
    let net = "// t\nsimulator lang=spectre\n\
        model pch_spaced bsim4 type = p\nmodel pch_bin.1 bsim4 type=p\nmodel n_utsoi utsoi type=n\n\
        model p_hisim hisim_hv type=p\nmodel d_500 dio500\nmodel z_n gaas type=n\nmodel z_p angelov type=p\n\
        MP1 (d g s b) pch_spaced w=1u\nMP2 (d g s b) pch_bin.1\nMP3 (d g s b) pch_bin\nMN1 (d g s b) n_utsoi\n\
        MP4 (d g s b) p_hisim\nD1 (a c) d_500\nZ1 (d g s) z_n\nZ2 (d g s) z_p\n\
        L1 (a b) inductor l=1n\nL2 (c d) inductor l=2n\nK1 mutual_inductor ind1=L1 ind2=L2 coupling=0.99\n\
        TL1 (a b c d) tline z0=50 td=500p\n";
    let r = read_as("/w/t.scs", net);
    let want = [
        ("MP1", "$PMOS_4TERM"),
        ("MP2", "$PMOS_4TERM"),
        ("MP3", "$PMOS_4TERM"),
        ("MN1", "$NMOS_4TERM"),
        ("MP4", "$PMOS_4TERM"),
        ("D1", "$DIODE"),
        ("Z1", "$NJFET"),
        ("Z2", "$PJFET"),
        ("L1", "$INDUCTOR"),
        ("TL1", "$TRANSMISSION_LINE"),
    ];
    for (name, symbol_name) in want {
        assert_eq!(symbol(&r, name), symbol_name, "{name}");
    }
    assert!(symbols(&r).iter().all(|(n, _)| n != "K1"), "{:?}", symbols(&r));
}

#[test]
fn the_spice_parts_of_a_spectre_file_are_drawn_and_share_their_model_cards_with_its_spectre_parts() {
    let net = "// t\nsimulator lang=spectre\nmodel pch_scs bsim4 type=p\n\
        simulator lang=spice\n.model pch_sp pmos level=54\n\
        R_sp a b 10k\nC_sp b ref 1p\nMP_sp_scs d1 g1 s1 b1 pch_scs w=1u\nMP_sp_sp d2 g2 s2 b2 pch_sp\n\
        simulator lang=spectre\nMP_scs_sp (d3 g3 s3 b3) pch_sp\n";
    let r = read_as("/w/t.scs", net);
    let want = [
        ("R_sp", "$RESISTOR"),
        ("C_sp", "$CAPACITOR"),
        ("MP_sp_scs", "$PMOS_4TERM"),
        ("MP_sp_sp", "$PMOS_4TERM"),
        ("MP_scs_sp", "$PMOS_4TERM"),
    ];
    for (name, symbol_name) in want {
        assert_eq!(symbol(&r, name), symbol_name, "{name}");
    }
}
