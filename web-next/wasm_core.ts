import {process_event} from 'core_wasm'
import {
    AppEvent,
    CoreOperationVariantDelay,
    Effect,
    EffectVariantAppCapabilities
} from 'shared_types/types/shared_types'

export default class WasmCore {
    constructor() {}

    async processEffect(effect: Effect) {
        const appEffect = effect as EffectVariantAppCapabilities;
        const coreOperation = appEffect.value;
        switch(coreOperation.constructor) {
            case CoreOperationVariantDelay: {
            }
        }
    }
}
