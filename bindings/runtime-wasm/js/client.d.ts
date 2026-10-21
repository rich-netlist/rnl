// client.js as TypeScript declarations, written by hand; keep them with the JSDoc in client.js.

import type {
  Catalog,
  CatalogRequest,
  Document,
  DrawingsRequest,
  LibraryRequest,
  Request,
  Snapshot,
  UnplacedDrawing,
  WriteRequest,
  Written,
} from "./core.js";

export type {
  Catalog,
  CatalogRequest,
  Document,
  DrawingsRequest,
  LibraryRequest,
  Literal,
  Op,
  Point,
  Request,
  Run,
  RunStyle,
  Snapshot,
  UnplacedDrawing,
  Vertex,
  WriteRequest,
  Written,
} from "./core.js";

/** The core, running in a Worker. */
export interface CoreWorker {
  /** Build the Document of a request; rejects with the core's message when there is none. */
  build(request: Request): Promise<Document>;
  /** Build the snapshot of a request with `parts`: the editor interface's view of the document. */
  snapshot(request: Request): Promise<Snapshot>;
  /** Write the ops into the netlist and build what was written; rejects when an op cannot be written. */
  write(request: WriteRequest): Promise<Written>;
  /** List the symbols, definitions and sets of the packages the core has. */
  catalog(request: CatalogRequest): Promise<Catalog>;
  /** The drawings of the objects without a position that the last build with `lazy_unplaced` listed, as the request names them. */
  drawings(request: DrawingsRequest): Promise<UnplacedDrawing[]>;
  /** Update the viewer library index; returns read errors. */
  library(request: LibraryRequest): Promise<string[]>;
  /** Stop the Worker; calls still waiting, and later ones, reject. */
  terminate(): void;
}

/** Start the core in a module Worker: worker.js next to this module, unless `worker` says where. */
export function startCore(options?: { worker?: string | URL }): CoreWorker;

/** Build a document with the files it needs, read through `read` until nothing new can be read. */
export function collect(
  core: { build(request: Request): Document | Promise<Document> },
  request: Request,
  read: (path: string) => Promise<string | null>,
): Promise<{ document: Document; files: Record<string, string> }>;

/** A file's text from its bytes, as rnl reads a file: UTF-8 without a BOM, else Latin-1. */
export function decodeText(bytes: BufferSource): string;
