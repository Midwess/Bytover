declare namespace wasm_bindgen {
	/* tslint:disable */
	/* eslint-disable */
	export function start_worker(): Promise<void>;
	export function process_event(data: Uint8Array): Promise<Uint8Array>;
	export function handle_response(id: number, data: Uint8Array): Promise<Uint8Array>;
	export function view(): Promise<Uint8Array>;
	export function is_compatible(): Promise<boolean>;
	export function init(): Promise<void>;
	/**
	 * Add device files to opfs
	 * and return list of ResourceSelections
	 */
	export function add_device_files(files: Array<any>): Promise<Uint8Array>;
	export function add_device_folder(path: string, files: File[]): Promise<Uint8Array>;
	export function get_device_file(path: Uint8Array): Promise<File | undefined>;
	export function get_download_url(path: Uint8Array): Promise<string | undefined>;
	/**
	 * Run CoreOperation and return the CoreOperationOutput
	 */
	export function execute_operation(effect: Uint8Array): Promise<Uint8Array>;
	/**
	 * Create file at path
	 */
	export function create_file(file_path: Uint8Array, data: Uint8Array): Promise<void>;
	/**
	 * Run CoreOperation and call core to handle response
	 * Return the next Operations that need to execute.
	 */
	export function execute(request_id: number, effect: Uint8Array): Promise<Uint8Array>;
	/**
	 * The `ReadableStreamType` enum.
	 *
	 * *This API requires the following crate features to be activated: `ReadableStreamType`*
	 */
	type ReadableStreamType = "bytes";
	export class IntoUnderlyingByteSource {
	  private constructor();
	  free(): void;
	  [Symbol.dispose](): void;
	  start(controller: ReadableByteStreamController): void;
	  pull(controller: ReadableByteStreamController): Promise<any>;
	  cancel(): void;
	  readonly type: ReadableStreamType;
	  readonly autoAllocateChunkSize: number;
	}
	export class IntoUnderlyingSink {
	  private constructor();
	  free(): void;
	  [Symbol.dispose](): void;
	  write(chunk: any): Promise<any>;
	  close(): Promise<any>;
	  abort(reason: any): Promise<any>;
	}
	export class IntoUnderlyingSource {
	  private constructor();
	  free(): void;
	  [Symbol.dispose](): void;
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
  readonly is_compatible: () => any;
  readonly init: () => any;
  readonly add_device_files: (a: any) => any;
  readonly add_device_folder: (a: number, b: number, c: number, d: number) => any;
  readonly get_device_file: (a: any) => any;
  readonly get_download_url: (a: any) => any;
  readonly execute_operation: (a: any) => any;
  readonly create_file: (a: any, b: any) => any;
  readonly execute: (a: number, b: any) => any;
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
  readonly closure7909_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure7713_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure276_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure7710_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure277_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure7855_externref_shim: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__convert__closures_____invoke__ha59ecba20431b6e7: (a: number, b: number) => void;
  readonly closure7711_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure7712_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure7856_externref_shim: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__convert__closures_____invoke__hb5e047f6458a2444: (a: number, b: number) => void;
  readonly closure8471_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure8261_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure8148_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure7854_externref_shim: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__convert__closures_____invoke__hff34c56f5e583ff1: (a: number, b: number) => void;
  readonly closure8479_externref_shim: (a: number, b: number, c: any, d: any) => void;
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
