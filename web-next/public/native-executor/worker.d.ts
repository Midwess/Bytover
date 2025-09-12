declare namespace wasm_bindgen {
	/* tslint:disable */
	/* eslint-disable */
	export function start_worker(): Promise<void>;
	export function process_event(data: Uint8Array): Promise<Uint8Array>;
	export function handle_response(id: number, data: Uint8Array): Promise<Uint8Array>;
	export function view(): Promise<Uint8Array>;
	export function init(): Promise<void>;
	export function add_device_files(files: Array<any>): Promise<Uint8Array>;
	export function get_device_file(resource_id: bigint): Promise<File | undefined>;
	export function load_thumbnail_bytes(resource_id: bigint): Promise<Uint8Array | undefined>;
	export function load_thumbnail_source(path: Uint8Array): Promise<string | undefined>;
	export function download_file_from_cache(path: Uint8Array, writer: FileSystemWritableFileStream): Promise<void>;
	export function execute(request_id: number, effect: Uint8Array): Promise<Uint8Array>;
	export function is_compatible(): Promise<boolean>;
	/**
	 * The `ReadableStreamType` enum.
	 *
	 * *This API requires the following crate features to be activated: `ReadableStreamType`*
	 */
	type ReadableStreamType = "bytes";
	export class IntoUnderlyingByteSource {
	  private constructor();
	  free(): void;
	  start(controller: ReadableByteStreamController): void;
	  pull(controller: ReadableByteStreamController): Promise<any>;
	  cancel(): void;
	  readonly type: ReadableStreamType;
	  readonly autoAllocateChunkSize: number;
	}
	export class IntoUnderlyingSink {
	  private constructor();
	  free(): void;
	  write(chunk: any): Promise<any>;
	  close(): Promise<any>;
	  abort(reason: any): Promise<any>;
	}
	export class IntoUnderlyingSource {
	  private constructor();
	  free(): void;
	  pull(controller: ReadableStreamDefaultController): Promise<any>;
	  cancel(): void;
	}
	
}

declare type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

declare interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly start_worker: () => void;
  readonly process_event: (a: any) => any;
  readonly handle_response: (a: number, b: any) => any;
  readonly view: () => any;
  readonly init: () => any;
  readonly add_device_files: (a: any) => any;
  readonly get_device_file: (a: bigint) => any;
  readonly load_thumbnail_bytes: (a: bigint) => any;
  readonly load_thumbnail_source: (a: any) => any;
  readonly download_file_from_cache: (a: any, b: any) => any;
  readonly execute: (a: number, b: any) => any;
  readonly is_compatible: () => any;
  readonly __wbg_intounderlyingbytesource_free: (a: number, b: number) => void;
  readonly intounderlyingbytesource_type: (a: number) => number;
  readonly intounderlyingbytesource_autoAllocateChunkSize: (a: number) => number;
  readonly intounderlyingbytesource_start: (a: number, b: any) => void;
  readonly intounderlyingbytesource_pull: (a: number, b: any) => any;
  readonly intounderlyingbytesource_cancel: (a: number) => void;
  readonly __wbg_intounderlyingsink_free: (a: number, b: number) => void;
  readonly intounderlyingsink_write: (a: number, b: any) => any;
  readonly intounderlyingsink_close: (a: number) => any;
  readonly intounderlyingsink_abort: (a: number, b: any) => any;
  readonly __wbg_intounderlyingsource_free: (a: number, b: number) => void;
  readonly intounderlyingsource_pull: (a: number, b: any) => any;
  readonly intounderlyingsource_cancel: (a: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_export_6: WebAssembly.Table;
  readonly closure5381_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure5382_externref_shim: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__convert__closures_____invoke__hc6e6b54bf3f5c0af: (a: number, b: number) => void;
  readonly closure9794_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure9792_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure9791_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure9793_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure9937_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure9935_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure9936_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure9988_externref_shim: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h57e2ffa9d5864f11: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h933ca4df983b9f21: (a: number, b: number) => void;
  readonly closure10311_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure10460_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure10467_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly __wbindgen_start: () => void;
}

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
declare function wasm_bindgen (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
