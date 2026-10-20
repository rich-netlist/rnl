import { defineConfig } from "vite";
import { publicPackageDir, publicPackageText } from "../../../scripts/packages.mjs";
import { join } from "node:path";
publicPackageText("analog-canvas-extensions");

// The two settings a frontend needs for the package installed into node_modules, as the package's
// README says: the Worker is a module with a top-level await, and pre-bundling would move the
// modules away from worker.js and the module next to them.
export default defineConfig({
  resolve: { alias: { "rnl-public-canvas?url": join(publicPackageDir("analog-canvas-extensions"), "analog-canvas-extensions.rnl") + "?url" } },
  worker: { format: "es" },
  optimizeDeps: { exclude: ["@rich-netlist/runtime"] },
});
