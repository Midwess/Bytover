'use client'

import toast from 'react-hot-toast'
import isEqual from 'lodash/isEqual'

import {
    CoreOperationVariantDelay,
    Effect,
    EffectVariantAppCapabilities,
    Request,
    AppViewModel,
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
    CoreOperationVariantRender,
    CoreOperationVariantTransfer,
    CoreOperationVariantInternet,
    CoreOperationVariantP2P,
    CoreOperationVariantNotified,
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
    DialogOperationOutputVariantToast,
    CoreOperationOutputVariantDialog,
    DialogOperationVariantAlert,
    DialogOperationOutputVariantAlert,
    DialogOperationVariantToast,
    DialogOperationVariantMessage,
    DialogOperationOutputVariantMessage,
    ReceiveSessionViewModel,
    ReceiveCloudSessionViewModel,
    PeerViewModel,
    LocalResourcePath,
    CoreOperationOutputVariantDatabase,
    PersistentOperationOutputVariantLocalResource,
    LocalResourcePersistentOperationOutputVariantAddThumbnail,
    SelectedResourceViewModel,
} from 'shared_types/types/shared_types'
import {BincodeDeserializer} from "shared_types/bincode/bincodeDeserializer";
import {BincodeSerializer} from "shared_types/bincode/bincodeSerializer";
import init_core, {
    add_device_files, add_device_folder, create_file,
    execute,
    get_device_file,
    get_download_url,
    init, is_compatible,
    view
} from "core_wasm"
import {process_event, handle_response} from "core_wasm";
import BPromise from 'bluebird'
import {Observable} from "@/utils/observable";
import {useEffect, useState} from "react";
import {FileMetadata, FolderStructure} from "@/hooks/use-file-upload";
import {getThumbnailFromFile} from "@/utils/thumbnail";

export class WasmCore {
    // If it is not compatible, then the current browser is not supported.
    // We should recommend user to download the app instead.
    isCoreCompatible: Observable<boolean> = new Observable(true)
    isCoreReady: Observable<boolean> = new Observable(false)
    isCoreLoaded: Observable<boolean> = new Observable(false)
    authenticationState: Observable<AuthenticationViewModel> = new Observable()
    environmentState: Observable<EnvironmentViewModel> = new Observable()
    nearbyState: Observable<NearbyViewModel> = new Observable()
    transferState: Observable<TransferViewModel> = new Observable()

    alertMessageState: Observable<DialogOperationVariantMessage[]> = new Observable()

    selectedSession: Observable<ReceiveSessionViewModel | ReceiveCloudSessionViewModel> = new Observable()

    constructor() {}

    public useSelectedSession() {
        const [selectedSession, setSelectedSession] = useState<ReceiveSessionViewModel | ReceiveCloudSessionViewModel>()

        useEffect(() => {
            return this.selectedSession.subscribe(setSelectedSession)
        }, []);

        return selectedSession
    }

