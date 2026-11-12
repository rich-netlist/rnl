//! Check the implementation's representable auto geometry after positions have settled.

use super::Interp;
use crate::diag::{codes, Severity};

impl Interp<'_> {
    pub(super) fn check_auto_wire_geometry(&mut self) {
        for scene in 0..self.scenes.len() {
            for wire in self.scenes[scene].wires.clone() {
                if self.is_failed(wire) || self.drawing_ignored(scene, wire) {
                    continue;
                }
                let Some(shape) = self.wire_shape(scene, wire) else { continue };
                if shape.mode != "auto" || !self.modern_wire_source(scene, shape.site.as_ref()) {
                    continue;
                }
                // Unknown coordinates split the wire into valid positioned runs. None here
                // means a known run cannot be routed within our finite geometry bounds.
                if self.wire_runs(scene, wire).is_none() {
                    self.diag_site(
                        Severity::Error,
                        codes::AUTO_WIRE_GEOMETRY_LIMIT,
                        &shape.site,
                        shape.off,
                        "the default auto wire path exceeds this implementation's finite geometry limits (13.1)".into(),
                    );
                }
            }
        }
    }
}
