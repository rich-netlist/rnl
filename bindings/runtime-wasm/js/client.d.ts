// client.js as TypeScript declarations, written by hand; keep them with the JSDoc in client.js.

import type {
  Catalog,
  CatalogRequest,
  Document,
  DrawingsRequest,
  FileResult,
  FileSymbols,
  FolderListing,
  FormatList,
  FormatsRequest,
  LibraryRequest,
  Outline,
  ReplaceRecordsRequest,
  Replaced,
  Request,
  Snapshot,
  UnplacedDrawing,
  WriteRequest,
  Written,
} from "./core.js";

export type {
  EditError,
  Catalog,
  CatalogRequest,
  Document,
  DrawingsRequest,
  FileCall,
  FileChild,
  FileInput,
  FileInputs,
  FileOutline,
  FileResult,
  FileSymbols,
  FolderListing,
  FormatInfo,
  FormatList,
  FormatsRequest,
  LibraryRequest,
  Literal,
  Op,
  Outline,
  OutlineCall,
  OutlineFile,
  OutlineLevel,
  OverlayOptions,
  OverlayStyle,
  Point,
  ReplaceRecordsRequest,
  Replaced,
  Request,
  Run,
  RunStyle,
  Snapshot,
  SubcktSymbol,
  UnplacedDrawing,
  Vertex,
  WriteRequest,
  Written,
} from "./core.js";

export { coreVersion, fileResultFormat } from "./version.js";

/** The core, running in a Worker. */
export interface CoreWorker {
  /** Build the Document of a request; rejects with the core's message when there is none. */
  build(request: Request): Promise<Document>;
  /** Build the snapshot of a request with `parts`: the editor interface's view of the document. */
  snapshot(request: Request): Promise<Snapshot>;
  /**
   * The outline of a request's entry: its files, levels and the subcircuits each calls, the language
   * version and packages of each file, and the diagnostics of reading the netlist, with no RNL
   * interpreted.
   */
  outline(request: Request): Promise<Outline>;
  /**
   * The reading of the request's `file`, one file of the document, in the context of its entry. Keep
   * one Worker for a run of them: later versions keep the packages it classified.
   */
  buildFile(request: Request & { file: string }): Promise<FileResult>;
  /** The symbols the subcircuits of the request's `symbols`, a file of the document, give themselves. */
  symbols(request: Request & { symbols: string }): Promise<FileSymbols>;
  /** Write and build. Refusals reject with EditError, carrying a code and host diagnostics. */
  write(request: WriteRequest): Promise<Written>;
  /**
   * Replace the records of a level of a netlist that has records already, in the file that holds the
   * level: the path of that file and its new text. Rejects with the core's message, which begins with
   * the kind of the refusal, when the level is not found, has no place for a record, or the records
   * given do not read.
   */
  replaceRecords(request: ReplaceRecordsRequest): Promise<Replaced>;
  /** List the symbols, definitions and sets of the packages the core has. */
  catalog(request: CatalogRequest): Promise<Catalog>;
  /** The drawings of the objects without a position that the last build with `lazy_unplaced` listed, as the request names them. */
  drawings(request: DrawingsRequest): Promise<UnplacedDrawing[]>;
  /** Update the viewer library index; returns read errors. */
  library(request: LibraryRequest): Promise<string[]>;
  /** The formats a file can be read with, and the format each file of `detect` has when none is set. */
  formats(request: FormatsRequest): Promise<FormatList>;
  /** The bundled packages a frontend lists among its built-in ones, by path with their texts: all but those of a draft series. */
  bundled(): Promise<Record<string, string>>;
  /**
   * The files of folders as a browser gives them (each path's first part the folder included, or a file given alone)
   * that the Library lists, as every viewer lists a folder included; the folders that had more files than are listed.
   */
  folders(paths: string[]): Promise<FolderListing>;
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

/** The outline of a document with the files it needs, read through `read` as `collect` reads them. */
export function collectOutline(
  core: { outline(request: Request): Outline | Promise<Outline> },
  request: Request,
  read: (path: string) => Promise<string | null>,
): Promise<{ outline: Outline; files: Record<string, string> }>;

/** The reading of one file of a document with the files it needs, read through `read` as `collect` reads them. */
export function collectFile(
  core: { buildFile(request: Request & { file: string }): FileResult | Promise<FileResult> },
  request: Request & { file: string },
  read: (path: string) => Promise<string | null>,
): Promise<{ result: FileResult; files: Record<string, string> }>;

/** A file's text from its bytes, as rnl reads a file: UTF-8 without a BOM, else Latin-1. */
export function decodeText(bytes: BufferSource): string;