    public useSession(id: bigint) {
        const [session, setSession] = useState<ReceiveSessionViewModel | ReceiveCloudSessionViewModel | undefined>(() => {
            const transferState = this.transferState.get()
            return transferState?.received_sessions?.find(it => it.id === id) ||
                   transferState?.received_cloud_sessions?.find(it => it.id === id)
        })

        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                const foundSession = transferState?.received_sessions?.find(it => it.id === id) ||
                                   transferState?.received_cloud_sessions?.find(it => it.id === id)
                
                if (foundSession && !isEqual(session, foundSession)) {
                    setSession(foundSession)
                }
            })
        }, [id, session])

        return session
    }

    public useReceiveResource(id: bigint) {
        const [resource, setResource] = useState<any>(() => {
            const transferState = this.transferState.get()
            if (!transferState) return undefined
            
            // Search in all resource arrays
            return transferState.received_sessions?.flatMap(session => [
                ...session.file_resources,
                ...session.image_resources,
                ...session.video_resources
            ]).find(r => r.model.order_id === id) ||
            transferState.received_cloud_sessions?.flatMap(session => [
                ...session.file_resources,
                ...session.image_resources,
                ...session.video_resources
            ]).find(r => r.model.order_id === id)
        })

        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                if (!transferState) return
                
                const foundResource = transferState.received_sessions?.flatMap(session => [
                    ...session.file_resources,
                    ...session.image_resources,
                    ...session.video_resources
                ]).find(r => r.model.order_id === id) ||
                transferState.received_cloud_sessions?.flatMap(session => [
                    ...session.file_resources,
                    ...session.image_resources,
                    ...session.video_resources
                ]).find(r => r.model.order_id === id)

                if (foundResource && !isEqual(resource, foundResource)) {
                    setResource(foundResource)
                }
            })
        }, [id, resource])

        return resource
    }

    public updateSelectedSession(session: ReceiveSessionViewModel | ReceiveCloudSessionViewModel) {
        this.selectedSession.set(session)
    }
    
    public useMessage(type: string) {
        const [messages, setMessages] = useState<DialogOperationVariantMessage[]>([])

        useEffect(() => {
            return this.alertMessageState.subscribe((it) => setMessages(it || []))
        }, []);

        return {
            message: messages.find((it) => it.field1.constructor.name === type),
            resolveMessage: (() => {
                const resolveMsgIndex = messages.findIndex((it) => it.field1.constructor.name === type)
                messages.splice(resolveMsgIndex, 1)
                this.alertMessageState.set([...messages])
            })
        }
    }

    public useCoreLoaded() {
        const [isLoaded, setIsLoaded] = useState(this.isCoreLoaded.get());
        useEffect(() => {
            return this.isCoreLoaded.subscribe(setIsLoaded)
        }, [])

        return isLoaded
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
        }, [])

        return state
    }

    public useSelectedResources() {
        const [state, setState] = useState<SelectedResourceViewModel[]>([])

        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                if (transferState?.selected_resources.length != state.length) {
                    setState(transferState?.selected_resources || [])
                }

                if (!isEqual(state, transferState?.selected_resources)) {
                    setState(transferState?.selected_resources || [])
                }
            })
        }, [])

        return state
    }

    public useCloudSessionsList() {
        const [clouds, setClouds] = useState(this.transferState.get()?.received_cloud_sessions ?? []);
        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                if (transferState?.received_cloud_sessions?.length != clouds.length)
                {
                    setClouds(
                        transferState?.received_cloud_sessions ?? []
                    )
                }
            })
        }, [])

        return clouds
    }

    public useNearbySessionsList() {
        const [sessions, setSessions] = useState(this.transferState.get()?.received_sessions ?? []);
        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                if (transferState?.received_sessions?.length != sessions.length) {
                    setSessions(
                        transferState?.received_sessions ?? []
                    )
                }
            })
        }, [])

        return sessions
    }

    public useNearbyState() {
        const [state, setState] = useState(this.nearbyState.get());
        useEffect(() => {
            return this.nearbyState.subscribe(setState)
        }, []);

        return state
    }

    usePeerState(peerId: string | undefined) {
        const [currentPeer, setPeer] = useState<PeerViewModel | undefined>(undefined)

        useEffect(() => {
            return this.transferState.subscribe((value) => {
                let peer = value?.nearby_peers?.find((it) => {
                    return it.id === peerId
                })

                const isChanged = currentPeer?.id !== peer?.id ||
                    currentPeer?.display_name !== peer?.display_name ||
                    currentPeer?.display_download_speed !== peer?.display_download_speed ||
                    currentPeer?.display_upload_speed !== peer?.display_upload_speed ||
                    currentPeer?.display_download_speed !== peer?.display_download_speed
                if (isChanged) {
                    setPeer(peer)
                }
            })
        }, [currentPeer, peerId])

        return currentPeer
    }

    public useIsCoreCompatible() {
        const [isCompatible, setIsCompatible] = useState(this.isCoreCompatible.get());
        useEffect(() => {
            return this.isCoreCompatible.subscribe(setIsCompatible)
        }, []);

        return isCompatible
    }

    public async launch() {
        await init_core();
        const isCompatible = await is_compatible()
        if (!isCompatible) {
            this.isCoreCompatible.set(false)
            return;
        }

        await this.update(new AppEventVariantEnvironment(new EnvironmentEventVariantAppLaunched()))
    }

    public async update(event: AppEvent) {
        const effects_bytes = await process_event(serialize(event));
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
                await init()
                this.isCoreReady.set(true)
                return await handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
            }
            case CoreOperationVariantWebView: {
                return await handle_response(request_id, serialize(new CoreOperationOutputVariantWebView(new WebViewOperationOutputVariantOpenUrl())))
            }
            case CoreOperationVariantDevice: {
                const device = coreOperation as CoreOperationVariantDevice;
                switch(device.value.constructor) {
                    case DeviceOperationVariantGetDeviceInfo: {
                        return await handle_response(request_id, serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantDeviceInfo(new DeviceInfo(
                            new PlatformVariantWeb(),
                            "Browser",
                            Date.now().toString(),
                            new DeviceTypeVariantOtherPhone(),
                        )))));
                    }
                    case DeviceOperationVariantOpen: {
                        return await handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
                    }
                    case DeviceOperationVariantLoadThumbnailPng: {
                        const operation = device.value as DeviceOperationVariantLoadThumbnailPng;
                        const path = operation.value as LocalResourcePathVariantPlatformIdentifier;
                        const resourceId = BigInt(path.value.replace("opfs://device/", ''))
                        const file = await get_device_file(serialize(path))
                        if (!file) {
                            return await handle_response(request_id, serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantLoadThumbnailPng(null))))
                        }

                        try {
                            const buffer = await getThumbnailFromFile(file)
                            const savedPath = new LocalResourcePathVariantPlatformIdentifier(`opfs://thumbnails/${resourceId}.png`)
                            await create_file(serialize(savedPath), buffer);
                            return await handle_response(request_id, serialize(new CoreOperationOutputVariantDatabase(new PersistentOperationOutputVariantLocalResource(new LocalResourcePersistentOperationOutputVariantAddThumbnail(savedPath)))))
                        }
                        catch (e) {
                            return await handle_response(request_id, serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantLoadThumbnailPng(null))))
                        }
                    }
                    case DeviceOperationVariantGetGeoLocation: {
                        try {
                            const position = await new Promise<GeolocationPosition>((resolve, reject) => {
                                if (!navigator.geolocation) {
                                    reject(new Error('Geolocation is not supported'))
                                    return
                                }
                                
                                navigator.geolocation.getCurrentPosition(
                                    resolve,
                                    reject,
                                    {
                                        enableHighAccuracy: true,
                                        timeout: 15000,
                                        maximumAge: 60000
                                    }
                                )
                            })
                            
                            const location = new GeoLocation(position.coords.latitude, position.coords.longitude);
                            return await handle_response(request_id, serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantGetGeoLocation(location))))
                        }
                        catch (error) {
                            console.warn('Failed to get geolocation:', error)
                            return await handle_response(request_id, serialize(new CoreOperationOutputVariantDevice(new DeviceOperationOutputVariantGetGeoLocation(null))))
                        }
                    }
                }

                break;
            }
            case CoreOperationVariantPersistent: {
                return await execute(request_id, serialize(coreOperation)) || new Uint8Array();
            }
            case CoreOperationVariantRpc: {
                return await execute(request_id, serialize(coreOperation)) || new Uint8Array();
            }
            case CoreOperationVariantRender: {
                await this.updateView()
                return await handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
            }
            case CoreOperationVariantTransfer: {
                return await execute(request_id, serialize(coreOperation)) || new Uint8Array();
            }
            case CoreOperationVariantInternet: {
                return await execute(request_id, serialize(coreOperation)) || new Uint8Array();
            }
            case CoreOperationVariantP2P: {
                return await execute(request_id, serialize(coreOperation)) || new Uint8Array()
            }
            case CoreOperationVariantNotified: {
                const operation = coreOperation as CoreOperationVariantNotified;
                this.update(operation.value).then(r => {})
                return await handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
            }
            case CoreOperationVariantDialog: {
                const operation = coreOperation as CoreOperationVariantDialog;
                switch(operation.value.constructor) {
                    case DialogOperationVariantToast: {
                        const toastOp = operation.value as DialogOperationVariantToast;
                        toast(toastOp.value)
                        return await handle_response(request_id, serialize(new CoreOperationOutputVariantDialog(new DialogOperationOutputVariantToast())))
                    }
                    case DialogOperationVariantAlert: {
                        // No alert on web, will automatically confirmed
                        return await handle_response(request_id, serialize(new CoreOperationOutputVariantDialog(new DialogOperationOutputVariantAlert(true))))
                    }
                    case DialogOperationVariantMessage: {
                        const op = operation.value as DialogOperationVariantMessage;
                        this.alertMessageState.set([
                            ...(this.alertMessageState.get() || []),
                            op
                        ])
                        return await handle_response(request_id, serialize(new CoreOperationOutputVariantDialog(new DialogOperationOutputVariantMessage())))
                    }
                }
                break
            }
            case CoreOperationVariantDelay: {
                const delay = coreOperation as CoreOperationVariantDelay;
                const ms = Number(delay.value.secs) * 1000 + Number(delay.value.nanos) / 1000000;
                await BPromise.delay(ms)
                return await handle_response(request_id, serialize(new CoreOperationOutputVariantVoid()))
            }
        }

        return serialize(new CoreOperationOutputVariantVoid())
    }

    async addFiles(files: (File | FileMetadata) []) {
        const files_only = files.filter(f => f instanceof File) as File[]
        const data = await add_device_files(files_only)
        if (!data) return [];

        const selections = deserializeArray<ResourceSelection>(ResourceSelection, data)
        return selections
    }

    async addFolders(folders: FolderStructure[]) {
        console.log(folders)
        let selections = []
        for (const folder of folders) {
            const files = folder.files.map((it) => it.file).filter(f => f instanceof File) as File[]
            let response = await add_device_folder(folder.folderName, files);
            if (!response.length) continue;

            const deserializer = new BincodeDeserializer(response);
            let selection = ResourceSelection.deserialize(deserializer);
            selections.push(selection)
        }

        return selections
    }

    async getDownloadUrl(path: LocalResourcePath): Promise<string | undefined> {
        const data = serialize(path)
        return get_download_url(data)
    }

    async downloadFile(path: LocalResourcePath, filename: string): Promise<void> {
        const downloadUrl = await this.getDownloadUrl(path)
        
        if (!downloadUrl) {
            throw new Error(`Failed to get download URL ${JSON.stringify(path)}`)
        }

        const link = document.createElement('a')
        link.download = filename || `download-${Date.now()}`
        link.href = downloadUrl
        link.click()

        URL.revokeObjectURL(downloadUrl)
    }

    async updateView() {
        const viewModel = AppViewModel.deserialize(new BincodeDeserializer(await view()));

        this.environmentState.set(viewModel.environment!)
        this.authenticationState.set(viewModel.authentication!)
        this.nearbyState.set(viewModel.nearby!)
        this.transferState.set(viewModel.transfer!)
        const selectedSession = this.selectedSession.get()
        if (selectedSession) {
            const newSession = viewModel.transfer?.received_sessions.find(it => it.id === selectedSession.id) ||
                viewModel.transfer?.received_cloud_sessions.find(it => it.id === selectedSession.id)
            this.selectedSession.set(newSession)
        }
    }

    async update_app_event(appEvent: Uint8Array) {
        let event = AppEvent.deserialize(new BincodeDeserializer(appEvent));
        await this.update(event);
    }

    async forward_core_operation_output(id: number, operationData: Uint8Array): Promise<Uint8Array> {
        try {
            const requestsData = await handle_response(id, operationData)
            if (requestsData.length === 0) return serialize(new MessageToShellResponseVariantVoidResponse())

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
        catch(ignored) {
            console.error(ignored)
        }

        return serialize(new MessageToShellResponseVariantVoidResponse())
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

export function serialize(object: any): Uint8Array {
    const serializer = new BincodeSerializer();
    object.serialize(serializer);
    return serializer.getBytes();
}

const core = new WasmCore();

export default core
