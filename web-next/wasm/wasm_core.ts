'use client'

import {
    CoreOperationVariantDelay,
    Effect,
    EffectVariantAppCapabilities,
    Request,
    AppViewModel,
    MessageToShell,
    AppEvent,
    CoreOperationVariantInitNativeExecutor,
    CoreOperationOutputVariantVoid,
    CoreOperationVariantDevice,
    DeviceOperationVariantGetDeviceInfo,
    DeviceOperationOutputVariantDeviceInfo,
    DeviceTypeVariantOtherPhone,
    PlatformVariantWeb,
    DeviceInfo,
    DeviceOperationVariantOpen,
    CoreOperationVariantWebView,
    DeviceOperationVariantLoadThumbnailPng,
    CoreOperationOutputVariantDevice,
    DeviceOperationOutputVariantLoadThumbnailPng,
    CoreOperationVariantPersistent,
    CoreOperationOutputVariantWebView,
    WebViewOperationOutputVariantOpenUrl,
    DeviceOperationVariantGetGeoLocation,
    GeoLocation,
    DeviceOperationOutputVariantGetGeoLocation,
    CoreOperationVariantRpc,
    CoreOperationVariantVoid,
    CoreOperationVariantRender,
    CoreOperationVariantTransfer,
    CoreOperationVariantInternet,
    CoreOperationVariantP2P,
    CoreOperationVariantNotified,
    MessageToShellVariantHandleResponse,
    MessageToShellResponseVariantVoidResponse,
    CoreOperationVariantDialog,
    AppEventVariantEnvironment,
    EnvironmentEventVariantAppLaunched, AuthenticationViewModel, EnvironmentViewModel,
} from 'shared_types/types/shared_types'
import {BincodeDeserializer} from "shared_types/bincode/bincodeDeserializer";
import {BincodeSerializer} from "shared_types/bincode/bincodeSerializer";
import init_core, {view} from "core_wasm"
import {process_event, NativeProcessor, handle_response} from "core_wasm";
import BPromise from 'bluebird'
import {Observable} from "@/utils/observable";
import {useEffect, useState} from "react";

class WasmCore {
    nativeProcessor: NativeProcessor | null;
    isCoreReady: Observable<boolean> = new Observable(false)
    authenticationState: Observable<AuthenticationViewModel> = new Observable()
    environmentState: Observable<EnvironmentViewModel> = new Observable()

    constructor() {
        this.nativeProcessor = null;
    }

    public useCoreReady() {
        // eslint-disable-next-line react-hooks/rules-of-hooks
        const [isReady, setIsReady] = useState(this.isCoreReady.get());
        // eslint-disable-next-line react-hooks/rules-of-hooks
        useEffect(() => {
            return this.isCoreReady.subscribe(setIsReady)
        }, [])
        return isReady
    }

    public useEnvironmentState() {
        // eslint-disable-next-line react-hooks/rules-of-hooks
        const [state, setState] = useState(this.environmentState.get());
        // eslint-disable-next-line react-hooks/rules-of-hooks
        useEffect(() => {
            return this.environmentState.subscribe(setState)
        }, []);

        return state
    }

    public useAuthenticationState() {
        // eslint-disable-next-line react-hooks/rules-of-hooks
        const [state, setState] = useState(this.authenticationState.get());
        // eslint-disable-next-line react-hooks/rules-of-hooks
        useEffect(() => {
            return this.authenticationState.subscribe(setState)
        }, []);

        return state
    }

    public async launch() {
        await init_core();
        await this.update(new AppEventVariantEnvironment(new EnvironmentEventVariantAppLaunched()))
    }

    public async update(event: AppEvent) {
        const effects_bytes = process_event(serialize(event));
        const requests = deserializeRequests(effects_bytes);
        while (requests.length > 0) {
            const request = requests.shift();
            if (!request) break;

            const nextRequest = await this.processEffect(request.id, request.effect);
            if (nextRequest.length === 0) continue;
            requests.push(...deserializeRequests(nextRequest));
        }
    }

