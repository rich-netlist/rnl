// The page of smoke.mjs: the package as a frontend uses it, the core in its Worker. It builds a
// netlist of 0.3 that draws a symbol of analog-canvas-extensions, which the core bundles, and writes
// an edit into a netlist without RNL records. #result gets what smoke.mjs checks, and its
// data-state "done", or "failed" with the error.
import { startCore } from "@rich-netlist/runtime";

const netlist = `* smoke
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="analog-canvas-extensions", version="0.1")}
R1 a 0 1k
E1 b 0 a 0 2
* @RNL { #R1 @PLACE(0, 0); #E1 @SET-SYMBOL $CANVAS_VCVS_DIAMOND => @PLACE(10, 0) }
.end
`;

const edited = `* edit
R1 a 0 1k
.end
`;

const result = document.getElementById("result");
const core = startCore();
try {
  const drawn = await core.build({ entry: "/smoke.sp", files: { "/smoke.sp": netlist } });
  const write = { dialect: "spice", netlist: edited, author: "smoke", ops: [{ level: "file", op: "set", object: "#R1", at: [2, 0] }] };
  const { text } = await core.write({ write, svg: false });
  result.textContent = JSON.stringify({
    levels: drawn.levels.length,
    errors: drawn.diagnostics.filter((d) => d.severity === "error").map((d) => d.message),
    svg: drawn.levels[0]?.svg?.length ?? 0,
    written: text,
  });
  result.dataset.state = "done";
} catch (e) {
  result.textContent = String(e?.stack ?? e);
  result.dataset.state = "failed";
} finally {
  core.terminate();
}
