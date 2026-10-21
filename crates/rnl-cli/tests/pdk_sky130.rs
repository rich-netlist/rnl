//! The SkyWater SKY130 PDK symbol package `pdk-sky130` 0.1 (`libraries/pdk/sky130`):
//! terminal declarations (`@DEVICE`), multi-terminal symbols (`$SKY130_*`), and model-to-symbol
//! rules (`@SELECT-INSTANCE`) for `sky130_fd_pr` in both SPICE and Spectre dialects, tested both
//! without PDK model files and with stubbed `.subckt` / `subckt` definitions.

use std::path::{Path, PathBuf};

#[path = "support/public_packages.rs"]
mod public_packages;

use rnl_cli::{interpret_file, FsProvider};
use rnl_render::RenderOptions;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn pdk_dir() -> PathBuf {
    public_packages::package_dir("pdk-sky130")
}

fn package_file() -> PathBuf {
    pdk_dir().join("pdk-sky130.rnl")
}

fn examples_dir() -> PathBuf {
    pdk_dir().join("examples")
}

fn diags(r: &Interpretation) -> Vec<String> {
    r.diagnostics().iter().map(|d| r.sink.render(d)).collect()
}

fn errors_and_warnings(r: &Interpretation) -> Vec<String> {
    r.diagnostics()
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
        .map(|d| r.sink.render(d))
        .collect()
}

const CUSTOM_SYMBOLS: [&str; 8] = [
    "$SKY130_RESISTOR_3TERM",
    "$SKY130_CAPACITOR_3TERM",
    "$SKY130_CAPACITOR_4TERM",
    "$SKY130_VARACTOR_3TERM",
    "$SKY130_NPN_4TERM",
    "$SKY130_PNP_4TERM",
    "$SKY130_NMOS_5TERM",
    "$SKY130_INDUCTOR_4TERM",
];

#[test]
fn package_checked_on_its_own_has_zero_diagnostics_and_integer_pins() {
    if !public_packages::available() {
        return;
    }
    let text = std::fs::read_to_string(package_file()).unwrap();
    let defined: Vec<&str> = text
        .lines()
        .filter(|l| l.contains(" = @DEFINE-SYMBOL("))
        .map(|l| l.trim_start().split(' ').next().unwrap())
        .collect();
    assert_eq!(defined, CUSTOM_SYMBOLS);

    let r = interpret_file(&package_file(), None, &FsProvider::new(&[]), &Options::default()).unwrap();
    assert!(diags(&r).is_empty(), "{:?}", diags(&r));

    let options = rnl_view::CatalogOptions { packages: Some(vec!["pdk-sky130".into()]), ..Default::default() };
    let c = rnl_view::catalog(&FsProvider::new(&[pdk_dir()]), &options);
    assert_eq!(c.symbols.len(), CUSTOM_SYMBOLS.len());
    for s in &c.symbols {
        let [x1, y1, x2, y2] = s.bbox;
        assert!(!s.pins.is_empty(), "{} has no pins", s.symbol);
        for p in &s.pins {
            let [px, py] = p.at;
            assert!(px >= x1 && px <= x2 && py >= y1 && py <= y2, "{} pin {:?} outside {:?}", s.symbol, p, s.bbox);
        }
        let left: Vec<f64> = s.pins.iter().filter(|p| p.direction == Some("left")).map(|p| p.at[0]).collect();
        let down: Vec<f64> = s.pins.iter().filter(|p| p.direction == Some("down")).map(|p| p.at[1]).collect();
        let right: Vec<f64> = s.pins.iter().filter(|p| p.direction == Some("right")).map(|p| p.at[0]).collect();
        let up: Vec<f64> = s.pins.iter().filter(|p| p.direction == Some("up")).map(|p| p.at[1]).collect();
        if let Some(min_x) = left.into_iter().reduce(f64::min) {
            assert_eq!(x1, min_x, "{} left box edge", s.symbol);
        }
        if let Some(min_y) = down.into_iter().reduce(f64::min) {
            assert_eq!(y1, min_y, "{} down box edge", s.symbol);
        }
        if let Some(max_x) = right.into_iter().reduce(f64::max) {
            assert_eq!(x2, max_x, "{} right box edge", s.symbol);
        }
        if let Some(max_y) = up.into_iter().reduce(f64::max) {
            assert_eq!(y2, max_y, "{} up box edge", s.symbol);
        }
    }
}

