import { defineConfig } from "vite";

// The two settings a frontend needs for the package installed into node_modules, as the package's
// README says: the Worker is a module with a top-level await, and pre-bundling would move the
// modules away from worker.js and the module next to them.
export default defineConfig({
  worker: { format: "es" },
  optimizeDeps: { exclude: ["@rich-netlist/runtime"] },
});
