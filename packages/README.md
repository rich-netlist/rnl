# packages

TypeScript packages. There is no pnpm workspace yet. The one package that exists is built from [`bindings/runtime-wasm`](../bindings/runtime-wasm/README.md) and [`bindings/solver-wasm`](../bindings/solver-wasm/README.md).

| Path | Content |
| --- | --- |
| [`runtime/`](runtime/README.md) | `@rich-netlist/runtime`, the core as WebAssembly for the browser and Node, with the solver as a second module (`@rich-netlist/runtime/solver`): the README of the npm package, and [`smoke/`](runtime/smoke/README.md), a frontend built with Vite that installs the package as npm does. It is released with the core: a pushed tag `core-v<version>` publishes it ([`publish-runtime.yml`](../.github/workflows/publish-runtime.yml)) under the dist-tag the maintainer sets for that release, `latest` for a release such as 0.4.0 ([release lines](../docs/releases.md)). |

A framework independent viewer core and an editing core are not started; they have no directories here. See the [roadmap](../docs/roadmap.md) for planned work.