#[test]
fn all_sky130_examples_check_and_render_without_and_with_pdk_models() {
    if !public_packages::available() {
        return;
    }
    let pkg_text = std::fs::read_to_string(package_file()).unwrap();
    let stub_spice = r#"* Stub SKY130 subckt headers
* @RNL { @USE-PACKAGE(name="rnl-std-fill") {} }
.subckt sky130_fd_pr__nfet_01v8 d g s b
.ends sky130_fd_pr__nfet_01v8
.subckt sky130_fd_pr__nfet_01v8_lvt d g s b
.ends sky130_fd_pr__nfet_01v8_lvt
.subckt sky130_fd_pr__nfet_05v0_nvt d g s b
.ends sky130_fd_pr__nfet_05v0_nvt
.subckt sky130_fd_pr__pfet_01v8 d g s b
.ends sky130_fd_pr__pfet_01v8
.subckt sky130_fd_pr__pfet_01v8_lvt d g s b
.ends sky130_fd_pr__pfet_01v8_lvt
.subckt sky130_fd_pr__pnp_05v5_W3p40L3p40 c b e s
.ends sky130_fd_pr__pnp_05v5_W3p40L3p40
.subckt sky130_fd_pr__res_high_po_0p35 r0 r1 b
.ends sky130_fd_pr__res_high_po_0p35
.subckt sky130_fd_pr__cap_mim_m3_1 c0 c1
.ends sky130_fd_pr__cap_mim_m3_1
.subckt sky130_fd_pr__cap_mim_m3_2 c0 c1
.ends sky130_fd_pr__cap_mim_m3_2
.subckt sky130_fd_pr__diode_pw2nd_05v5 a k
.ends sky130_fd_pr__diode_pw2nd_05v5
"#;
    let stub_spectre = r#"// Stub SKY130 subckt headers
// @RNL { @USE-PACKAGE(name="rnl-std-fill") {} }
subckt sky130_fd_pr__nfet_01v8 (d g s b)
ends sky130_fd_pr__nfet_01v8
subckt sky130_fd_pr__pfet_01v8 (d g s b)
ends sky130_fd_pr__pfet_01v8
subckt sky130_fd_pr__cap_mim_m3_1 (c0 c1)
ends sky130_fd_pr__cap_mim_m3_1
"#;

    for name in ["ota5t.sp", "ota5t.scs", "bandgap.sp", "mim_switch.sp"] {
        let path = examples_dir().join(name);
        let fs_provider = FsProvider::new(&[pdk_dir()]);

        // 1. Without PDK model files
        let r_without = interpret_file(&path, None, &fs_provider, &Options::default()).unwrap();
        assert!(diags(&r_without).is_empty(), "{name} (without PDK): {:?}", diags(&r_without));
        let lv_without = r_without.model.levels.iter().find(|l| l.title == name).unwrap();
        let svg_without = rnl_render::svg(lv_without, &RenderOptions::default());
        assert!(svg_without.contains("<svg"), "{name}");

        // 2. With PDK model files (.subckt / subckt headers included)
        let raw = std::fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
        let with_pdk = if name.ends_with(".sp") {
            raw.replace("* .include \"sky130_fd_pr_stub.spice\"", ".include \"sky130_fd_pr_stub.spice\"")
        } else {
            raw.replace("// include \"sky130_fd_pr_stub.scs\"", "include \"sky130_fd_pr_stub.scs\"")
        };
        assert_ne!(with_pdk, raw, "{name} should contain stub include comment");

        let mut mem = MemoryProvider::new()
            .with(name, &with_pdk)
            .with("sky130_fd_pr_stub.spice", stub_spice)
            .with("sky130_fd_pr_stub.scs", stub_spectre)
            .with("pkg/pdk-sky130.rnl", &pkg_text);
        mem.package_dirs.push("pkg".into());
        let r_with = interpret_file(Path::new(name), None, &mem, &Options::default()).unwrap();
        assert!(diags(&r_with).is_empty(), "{name} (with PDK): {:?}", diags(&r_with));
        let lv_with = r_with.model.levels.iter().find(|l| l.title == name).unwrap();
        let svg_with = rnl_render::svg(lv_with, &RenderOptions::default());
        assert_eq!(svg_with, svg_without, "{name}: SVG should match with and without PDK models");
    }
}

