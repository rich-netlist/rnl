# packages

TypeScript packages. There is no pnpm workspace yet. The one package that exists is built from [`bindings/runtime-wasm`](../bindings/runtime-wasm/README.md).

| Path | Content |
| --- | --- |
| [`runtime/`](runtime/README.md) | `@rich-netlist/runtime`, the core as WebAssembly for the browser and Node: the README of the npm package, and [`smoke/`](runtime/smoke/README.md), a frontend built with Vite that installs the package as npm does. Previews are published under the dist-tag `next` when a tag `runtime-v<version>` is pushed ([`publish-runtime.yml`](../.github/workflows/publish-runtime.yml)). |
| [`viewer/`](viewer/README.md), [`editor/`](editor/README.md), [`solver/`](solver/README.md) | Not started: a framework independent viewer core, an editing core, and a TypeScript interface to the solver's WebAssembly binding, which does not exist yet either. |

See the [roadmap](../docs/roadmap.md) for planned work.
