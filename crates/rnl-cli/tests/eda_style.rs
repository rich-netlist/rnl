//! Independent public EDA geometry and style packages, loaded from disk.

use std::path::{Path, PathBuf};

use rnl_cli::{interpret_file, FsProvider};
use rnl_render::{Kind, RenderOptions};
use rnl_runtime::diag::Severity;
use rnl_runtime::model::Layer;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn package_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/eda-style")
}

fn symbol_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/eda-symbol")
}

fn clean(r: &Interpretation) {
    let problems: Vec<_> = r
        .diagnostics()
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
        .map(|d| r.sink.render(d))
        .collect();
    assert!(problems.is_empty(), "{problems:?}");
}

fn symbols() -> rnl_view::Catalog {
    let options = rnl_view::CatalogOptions { packages: Some(vec!["eda-symbol".into()]), ..Default::default() };
    rnl_view::catalog(&FsProvider::new(&[symbol_dir()]), &options)
}

#[test]
fn package_and_gallery_check_without_errors_or_warnings_and_the_gallery_renders() {
    let provider = FsProvider::new(&[symbol_dir(), package_dir()]);
    for (dir, name, levels) in [
        (symbol_dir(), "eda-symbol.rnl", 0),
        (package_dir(), "eda-style.rnl", 0),
        (package_dir(), "eda-style-gallery.scs", 7),
        (package_dir(), "examples/custom-symbol.sp", 2),
    ] {
        let r = interpret_file(&dir.join(name), None, &provider, &Options::default()).unwrap();
        clean(&r);
        assert_eq!(r.model.levels.len(), levels, "{name}");
        for level in &r.model.levels {
            let svg = rnl_render::svg(level, &RenderOptions::default());
            assert!(svg.contains("<svg"), "{name}: {}", level.title);
        }
        if name.ends_with(".scs") {
            let svg = rnl_render::svg(&r.model.levels[0], &RenderOptions::default());
            assert!(svg.contains("#00cc66"));
            assert!(svg.contains("#ff0000"));
            assert!(svg.contains("font-family=\"Lucida Console, Monaco, DejaVu Sans Mono, Consolas, monospace\""));
            assert!(svg.contains("vector-effect=\"non-scaling-stroke\""));
        }
    }
}

#[test]
fn every_eda_symbol_has_its_pin_sides_on_its_outermost_pins() {
    let c = symbols();
    assert_eq!(c.packages.len(), 1);
    assert_eq!(c.symbols.len(), 137);
    for s in &c.symbols {
        let [x1, y1, x2, y2] = s.bbox;
        for p in &s.pins {
            let [x, y] = p.at;
            assert!(x >= x1 && x <= x2 && y >= y1 && y <= y2, "{} pin {p:?} outside {:?}", s.symbol, s.bbox);
        }
        for (direction, axis, edge, minimum) in
            [("left", 0, x1, true), ("down", 1, y1, true), ("right", 0, x2, false), ("up", 1, y2, false)]
        {
            let pins = s.pins.iter().filter(|p| p.direction == Some(direction)).map(|p| p.at[axis]);
            let outermost = pins.reduce(if minimum { f64::min } else { f64::max });
            if let Some(pin) = outermost {
                assert_eq!(edge, pin, "{} {direction} box edge", s.symbol);
            }
        }
    }
}

#[test]
fn neither_package_is_builtin_and_neither_imports_the_other() {
    let c = rnl_view::catalog(&FsProvider::new(&[]), &rnl_view::CatalogOptions::default());
    for name in ["eda-symbol", "eda-style"] {
        assert!(!c.packages.iter().any(|p| p.name == name));
    }
    let geometry = symbols();
    assert!(geometry.packages[0].references.is_empty());
    assert!(geometry.definitions.is_empty());
    let options = rnl_view::CatalogOptions { packages: Some(vec!["eda-style".into()]), ..Default::default() };
    let style = rnl_view::catalog(&FsProvider::new(&[package_dir()]), &options);
    assert_eq!(style.packages.len(), 1);
    assert!(style.packages[0].references.is_empty());
    assert!(style.symbols.is_empty());
    assert_eq!(style.definitions.len(), 1);
    assert_eq!(style.definitions[0].definition, "$PIN_SQUARE");
}

