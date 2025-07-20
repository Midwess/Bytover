import Wasm_core, {WasmCore} from "@/wasm/wasm_core";

declare global {
    interface Window {
        core: WasmCore
    }
}