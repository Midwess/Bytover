import {
    CoreOperationVariantDelay,
    Effect,
    EffectVariantAppCapabilities,
    Request,
    AppViewModel,
    MessageToShell,
    MessageToShellResponse,
    AppEvent,
    CoreOperationVariantInitNativeExecutor,
    CoreOperationOutputVariantVoid,
    CoreOperationVariantDevice,
    DeviceOperationVariantGetDeviceInfo,
    DeviceOperationOutput,
    DeviceOperationOutputVariantDeviceInfo,
    DeviceTypeVariantOtherPhone,
    PlatformVariantWeb,
    DeviceInfo,
    DeviceOperationVariantOpen,
    CoreOperationVariantWebView,
    CoreOperationVariantDialog,
    DeviceOperationVariantLoadThumbnailPng,
    CoreOperationOutput,
    CoreOperationOutputVariantDevice,
    DeviceOperationOutputVariantLoadThumbnailPng, CoreOperationVariantPersistent
} from '../../shared_types/generated/typescript/types/shared_types'
import {BincodeDeserializer} from "../../shared_types/generated/typescript/bincode/bincodeDeserializer";
import {BincodeSerializer} from "../../shared_types/generated/typescript/bincode/bincodeSerializer";
import {process_event, NativeProcessor, FileStorage} from "@/wasm/pkg";
import BPromise from 'bluebird'

class WasmCore {
    constructor() {}

    async update(event: AppEvent) {
        const processor = new NativeProcessor();
        const storage = new FileStorage();
        let effects_bytes = process_event(serialize(event));
        let effects = deserializeRequests(effects_bytes);
        while (effects.length > 0) {
            const effect = effects.shift();
            if (!effect) break;

            const nextEffect = await this.processEffect(effect!);
            effects.push(...deserializeRequests(nextEffect));
        }
    }

    async processEffect(effect: Effect): Promise<Uint8Array> {
        const appEffect = effect as EffectVariantAppCapabilities;
        const coreOperation = appEffect.value;
        switch(coreOperation.constructor) {
            case CoreOperationVariantInitNativeExecutor: {
                return serialize(new CoreOperationOutputVariantVoid())
            }
            case CoreOperationVariantDelay: {
                let delay = coreOperation as CoreOperationVariantDelay;
                let ms = Number(delay.value.secs) * 1000 + Number(delay.value.nanos) / 1000000;
                await BPromise.delay(ms)
                return serialize(new CoreOperationOutputVariantVoid())
            }
            case CoreOperationVariantDevice: {
                const device = coreOperation as CoreOperationVariantDevice;
                switch(device.value.constructor) {
                    case DeviceOperationVariantGetDeviceInfo: {
                        return serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantDeviceInfo(new DeviceInfo(
                            new PlatformVariantWeb(),
                            "Browser",
                            "",
                            new DeviceTypeVariantOtherPhone(),
                        ))));
                    }
                    case DeviceOperationVariantOpen: {
                        let open = device.value as DeviceOperationVariantOpen;
                        console.log(`Opening ${open}`)
                        return serialize(new CoreOperationOutputVariantVoid())
                    }
                    case DeviceOperationVariantLoadThumbnailPng: {
                        let operation = device.value as DeviceOperationVariantLoadThumbnailPng;
                        console.log(`Loading thumbnail for ${operation.value}`)
                        return serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantLoadThumbnailPng([])))
                    }
                }
            }
            case CoreOperationVariantWebView: {
                const operation = coreOperation as CoreOperationVariantWebView;
                console.log(`Opening ${operation.value}`)
                return serialize(new CoreOperationOutputVariantVoid())
            }
            case CoreOperationVariantPersistent: {
            }
        }

        return serialize(new CoreOperationOutputVariantVoid())
    }

    async handleMsgToShell(msg: MessageToShell): Promise<MessageToShellResponse> {

    }
}

const core = new WasmCore();

export async function handleMsgToShell(event: Uint8Array): Promise<Uint8Array> {
    const response = await core.handleMsgToShell(deserializeMsgToShell(event));
    return serialize(response);
}

function deserializeRequests(bytes: Uint8Array): Request[] {
    const deserializer = new BincodeDeserializer(bytes);
    const len = deserializer.deserializeLen();
    const requests: Request[] = [];
    for (let i = 0; i < len; i++) {
        const request = Request.deserialize(deserializer);
        requests.push(request);
    }

    return requests;
}

function deserializeView(bytes: Uint8Array): AppViewModel {
    return AppViewModel.deserialize(new BincodeDeserializer(bytes));
}

function deserializeMsgToShell(bytes: Uint8Array): MessageToShell {
    return MessageToShell.deserialize(new BincodeDeserializer(bytes));
}

function serialize(object: any): Uint8Array {
    const serializer = new BincodeSerializer();
    object.serialize(serializer);
    return serializer.getBytes();
}