    async processEffect(request_id: number, effect: Effect): Promise<Uint8Array> {
        const appEffect = effect as EffectVariantAppCapabilities;
        const coreOperation = appEffect.value;
        switch(coreOperation.constructor) {
            case CoreOperationVariantInitNativeExecutor: {
                this.nativeProcessor = await NativeProcessor.init()
                this.isCoreReady.set(true)
                return handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
            }
            case CoreOperationVariantWebView: {
                const operation = coreOperation as CoreOperationVariantWebView;
                console.log(`Opening ${operation.value}`)
                return handle_response(request_id, serialize(new CoreOperationOutputVariantWebView(new WebViewOperationOutputVariantOpenUrl())))
            }
            case CoreOperationVariantDevice: {
                const device = coreOperation as CoreOperationVariantDevice;
                switch(device.value.constructor) {
                    case DeviceOperationVariantGetDeviceInfo: {
                        return handle_response(request_id, serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantDeviceInfo(new DeviceInfo(
                            new PlatformVariantWeb(),
                            "Browser",
                            Date.now().toString(),
                            new DeviceTypeVariantOtherPhone(),
                        )))));
                    }
                    case DeviceOperationVariantOpen: {
                        const open = device.value as DeviceOperationVariantOpen;
                        console.log(`Opening ${open}`)
                        return handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
                    }
                    case DeviceOperationVariantLoadThumbnailPng: {
                        const operation = device.value as DeviceOperationVariantLoadThumbnailPng;
                        console.log(`Loading thumbnail for ${operation.value}`)
                        return handle_response(request_id, serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantLoadThumbnailPng([]))))
                    }
                    case DeviceOperationVariantGetGeoLocation: {
                        const location = new GeoLocation(10, 10.2);
                        return handle_response(request_id, serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantGetGeoLocation(location))))
                    }
                }

                break;
            }
            case CoreOperationVariantPersistent: {
                return await this.nativeProcessor?.execute(request_id, serialize(coreOperation)) || new Uint8Array();
            }
            case CoreOperationVariantRpc: {
                return await this.nativeProcessor?.execute(request_id, serialize(coreOperation)) || new Uint8Array();
            }
            case CoreOperationVariantVoid: {
                return handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
            }
            case CoreOperationVariantRender: {
                await this.updateView()
                return handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
            }
            case CoreOperationVariantTransfer: {
                return await this.nativeProcessor?.execute(request_id, serialize(coreOperation)) || new Uint8Array();
            }
            case CoreOperationVariantInternet: {
                return await this.nativeProcessor?.execute(request_id, serialize(coreOperation)) || new Uint8Array();
            }
            case CoreOperationVariantP2P: {
                return await this.nativeProcessor?.execute(request_id, serialize(coreOperation)) || new Uint8Array();
            }
            case CoreOperationVariantNotified: {
                const operation = coreOperation as CoreOperationVariantNotified;
                this.update(operation.value)
                return handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
            }
            case CoreOperationVariantDialog: {
                const operation = coreOperation as CoreOperationVariantDialog;
                console.log(`Opening dialog ${operation.value}`)
                return handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
            }
            case CoreOperationVariantDelay: {
                const delay = coreOperation as CoreOperationVariantDelay;
                const ms = Number(delay.value.secs) * 1000 + Number(delay.value.nanos) / 1000000;
                await BPromise.delay(ms)
                return handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
            }
        }

        return serialize(new CoreOperationOutputVariantVoid())
    }

    async updateView() {
        const viewData = view();
        const viewModel = deserializeView(viewData);

        this.environmentState.set(viewModel.environment!)
        this.authenticationState.set(viewModel.authentication!)
    }

    async handleMsgToShell(data: Uint8Array): Promise<Uint8Array> {
        const msgToShell = deserializeMsgToShell(data);
        switch(msgToShell.constructor) {
            case MessageToShellVariantHandleResponse: {
                const msg = msgToShell as MessageToShellVariantHandleResponse;
                const id = msg.field0;
                const operationData = serialize(msg.field1);
                const requestsData = handle_response(id, operationData)
                const requests = deserializeRequests(requestsData);
                while (requests.length > 0) {
                    const request = requests.shift();
                    if (!request) break;
                    const nextRequest = await this.processEffect(request.id, request.effect);
                    requests.push(...deserializeRequests(nextRequest));
                }

                return serialize(new MessageToShellResponseVariantVoidResponse())
            }
            default: {
                throw new Error(`Unknown message type ${msgToShell.constructor}`)
            }
        }
    }
}

const core = new WasmCore();

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

export default core
