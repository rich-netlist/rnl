//! Orientation codes and the routing of wires (spec-draft 11.2, 11.7, 13.1).

/// A point in scene or symbol coordinates: x to the right, y up (11.7).
pub type Pt = (f64, f64);

/// One of the eight orientation codes, named as in Virtuoso (11.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Orient {
    #[default]
    R0,
    R90,
    R180,
    R270,
    MY,
    MX,
    MYR90,
    MXR90,
}

impl Orient {
    pub const ALL: [Orient; 8] =
        [Orient::R0, Orient::R90, Orient::R180, Orient::R270, Orient::MY, Orient::MX, Orient::MYR90, Orient::MXR90];

    pub fn parse(s: &str) -> Option<Orient> {
        Some(match s {
            "R0" => Orient::R0,
            "R90" => Orient::R90,
            "R180" => Orient::R180,
            "R270" => Orient::R270,
            "MY" => Orient::MY,
            "MX" => Orient::MX,
            "MYR90" => Orient::MYR90,
            "MXR90" => Orient::MXR90,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Orient::R0 => "R0",
            Orient::R90 => "R90",
            Orient::R180 => "R180",
            Orient::R270 => "R270",
            Orient::MY => "MY",
            Orient::MX => "MX",
            Orient::MYR90 => "MYR90",
            Orient::MXR90 => "MXR90",
        }
    }

    /// The matrix (a, b, c, d): (x, y) becomes (a x + b y, c x + d y).
    pub fn matrix(self) -> (i32, i32, i32, i32) {
        match self {
            Orient::R0 => (1, 0, 0, 1),
            Orient::R90 => (0, -1, 1, 0),
            Orient::R180 => (-1, 0, 0, -1),
            Orient::R270 => (0, 1, -1, 0),
            Orient::MY => (-1, 0, 0, 1),
            Orient::MX => (1, 0, 0, -1),
            Orient::MYR90 => (0, -1, -1, 0),
            Orient::MXR90 => (0, 1, 1, 0),
        }
    }

    pub fn apply(self, p: Pt) -> Pt {
        let (a, b, c, d) = self.matrix();
        (a as f64 * p.0 + b as f64 * p.1, c as f64 * p.0 + d as f64 * p.1)
    }

    /// `outer` applied after `self`.
    pub fn then(self, outer: Orient) -> Orient {
        let (a0, a1, a2, a3) = outer.matrix();
        let (b0, b1, b2, b3) = self.matrix();
        let m = (a0 * b0 + a1 * b2, a0 * b1 + a1 * b3, a2 * b0 + a3 * b2, a2 * b1 + a3 * b3);
        Orient::ALL.into_iter().find(|o| o.matrix() == m).unwrap_or(Orient::R0)
    }

    /// Does the code swap the axes (a quarter turn)?
    pub fn swaps_axes(self) -> bool {
        matches!(self, Orient::R90 | Orient::R270 | Orient::MYR90 | Orient::MXR90)
    }
}

pub const TOL: f64 = 1e-6;

/// The vertices of a wire through its points for a routing mode (13.1). `auto` is drawn
/// straight by a viewer that does not route.
pub fn route(pts: &[Pt], mode: &str) -> Vec<Pt> {
    if pts.len() < 2 {
        return pts.to_vec();
    }
    let mut out = vec![pts[0]];
    for w in pts.windows(2) {
        let ((x1, y1), (x2, y2)) = (w[0], w[1]);
        let (dx, dy) = (x2 - x1, y2 - y1);
        if mode == "auto" || mode == "straight" || dx.abs() < TOL || dy.abs() < TOL {
            out.push(w[1]);
            continue;
        }
        match mode {
            "horizontal-vertical" => out.extend([(x2, y1), w[1]]),
            "vertical-horizontal" => out.extend([(x1, y2), w[1]]),
            "manhattan" => {
                if dx.abs() >= dy.abs() {
                    out.extend([(x1 + dx / 2.0, y1), (x1 + dx / 2.0, y2), w[1]]);
                } else {
                    out.extend([(x1, y1 + dy / 2.0), (x2, y1 + dy / 2.0), w[1]]);
                }
            }
            "manhattan45" => {
                if dx.abs() >= dy.abs() {
                    let run = (dx.abs() - dy.abs()) / 2.0 * dx.signum();
                    out.extend([(x1 + run, y1), (x2 - run, y2), w[1]]);
                } else {
                    let run = (dy.abs() - dx.abs()) / 2.0 * dy.signum();
                    out.extend([(x1, y1 + run), (x2, y2 - run), w[1]]);
                }
            }
            _ => out.push(w[1]),
        }
    }
    out
}

/// A number written back so that it reads as the same value, shortest form (4.3, 17).
pub fn fmt_num(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        let s = format!("{v}");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes() {
        assert_eq!(Orient::R90.apply((1.0, 0.0)), (0.0, 1.0));
        assert_eq!(Orient::MYR90.apply((1.0, 2.0)), (-2.0, -1.0));
        assert_eq!(Orient::MXR90.apply((1.0, 2.0)), (2.0, 1.0));
        // MYR90 is MY then R90
        assert_eq!(Orient::MY.then(Orient::R90), Orient::MYR90);
        assert_eq!(Orient::R90.then(Orient::R90), Orient::R180);
    }

    #[test]
    fn routes() {
        assert_eq!(route(&[(0.0, 0.0), (4.0, 2.0)], "manhattan"), vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (4.0, 2.0)]);
        assert_eq!(route(&[(0.0, 0.0), (4.0, 2.0)], "manhattan45"), vec![(0.0, 0.0), (1.0, 0.0), (3.0, 2.0), (4.0, 2.0)]);
        assert_eq!(fmt_num(2.0), "2");
        assert_eq!(fmt_num(-0.5), "-0.5");
    }
}