#[test]
fn all_93_shared_symbols_follow_standard_terminal_directions() {
    let geometry = symbols();
    let standard = rnl_view::catalog(&FsProvider::new(&[]), &rnl_view::CatalogOptions::default());
    let shared: Vec<_> = standard.symbols.iter().filter(|s| s.version == "0.3").collect();
    assert_eq!(shared.len(), 93);
    for s in shared {
        let actual = geometry.symbols.iter().find(|a| a.symbol == s.symbol).unwrap();
        assert_eq!(actual.pins.len(), s.pins.len(), "{}", s.symbol);
        for pin in &s.pins {
            let actual = actual.pins.iter().find(|a| a.terminal == pin.terminal).unwrap();
            assert_eq!(actual.direction, pin.direction, "{} terminal {:?}", s.symbol, pin.terminal);
        }
    }
    for (name, count) in [("$SCHOTTKY", 2), ("$SP3TSWITCH", 4), ("$SP4TSWITCH", 5)] {
        let s = geometry.symbols.iter().find(|s| s.symbol == name).unwrap();
        assert_eq!(s.pins.len(), count);
        assert_eq!(s.pins[0].direction, Some("left"));
        assert!(s.pins[1..].iter().all(|p| p.direction == Some("right")));
    }
    for name in ["$D_FLIP_FLOP", "$D_FLIP_FLOP_Q_ONLY", "$D_FLIP_FLOP_WITH_RESET"] {
        let s = geometry.symbols.iter().find(|s| s.symbol == name).unwrap();
        assert!(s.pins[0].at[1] > s.pins[1].at[1], "{name}: D above CLK");
        if name == "$D_FLIP_FLOP_Q_ONLY" {
            assert_eq!(s.pins[2].at[1], 0.0);
        }
    }
    for name in
        ["$NMOS_4TERM", "$PMOS_4TERM", "$DEPLETION_NMOS_4TERM", "$DEPLETION_PMOS_4TERM", "$NDMOS_4TERM", "$PDMOS_4TERM"]
    {
        let s = geometry.symbols.iter().find(|s| s.symbol == name).unwrap();
        assert_eq!(s.pins[3].at, [0.0, 0.0], "{name}: bulk lead remains at the origin");
    }
}

#[test]
fn style_alone_and_combined_style_mark_standard_custom_and_virtual_pins_once() {
    let example = std::fs::read_to_string(package_dir().join("examples/custom-symbol.sp")).unwrap();
    for (geometry, style) in [(true, false), (false, true), (true, true)] {
        let text = example
            .lines()
            .filter(|line| geometry || !line.contains("@USE-PACKAGE(name=\"eda-symbol\""))
            .filter(|line| style || !line.contains("@USE-PACKAGE(name=\"eda-style\""))
            .collect::<Vec<_>>()
            .join("\n");
        let mut provider = MemoryProvider::new().with("/demo.sp", &text);
        provider.package_dirs.push("/packages".into());
        for (enabled, name, dir) in [(geometry, "eda-symbol", symbol_dir()), (style, "eda-style", package_dir())] {
            if enabled {
                provider.insert(
                    &format!("/packages/{name}.rnl"),
                    &std::fs::read_to_string(dir.join(format!("{name}.rnl"))).unwrap(),
                );
            }
        }
        let r = rnl_view::interpret("/demo.sp", None, &provider, &Options::default()).unwrap();
        clean(&r);
        let level = &r.model.levels[0];
        let prims = rnl_render::primitives(level, &RenderOptions::default());
        let markers: Vec<_> =
            prims.iter().filter(|p| matches!(&p.kind, Kind::Rect { fill: Some(c), .. } if c == "#ff0000")).collect();
        assert_eq!(markers.len(), if style { 6 } else { 0 }, "geometry={geometry}, style={style}");
        for marker in &markers {
            assert_eq!(marker.layer, Layer::Top);
            let [x0, y0, x1, y1] = marker.bbox;
            assert!((x1 - x0 - 0.8).abs() < 1e-12 && (y1 - y0 - 0.8).abs() < 1e-12);
        }
        if style {
            for [x, y] in [[6.0, 0.0], [10.0, 0.0], [-8.0, -10.0], [0.0, -10.0]] {
                assert_eq!(
                    markers
                        .iter()
                        .filter(|p| {
                            let [x0, y0, x1, y1] = p.bbox;
                            ((x0 + x1) / 2.0 - x).abs() < 1e-12 && ((y0 + y1) / 2.0 - y).abs() < 1e-12
                        })
                        .count(),
                    1
                );
            }
            assert!(prims.iter().any(|p| matches!(&p.kind, Kind::Text(t) if t.color == "#ff0000" && t.runs.iter().any(|r| r.text == "custom"))));
            assert!(!level.wires.is_empty());
            assert!(level.wires.iter().all(|w| w.layer == Layer::Bottom));
        }
    }
}
