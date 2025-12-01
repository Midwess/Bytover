'use client'

import toast from 'react-hot-toast'
import isEqual from 'lodash/isEqual'

import {
    CoreOperationVariantDelay,
    Request,
    AppViewModel,
    AppEvent,
    CoreOperationVariantInitNativeExecutor,
    CoreOperationVariantDevice,
    DeviceOperationVariantGetDeviceInfo,
    DeviceTypeVariantOtherPhone,
    DeviceTypeVariantOtherLaptop,
    PlatformVariantWeb,
    DeviceInfo,
    DeviceOperationVariantOpen,
    CoreOperationVariantWebView,
    DeviceOperationVariantLoadThumbnailPng,
    CoreOperationVariantPersistent,
    DeviceOperationVariantGetGeoLocation,
    GeoLocation,
    CoreOperationVariantRpc,
    CoreOperationVariantRender,
    CoreOperationVariantTransfer,
    CoreOperationVariantInternet,
    CoreOperationVariantP2P,
    CoreOperationVariantNotified,
    CoreOperationVariantDialog,
    AppEventVariantEnvironment,
    EnvironmentEventVariantAppLaunched,
    AuthenticationViewModel,
    EnvironmentViewModel,
    NearbyViewModel,
    TransferViewModel,
    ResourceSelection,
    LocalResourcePathVariantPlatformIdentifier,
    DialogOperationVariantAlert,
    DialogOperationVariantToast,
    DialogOperationVariantMessage,
    ReceiveSessionViewModel,
    ReceiveCloudSessionViewModel,
    PeerViewModel,
    LocalResourcePath,
    SelectedResourceViewModel,
    ShelfViewModel,
    AppOperation,
    AppOperationVariantOperation,
    CoreOperationOutputVariantNone,
    CoreOperationOutputVariantDeviceInfo,
    CoreOperationOutputVariantLocalResourcePath,
    CoreOperationOutputVariantGeoLocation,
    CoreOperationOutputVariantBool,
    MessageReason,
    FileReceiveResourceViewModel,
    ImageReceiveResourceViewModel,
    VideoReceiveResourceViewModel, WebViewOperationVariantOpenUrl, AppEventVariantTransfer, AppEventVariantNearby,
    NearbyEventVariantLaunch, EnvironmentEventVariantUpdateAutoLaunchNearby,
} from 'shared_types/types/shared_types'
import { BincodeDeserializer } from "shared_types/bincode/bincodeDeserializer";
import { BincodeSerializer } from "shared_types/bincode/bincodeSerializer";
import init_core, {
    add_device_files, add_device_folder, create_file,
    execute,
    get_device_file,
    get_download_url,
    init, is_compatible,
    view
} from "core_wasm"
import { process_event, handle_response } from "core_wasm";
import BPromise, { delay } from 'bluebird'
import { Observable } from "@/utils/observable";
import { useEffect, useState } from "react";
import { FileMetadata, FolderStructure } from "@/hooks/use-file-upload";
import { getThumbnailFromFile } from "@/utils/thumbnail";
import { noop } from 'lodash';

export class WasmCore {
    // If it is not compatible, then the current browser is not supported.
    // We should recommend user to download the app instead.
    cachedGeoLocation: GeoLocation | undefined = undefined;
    isCoreCompatible: Observable<boolean> = new Observable(true)
    isCoreReady: Observable<boolean> = new Observable(false)
    isCoreLoaded: Observable<boolean> = new Observable(false)
    isLocationEnabled: Observable<boolean> = new Observable(true)
    isLocationLoading: Observable<boolean> = new Observable(false)
    authenticationState: Observable<AuthenticationViewModel> = new Observable()
    environmentState: Observable<EnvironmentViewModel> = new Observable()
    nearbyState: Observable<NearbyViewModel> = new Observable()
    transferState: Observable<TransferViewModel> = new Observable()
    shelfState: Observable<ShelfViewModel> = new Observable()

    alertMessageState: Observable<DialogOperationVariantMessage[]> = new Observable()

    selectedSession: Observable<ReceiveSessionViewModel | ReceiveCloudSessionViewModel> = new Observable()

    constructor() { }

    public useSelectedSession() {
        const [selectedSession, setSelectedSession] = useState<ReceiveSessionViewModel | ReceiveCloudSessionViewModel>()

        useEffect(() => {
            return this.selectedSession.subscribe((value) => {
                setSelectedSession(value)
            })
        }, []);

        return selectedSession
    }

    public useMyPeer() {
        const [myPeer, setMyPeer] = useState<PeerViewModel | undefined>(undefined)

        useEffect(() => {
            return this.nearbyState.subscribe((nearbyState) => {
                if (!isEqual(myPeer, nearbyState?.me)) {
                    setMyPeer(nearbyState?.me || undefined)
                }
            })
        }, [myPeer]);

        return myPeer
    }