#[test]
fn device_families_and_symbols_match_in_spice_and_spectre() {
    if !public_packages::available() {
        return;
    }
    let pkg_text = std::fs::read_to_string(package_file()).unwrap();
    let spice = r#"* All SKY130 device families in SPICE
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="pdk-sky130", version="0.1") }
XM_N d g s 0 sky130_fd_pr__nfet_01v8 w=1 l=0.15
XM_NVT d g s 0 sky130_fd_pr__nfet_05v0_nvt w=2 l=0.9
XM_ND d g s 0 sky130_fd_pr__nfet_20v0 w=2 l=1
XM_ISO d g s b 0 sky130_fd_pr__nfet_20v0_iso w=2 l=1
XM_P d g s vdd sky130_fd_pr__pfet_01v8_hvt w=2 l=0.15
XM_PD d g s vdd sky130_fd_pr__pfet_20v0 w=2 l=1
XQ_N c b e 0 sky130_fd_pr__npn_05v5_W1p00L1p00
XQ_P 0 b e 0 sky130_fd_pr__pnp_05v5_W3p40L3p40
XR_PO a b 0 sky130_fd_pr__res_high_po_0p35 l=5
XR_XPO a b 0 sky130_fd_pr__res_xhigh_po w=0.35 l=5
XR_DIFF a b 0 sky130_fd_pr__res_generic_nd w=1 l=5
XR_M1 a b sky130_fd_pr__res_generic_m1 w=1 l=5
XC_MIM a b sky130_fd_pr__cap_mim_m3_1 w=5 l=5
XC_VAR a b 0 sky130_fd_pr__cap_var_lvt w=5 l=0.5
XC_VPP3 a b 0 sky130_fd_pr__cap_vpp_08p6x07p8_l1m1m2_noshield_o1
XC_VPP4 a b 0 vdd sky130_fd_pr__cap_vpp_11p5x11p7_l1m1m2m3m4_shieldm5_floatm3
XD_PW 0 a sky130_fd_pr__diode_pw2nd_05v5
XL_IND a b ct 0 sky130_fd_pr__ind_03_90
* @RNL {
*   # @SELECT-INSTANCE => @PLACE(0, 0);
*   &d @WIRE {#XM_N:d #XM_NVT:d #XM_ND:d #XM_ISO:d #XM_P:d #XM_PD:d};
*   &0 @WIRE {#XM_ISO:sub #XQ_N:s #XQ_P:s #XR_PO:b #XR_XPO:sub #XR_DIFF:b #XC_VAR:b #XC_VPP3:b #XC_VPP4:b #XL_IND:sub};
*   &a @WIRE {#XR_PO:r0 #XR_XPO:r0 #XR_DIFF:t1 #XR_M1:r0 #XC_MIM:c0 #XC_VAR:c0 #XC_VPP3:c0 #XC_VPP4:c0 #XD_PW:k #XL_IND:a};
*   &vdd @WIRE {#XC_VPP4:m5 #XM_P:b #XM_PD:b}
* }
.end
"#;
    let spectre = r#"// SKY130 device families in Spectre (no X prefix)
// @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="pdk-sky130", version="0.1") }
M_N (d g s 0) sky130_fd_pr__nfet_01v8 w=1u l=0.15u
M_P (d g s vdd) sky130_fd_pr__pfet_01v8_hvt w=2u l=0.15u
Q_P (0 b e 0) sky130_fd_pr__pnp_05v5_W3p40L3p40
R_PO (a b 0) sky130_fd_pr__res_high_po_0p35 l=5u
C_MIM (a b) sky130_fd_pr__cap_mim_m3_1 w=5u l=5u
D_PW (0 a) sky130_fd_pr__diode_pw2nd_05v5
L_IND (a b ct 0) sky130_fd_pr__ind_03_90
// @RNL {
//   # @SELECT-INSTANCE => @PLACE(0, 0);
//   &d @WIRE {#M_N:d #M_P:d};
//   &0 @WIRE {#Q_P:s #R_PO:b #L_IND:sub};
//   &a @WIRE {#R_PO:r0 #C_MIM:c0 #D_PW:k #L_IND:a}
// }
"#;
    let mut mem =
        MemoryProvider::new().with("all.sp", spice).with("all.scs", spectre).with("pkg/pdk-sky130.rnl", &pkg_text);
    mem.package_dirs.push("pkg".into());
    let r = interpret_file(Path::new("all.sp"), None, &mem, &Options::default()).unwrap();
    assert!(errors_and_warnings(&r).is_empty(), "{:?}", errors_and_warnings(&r));

    let lv = r.model.levels.iter().find(|l| l.title == "all.sp").unwrap();
    let sym_of = |id: &str| -> &str {
        let item = lv.items.iter().find(|i| i.id == id).unwrap_or_else(|| panic!("missing {id}"));
        lv.symbols[item.symbol].name.as_str()
    };
    assert_eq!(sym_of("#XM_N"), "$NMOS_4TERM");
    assert_eq!(sym_of("#XM_NVT"), "$DEPLETION_NMOS_4TERM");
    assert_eq!(sym_of("#XM_ND"), "$NDMOS_4TERM");
    assert_eq!(sym_of("#XM_ISO"), "$SKY130_NMOS_5TERM");
    assert_eq!(sym_of("#XM_P"), "$PMOS_4TERM");
    assert_eq!(sym_of("#XM_PD"), "$PDMOS_4TERM");
    assert_eq!(sym_of("#XQ_N"), "$SKY130_NPN_4TERM");
    assert_eq!(sym_of("#XQ_P"), "$SKY130_PNP_4TERM");
    assert_eq!(sym_of("#XR_PO"), "$SKY130_RESISTOR_3TERM");
    assert_eq!(sym_of("#XR_XPO"), "$SKY130_RESISTOR_3TERM");
    assert_eq!(sym_of("#XR_DIFF"), "$SKY130_RESISTOR_3TERM");
    assert_eq!(sym_of("#XR_M1"), "$RESISTOR");
    assert_eq!(sym_of("#XC_MIM"), "$CAPACITOR");
    assert_eq!(sym_of("#XC_VAR"), "$SKY130_VARACTOR_3TERM");
    assert_eq!(sym_of("#XC_VPP3"), "$SKY130_CAPACITOR_3TERM");
    assert_eq!(sym_of("#XC_VPP4"), "$SKY130_CAPACITOR_4TERM");
    assert_eq!(sym_of("#XD_PW"), "$DIODE");
    assert_eq!(sym_of("#XL_IND"), "$SKY130_INDUCTOR_4TERM");

    let r_scs = interpret_file(Path::new("all.scs"), None, &mem, &Options::default()).unwrap();
    assert!(errors_and_warnings(&r_scs).is_empty(), "{:?}", errors_and_warnings(&r_scs));
    let lv_scs = r_scs.model.levels.iter().find(|l| l.title == "all.scs").unwrap();
    let sym_scs = |id: &str| -> &str {
        let item = lv_scs.items.iter().find(|i| i.id == id).unwrap_or_else(|| panic!("missing {id}"));
        lv_scs.symbols[item.symbol].name.as_str()
    };
    assert_eq!(sym_scs("#M_N"), "$NMOS_4TERM");
    assert_eq!(sym_scs("#M_P"), "$PMOS_4TERM");
    assert_eq!(sym_scs("#Q_P"), "$SKY130_PNP_4TERM");
    assert_eq!(sym_scs("#R_PO"), "$SKY130_RESISTOR_3TERM");
    assert_eq!(sym_scs("#C_MIM"), "$CAPACITOR");
    assert_eq!(sym_scs("#D_PW"), "$DIODE");
    assert_eq!(sym_scs("#L_IND"), "$SKY130_INDUCTOR_4TERM");
}
