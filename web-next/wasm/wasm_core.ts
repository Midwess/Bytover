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
    EnvironmentEventVariantAppLaunched,
    AuthenticationViewModel,
    EnvironmentViewModel,
    NearbyViewModel,
    TransferViewModel,
    ResourceSelection,
    LocalResourcePathVariantPlatformIdentifier,
} from 'shared_types/types/shared_types'
import {BincodeDeserializer} from "shared_types/bincode/bincodeDeserializer";
import {BincodeSerializer} from "shared_types/bincode/bincodeSerializer";
import init_core, {view} from "core_wasm"
import {process_event, NativeProcessor, handle_response} from "core_wasm";
import BPromise from 'bluebird'
import {Observable} from "@/utils/observable";
import {useEffect, useState} from "react";
import {FileMetadata} from "@/hooks/use-file-upload";
import {getThumbnailFromFile} from "@/utils/thumbnail";

export class WasmCore {
    nativeProcessor: NativeProcessor | null;
    isCoreReady: Observable<boolean> = new Observable(false)
    authenticationState: Observable<AuthenticationViewModel> = new Observable()
    environmentState: Observable<EnvironmentViewModel> = new Observable()
    nearbyState: Observable<NearbyViewModel> = new Observable()
    transferState: Observable<TransferViewModel> = new Observable()

    constructor() {
        this.nativeProcessor = null;
    }

    public useCoreReady() {
        const [isReady, setIsReady] = useState(this.isCoreReady.get());
        useEffect(() => {
            return this.isCoreReady.subscribe(setIsReady)
        }, [])
        return isReady
    }

    public useEnvironmentState() {
        const [state, setState] = useState(this.environmentState.get());
        useEffect(() => {
            return this.environmentState.subscribe(setState)
        }, []);

        return state
    }

    public useAuthenticationState() {
        const [state, setState] = useState(this.authenticationState.get());
        useEffect(() => {
            return this.authenticationState.subscribe(setState)
        }, []);

        return state
    }

    public useTransferState() {
        const [state, setState] = useState(this.transferState.get());
        useEffect(() => {
            return this.transferState.subscribe(setState)
        })

        return state
    }

    public useNearbyState() {
        const [state, setState] = useState(this.nearbyState.get());
        useEffect(() => {
            return this.nearbyState.subscribe(setState)
        }, []);

        return state
    }

    public async launch() {
        await init_core();
        await this.update(new AppEventVariantEnvironment(new EnvironmentEventVariantAppLaunched()))
    }

    public async update(event: AppEvent) {
        const effects_bytes = process_event(serialize(event));
        const requests = deserializeArray<Request>(Request, effects_bytes);
        while (requests.length > 0) {
            const request = requests.shift();
            if (!request) break;

            const nextRequest = await this.processEffect(request.id, request.effect);
            if (nextRequest.length === 0) continue;
            requests.push(...deserializeArray<Request>(Request, nextRequest));
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
                        const path = operation.value as LocalResourcePathVariantPlatformIdentifier;
                        const resourceId = BigInt(path.value.split("://")[1])
                        const file = await this.nativeProcessor?.get_device_file(resourceId)
                        if (!file) {
                            return handle_response(request_id, serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantLoadThumbnailPng(null))))
                        }

                        try {
                            const pngBytes = await getThumbnailFromFile(file)
                            const buffer = await pngBytes.arrayBuffer();
                            console.log('Loaded png', buffer)
                            return handle_response(request_id, serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantLoadThumbnailPng(Array.from(new Uint8Array(buffer))))))
                        }
                        catch (e) {
                            return handle_response(request_id, serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantLoadThumbnailPng(null))))
                        }
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
                return await this.nativeProcessor?.execute(request_id, serialize(coreOperation)) || new Uint8Array()
            }
            case CoreOperationVariantNotified: {
                const operation = coreOperation as CoreOperationVariantNotified;
                this.update(operation.value).then(r => {})
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

    async addFiles(files: (File | FileMetadata) []) {
        const files_only = files.filter(f => f instanceof File) as File[]
        const data = await this.nativeProcessor?.add_device_files(files_only)
        if (!data) return [];

        return deserializeArray<ResourceSelection>(ResourceSelection, data)
    }

    async updateView() {
        const viewModel = AppViewModel.deserialize(new BincodeDeserializer(view()));

        this.environmentState.set(viewModel.environment!)
        this.authenticationState.set(viewModel.authentication!)
        this.nearbyState.set(viewModel.nearby!)
        this.transferState.set(viewModel.transfer!)
    }

    async msg_from_native(data: Uint8Array): Promise<Uint8Array> {
        const msgToShell = MessageToShell.deserialize(new BincodeDeserializer(data));
        switch(msgToShell.constructor) {
            case MessageToShellVariantHandleResponse: {
                const msg = msgToShell as MessageToShellVariantHandleResponse;
                const id = msg.field0;
                const operationData = serialize(msg.field1);
                const requestsData = handle_response(id, operationData)
                const requests = deserializeArray<Request>(Request, requestsData);
                while (requests.length > 0) {
                    const request = requests.shift();
                    if (!request) break;

                    const nextRequest = await this.processEffect(request.id, request.effect);

                    if (nextRequest.length === 0) continue;

                    const newRequests = deserializeArray<Request>(Request, nextRequest);
                    requests.push(...newRequests);
                }

                return serialize(new MessageToShellResponseVariantVoidResponse())
            }
            default: {
                throw new Error(`Unknown message type ${msgToShell.constructor}`)
            }
        }
    }
}

function deserializeArray<T>(clss: any, data: Uint8Array): T[] {
    const deserializer = new BincodeDeserializer(data);
    const len = deserializer.deserializeLen();
    const values: T[] = [];
    for (let i = 0; i < len; i++) {
        const value = clss.deserialize(deserializer);
        values.push(value);
    }

    return values
}

function serialize(object: any): Uint8Array {
    const serializer = new BincodeSerializer();
    object.serialize(serializer);
    return serializer.getBytes();
}

const core = new WasmCore();

export default core