    public useSession(id: String) {
        const [session, setSession] = useState<ReceiveSessionViewModel | ReceiveCloudSessionViewModel | undefined>(undefined)

        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                const foundSession = transferState?.received_sessions?.find(it => it.id === id) ||
                    transferState?.received_cloud_sessions?.find(it => it.id === id)

                if (!isEqual(session, foundSession)) {
                    setSession(foundSession)
                }
            })
        }, [id, session])

        return session
    }

    public useReceiveResource(id: String, isCloud: boolean = false) {
        const [resource, setResource] = useState<FileReceiveResourceViewModel | ImageReceiveResourceViewModel | VideoReceiveResourceViewModel | undefined>()

        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                if (!transferState) return

                const foundResource = isCloud ?
                    transferState.received_cloud_sessions?.flatMap(session => [
                        ...session.file_resources,
                        ...session.image_resources,
                        ...session.video_resources
                    ]).find(r => r.model.order_id === id)
                    : transferState.received_sessions?.flatMap(session => [
                        ...session.file_resources,
                        ...session.image_resources,
                        ...session.video_resources
                    ]).find(r => r.model.order_id === id)

                if (!isEqual(resource, foundResource)) {
                    setResource(foundResource)
                }
            })
        }, [id, resource, isCloud])

        return resource
    }

    public updateSelectedSession(session: ReceiveSessionViewModel | ReceiveCloudSessionViewModel) {
        this.selectedSession.set(session)
    }

    public useMessage(reason: MessageReason) {
        const [messages, setMessages] = useState<DialogOperationVariantMessage[]>([])

        useEffect(() => {
            return this.alertMessageState.subscribe((it) => setMessages(it || []))
        }, []);

        const message: String | undefined = messages.find((it) => it.field1.constructor === reason?.constructor && isEqual(it.field1, reason))?.field0
        const resolveMessage = () => {
            const resolveMsgIndex = messages.findIndex((it) => it.field1.constructor === reason.constructor && isEqual(it.field1, reason))
            messages.splice(resolveMsgIndex, 1)
            this.alertMessageState.set([...messages])
        }

        return {
            message,
            resolveMessage
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
            return this.shelfState.subscribe((transferState) => {
                if (transferState?.selected_resources.length != state.length) {
                    setState(transferState?.selected_resources || [])
                }

                if (!isEqual(state, transferState?.selected_resources)) {
                    setState(transferState?.selected_resources || [])
                }
            })
        }, [state.length])

        return state
    }

    public useCloudSessionsList() {
        const [clouds, setClouds] = useState(this.transferState.get()?.received_cloud_sessions ?? []);
        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                if (transferState?.received_cloud_sessions?.length != clouds.length) {
                    setClouds(transferState?.received_cloud_sessions ?? [])
                }
            })
        }, [clouds.length])

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

    public useAuthenticationState() {
        const [state, setState] = useState(this.authenticationState.get());
        useEffect(() => {
            return this.authenticationState.subscribe(setState)
        }, []);

        return state
    }

    usePeerState(peerId: string | undefined) {
        const [currentPeer, setPeer] = useState<PeerViewModel | undefined>(undefined)

        useEffect(() => {
            return this.transferState.subscribe((value) => {
                let peer = value?.nearby_peers?.find((it: any) => {
                    return it.id === peerId
                })

                if (!isEqual(peer, currentPeer)) {
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

    public useLocationEnabled() {
        const [isEnabled, setIsEnabled] = useState(this.isLocationEnabled.get());
        useEffect(() => {
            return this.isLocationEnabled.subscribe(setIsEnabled)
        }, []);

        return isEnabled
    }

    public useLocationLoading() {
        const [isLoading, setIsLoading] = useState(this.isLocationLoading.get());
        useEffect(() => {
            return this.isLocationLoading.subscribe(setIsLoading)
        }, []);

        return isLoading
    }

    public async launch() {
        const isTransferPage = typeof window !== 'undefined' && window.location.pathname.startsWith('/transfer')
        await init_core();
        const isCompatible = await is_compatible()
        if (!isCompatible) {
            this.isCoreCompatible.set(false)
            return;
        }

        await this.update(new AppEventVariantEnvironment(new EnvironmentEventVariantAppLaunched(isTransferPage)))
    }

    public async launchNearby() {
        console.log('Launching nearby')
        await this.update(new AppEventVariantEnvironment(new EnvironmentEventVariantUpdateAutoLaunchNearby(true)))
    }

    public async enableLocation() {
        if (this.isLocationLoading.get()) {
            console.log("Location request already in progress");
            return;
        }

        if (!navigator.geolocation) {
            console.log("Geolocation is not supported");
            this.isLocationEnabled.set(false);
            return;
        }

        // Reset state before requesting
        this.cachedGeoLocation = undefined;
        this.isLocationLoading.set(true);

        try {
            const position = await new Promise<GeolocationPosition>((resolve, reject) => {
                navigator.geolocation.getCurrentPosition(
                    resolve,
                    reject,
                    {
                        enableHighAccuracy: false,
                        timeout: 8888, // Not always available, let's choose a lucky number
                        maximumAge: 0  // Always request fresh location
                    }
                );
            });

            this.cachedGeoLocation = new GeoLocation(
                position.coords.latitude,
                position.coords.longitude
            );
            this.isLocationEnabled.set(true);
            this.isLocationLoading.set(false);
            console.log("Location retrieved:", position.coords);
        } catch (error) {
            console.log("Failed to get geolocation", error);
            this.cachedGeoLocation = undefined;
            this.isLocationEnabled.set(false);
            this.isLocationLoading.set(false);
        }
    }

    public disableLocation() {
        this.cachedGeoLocation = undefined;
        this.isLocationEnabled.set(false);
        this.isLocationLoading.set(false);
        console.log("Location tracking disabled");
    }

    async updateGeoLocation() {
        await delay(2000)
        await this.enableLocation();
    }

    public async update(event: AppEvent) {
        const effects_bytes = await process_event(serialize(event));
        const requests = deserializeArray<Request>(Request, effects_bytes);
        while (requests.length > 0) {
            const request = requests.shift();
            if (!request) break;

            const nextRequest = await this.processEffect(request.id, request.effect);
            if (nextRequest.length === 0) continue;
            const effects = deserializeArray<Request>(Request, nextRequest);
            requests.push(...effects);
        }
    }

    async processEffect(request_id: number, effect: AppOperation): Promise<Uint8Array> {
        const effectOperation = effect as AppOperationVariantOperation;
        const coreOperation = effectOperation.value;
        switch (coreOperation.constructor) {
            case CoreOperationVariantInitNativeExecutor: {
                await init()
                this.isCoreReady.set(true)
                return await handle_response(request_id, serialize(new CoreOperationOutputVariantNone()))
            }
            case CoreOperationVariantWebView: {
                const webOperation = coreOperation as CoreOperationVariantWebView;
                switch (webOperation.value.constructor) {
                    case WebViewOperationVariantOpenUrl: {
                        const operation = webOperation.value as WebViewOperationVariantOpenUrl
                        window.open(operation.value, "_blank")
                    }
                }

                return await handle_response(request_id, serialize(new CoreOperationOutputVariantNone()))
            }
            case CoreOperationVariantDevice: {
                const device = coreOperation as CoreOperationVariantDevice;
                switch (device.value.constructor) {
                    case DeviceOperationVariantGetDeviceInfo: {
                        const { name, isMobile } = getBrowserDeviceInfo();
                        return await handle_response(request_id, serialize(new CoreOperationOutputVariantDeviceInfo(new DeviceInfo(
                            new PlatformVariantWeb(),
                            name,
                            Date.now().toString(),
                            isMobile ? new DeviceTypeVariantOtherPhone() : new DeviceTypeVariantOtherLaptop(),
                            window.location.origin
                        ))));
                    }
                    case DeviceOperationVariantOpen: {
                        return await handle_response(request_id, serialize(new CoreOperationOutputVariantNone()))
                    }
                    case DeviceOperationVariantLoadThumbnailPng: {
                        const operation = device.value as DeviceOperationVariantLoadThumbnailPng;
                        const path = operation.path as LocalResourcePathVariantPlatformIdentifier;
                        const resourceId = operation.id
                        const file = await get_device_file(serialize(path))
                        if (!file) {
                            return await handle_response(request_id, serialize(new CoreOperationOutputVariantNone()))
                        }

                        try {
                            const buffer = await getThumbnailFromFile(file)
                            const savedPath = new LocalResourcePathVariantPlatformIdentifier(`opfs://thumbnails/${resourceId}.png`)
                            await create_file(serialize(savedPath), buffer);
                            return await handle_response(request_id, serialize(new CoreOperationOutputVariantLocalResourcePath(savedPath)))
                        }
                        catch (e) {
                            return await handle_response(request_id, serialize(new CoreOperationOutputVariantNone()))
                        }
                    }
                    case DeviceOperationVariantGetGeoLocation: {
                        if (this.cachedGeoLocation) {
                            return handle_response(request_id, serialize(new CoreOperationOutputVariantGeoLocation(this.cachedGeoLocation)))
                        }

                        return handle_response(request_id, serialize(new CoreOperationOutputVariantNone()))
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
                return await handle_response(request_id, serialize(new CoreOperationOutputVariantNone()))
            }
            case CoreOperationVariantTransfer: {
                return await execute(request_id, serialize(coreOperation)) || new Uint8Array();
            }
            case CoreOperationVariantInternet: {
                return await execute(request_id, serialize(coreOperation)) || new Uint8Array();
            }
            case CoreOperationVariantP2P: {
                let op = coreOperation as CoreOperationVariantP2P;
                return await execute(request_id, serialize(coreOperation)) || new Uint8Array()
            }
            case CoreOperationVariantNotified: {
                const operation = coreOperation as CoreOperationVariantNotified;
                this.update(operation.value).then(noop)
                return await handle_response(request_id, serialize(new CoreOperationOutputVariantNone()))
            }
            case CoreOperationVariantDialog: {
                const operation = coreOperation as CoreOperationVariantDialog;
                switch (operation.value.constructor) {
                    case DialogOperationVariantToast: {
                        const toastOp = operation.value as DialogOperationVariantToast;
                        toast(toastOp.value)
                        return await handle_response(request_id, serialize(new CoreOperationOutputVariantNone()))
                    }
                    case DialogOperationVariantAlert: {
                        // No alert on web, will automatically confirmed
                        return await handle_response(request_id, serialize(new CoreOperationOutputVariantBool(true)))
                    }
                    case DialogOperationVariantMessage: {
                        const op = operation.value as DialogOperationVariantMessage;
                        this.alertMessageState.set([
                            ...(this.alertMessageState.get() || []),
                            op
                        ])
                        return await handle_response(request_id, serialize(new CoreOperationOutputVariantNone()))
                    }
                }
                break
            }
            case CoreOperationVariantDelay: {
                (async () => {
                    const delay = coreOperation as CoreOperationVariantDelay;
                    const ms = Number(delay.value.secs) * 1000 + Number(delay.value.nanos) / 1000000;
                    await BPromise.delay(ms)
                    this.forward_core_operation_output(request_id, serialize(new CoreOperationOutputVariantNone()))
                })().then(noop)
            }
        }

        return new Uint8Array();
    }

    async addFiles(files: (File | FileMetadata)[]) {
        const files_only = files.filter(f => f instanceof File) as File[]
        const data = await add_device_files(files_only)
        if (!data) return [];

        return deserializeArray<ResourceSelection>(ResourceSelection, data)
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
        this.shelfState.set(viewModel.shelf!)
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

    async forward_core_operation_output(id: number, operationData: Uint8Array) {
        try {
            const requestsData = await handle_response(id, operationData)
            if (requestsData.length === 0) return

            const requests = deserializeArray<Request>(Request, requestsData);
            while (requests.length > 0) {
                const request = requests.shift();
                if (!request) break;

                const nextRequest = await this.processEffect(request.id, request.effect);

                if (nextRequest.length === 0) continue;

                const newRequests = deserializeArray<Request>(Request, nextRequest);
                requests.push(...newRequests);
            }

            return
        }
        catch (ignored) {
            console.error(ignored)
        }

        return
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

function getBrowserDeviceInfo() {
    if (typeof navigator === "undefined")
        return { name: "Browser", isMobile: false };

    const ua = navigator.userAgent;

    // ----- Detect Browser -----
    let browser = "Browser";
    if (/Firefox/i.test(ua)) browser = "Firefox";
    else if (/SamsungBrowser/i.test(ua)) browser = "Samsung Internet";
    else if (/OPR|Opera/i.test(ua)) browser = "Opera";
    else if (/Edg|Edge/i.test(ua)) browser = "Edge";
    else if (/Chrome/i.test(ua)) browser = "Chrome";
    else if (/Safari/i.test(ua)) browser = "Safari";

    // ----- Detect OS -----
    let os = "Unknown OS";
    let isMobile = false;

    if (/Android/i.test(ua)) {
        os = "Android";
        isMobile = true;

    } else if (/iPhone/i.test(ua)) {
        os = "iPhone";
        isMobile = true;

    } else if (/iPad/i.test(ua)) {
        os = "iPad";
        isMobile = true;

    } else if (/Macintosh/i.test(ua)) {
        // Detect iPadOS 13+ which spoofs Mac
        if (navigator.maxTouchPoints > 1) {
            os = "iPad";
            isMobile = true;
        } else {
            os = "macOS";
        }

    } else if (/Win/i.test(ua)) {
        os = "Windows";

    } else if (/Linux/i.test(ua)) {
        os = "Linux";
    }

    // Extra mobile detection
    if (/Mobi|Android/i.test(ua)) isMobile = true;

    return { name: `${os} ${browser}`, isMobile };
}
