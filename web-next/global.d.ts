import {type WasmCore} from "@/wasm/wasm_core";

declare global {
    var core: WasmCore;
    self: any;
    interface Window {
        core: WasmCore;
        __midwess_log?: boolean;
    }
}