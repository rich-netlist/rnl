// Compile consumer requests against the declarations shipped beside the built core.
import type { Core, Op, SymbolPin, Vertex, WriteRequest } from "../dist/core.js";
import { startCore } from "../dist/client.js";

export const pins: SymbolPin[] = [
  { terminals: [1], names: ["p", "alias"], at: [-1, 0] },
  { terminals: [1], names: ["bus"], bus: true, at: [1, 0] },
  { terminals: [], names: ["free"], at: [0, 1] },
];
export const endpoint: Vertex = { owner: "#R1", pin: 2, reference: "#R1::free" };
export const text: Op = { op: "add-text", level: "file", attach: "#R1", markup: "R", fix_rotate: true };
export const rule: Op = {
  op: "add-rule",
  level: "file",
  select: { kind: "instance", params: { name: "R*" } },
  action: { op: "add-text", markup: "", key: "name" },
};
export const dots: Op = {
  op: "add-rule",
  level: "file",
  select: { kind: "usage", definition: "$JUNCTION", params: { recursive: true } },
  action: { op: "set", symbol: "$BIG_DOT" },
};
export const request: WriteRequest = {
  write: {
    language_version: "0.4",
    dialect: "spice",
    netlist: "* typed write\nR1 a b 1k\n",
    author: "test",
    ops: [
      { op: "use-package", name: "roles", version: "0.4" },
      { op: "define-symbol", level: "file", name: "$S", pins, prims: [{ shape: "circle", points: [[0, 0]], params: { radius: 1 } }] },
      { op: "set", level: "file", object: "#R1", symbol: "$S", at: [0, 0] },
      { op: "add-wire", level: "file", nets: [], via: [endpoint, { at: [2, 1] }] },
      { op: "add-usage", level: "file", definition: "$D", attach: { members: ["#R1", [0, 1]] } },
      text,
    ],
  },
  std: false,
  package_dirs: ["/defs"],
  files: {
    "/defs/roles.rnl": '@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="roles",version="0.4") }\n@RNL { $D = @DEFINE(roles=["{members}"]) {} }\n',
  },
};
export const released: WriteRequest = {
  write: { dialect: "spice", netlist: "* old\nR1 a b 1k\n", author: "test", ops: [] },
};

// Reject malformed identities and option types at the consumer boundary.
// @ts-expect-error a pin index without its writable reference is not a vertex
export const missingReference: Vertex = { owner: "#R1", pin: 2 };
// @ts-expect-error the flag is a boolean, not its spelling as text
export const wrongFlag: Op = { op: "add-text", markup: "R", fix_rotate: "true" };
// @ts-expect-error edit role collections contain ids or points, not nested collections
export const nestedRole: Op = { op: "add-usage", definition: "$D", attach: { members: [["#R1"]] } };

// @ts-expect-error a rule's text goes on what it selects, so it takes no place of its own
export const placedRule: Op = { op: "add-rule", level: "file", select: { kind: "net" }, action: { op: "add-text", markup: "", attach: "#R1" } };

export function write(core: Core) {
  core.write(released);
  return core.write(request);
}

export async function writeInWorker() {
  const worker = startCore();
  try {
    return await worker.write(request);
  } finally {
    worker.terminate();
  }
}
