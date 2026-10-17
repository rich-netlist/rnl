# apps

User-facing applications.

| Path | Content |
| --- | --- |
| [`desktop/`](desktop/README.md) | RNL Viewer: opens a netlist, draws its levels and lists its diagnostics. It can solve a netlist as a preview and save a solved copy; it never changes the files it opens. |
| [`viewer-ui/`](viewer-ui/README.md) | The page of the viewers: the drawing, the netlist text, the problems and the hierarchy, behind a shell per platform. The desktop viewer and the web viewer show it. |
| [`web/`](web/README.md) | The web viewer: the same page in a browser, with the core as WebAssembly in a Worker. It opens files, a folder or a URL, and only reads. A static site, built with Node, and published on the project site at [rich-netlist.github.io/rnl/viewer/](https://rich-netlist.github.io/rnl/viewer/). |

See the [roadmap](../docs/roadmap.md) for planned work.
