//! `rnl render --png`: a PNG image of each drawing level instead of its SVG, drawn with the fonts the
//! command holds.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const NETLIST: &str = "* t\nR1 a b 10k\nR2 b 0 20k\nX1 a 0 cell\n.subckt cell p n\nC1 p n 1p\n.ends cell\n.end\n";

fn dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sp"), NETLIST).unwrap();
    d
}

fn render(d: &Path, more: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rnl"))
        .arg("render")
        .arg(d.join("t.sp"))
        .arg("-o")
        .arg(d.join("out"))
        .arg("--no-std")
        .args(more)
        .output()
        .unwrap()
}

/// The width and height of a PNG image, from its header.
fn size(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let be = |i: usize| u32::from_be_bytes(png[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

/// The width and height of an SVG, from its attributes.
fn svg_size(svg: &str) -> (f32, f32) {
    let attr = |name: &str| {
        let at = svg.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
        svg[at..at + svg[at..].find('"').unwrap()].parse::<f32>().unwrap()
    };
    (attr("width"), attr("height"))
}

#[test]
fn png_writes_an_image_of_each_level_scaled() {
    let d = dir("render-png");
    let out = render(&d, &["--png"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let printed = String::from_utf8_lossy(&out.stdout);
    let files: Vec<&str> = printed.lines().collect();
    assert_eq!(files.len(), 2, "{printed}");
    assert!(files[0].ends_with("t.png") && files[1].ends_with("t.cell.png"), "{printed}");
    // the SVG of the level, twice its units by default
    let svg_out = render(&d, &[]);
    assert!(svg_out.status.success());
    let (w, h) = svg_size(&std::fs::read_to_string(d.join("out").join("t.svg")).unwrap());
    let image = std::fs::read(d.join("out").join("t.png")).unwrap();
    assert_eq!(size(&image), ((w * 2.0).ceil() as u32, (h * 2.0).ceil() as u32));
    let out = render(&d, &["--png", "--scale", "1"]);
    assert!(out.status.success());
    let image = std::fs::read(d.join("out").join("t.png")).unwrap();
    assert_eq!(size(&image), (w.ceil() as u32, h.ceil() as u32));
}

#[test]
fn scale_takes_a_number_above_0_and_at_most_16() {
    let d = dir("render-png-scale");
    for bad in ["0", "-1", "17", "two"] {
        let out = render(&d, &["--png", "--scale", bad]);
        assert_eq!(out.status.code(), Some(2), "{bad}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("--scale needs a number above 0 and at most 16"), "{bad}: {err}");
    }
}

#[test]
fn net_names_are_written_when_asked() {
    let d = dir("render-net-names");
    // two resistors placed, and a wire on b
    let placed = "* t
* @RNL(type=\"header\") { @VERSION(\"0.4\") }
R1 a b 1k
R2 b c 1k
* @RNL {
*   $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (2,0) @PIN(terminal=2) };
*   # @SELECT-INSTANCE => @SET-SYMBOL $S;
*   #R1 @PLACE(0, 0); #R2 @PLACE(4, 0);
*   &b @WIRE {#R1:2 #R2:1}
* }
.end
";
    std::fs::write(d.join("t.sp"), placed).unwrap();
    let out = render(&d, &[]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let plain = std::fs::read_to_string(d.join("out").join("t.svg")).unwrap();
    assert!(!plain.contains("net-labels"), "off by default");
    let out = render(&d, &["--net-names"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let named = std::fs::read_to_string(d.join("out").join("t.svg")).unwrap();
    assert_eq!(named.matches("<g class=\"net-labels\"").count(), 1, "{named}");
    for net in ["a", "b", "c"] {
        assert!(named.contains(&format!("paint-order=\"stroke\">{net}</text>")), "{net} in {named}");
    }
    // and in the image
    let out = render(&d, &["--net-names", "--png"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
