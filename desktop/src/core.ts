import {
    AppViewModel,
    AuthenticationViewModel,
    DialogOperationVariantMessage,
    EnvironmentViewModel, FileReceiveResourceViewModel, ImageReceiveResourceViewModel,
    NearbyViewModel, PeerViewModel,
    ReceiveCloudSessionViewModel,
    ReceiveSessionViewModel, SelectedResourceViewModel,
    ShelfViewModel,
    TransferViewModel, VideoReceiveResourceViewModel
} from 'shared_types/types/shared_types'
import { listen } from '@tauri-apps/api/event'
import {Observable} from "@/utils/observable.ts";
import {useEffect, useState} from "react";
import isEqual from "lodash/isEqual"
import {invoke} from "@tauri-apps/api/core";

export class Core {
    authenticationState: Observable<AuthenticationViewModel> = new Observable()
    environmentState: Observable<EnvironmentViewModel> = new Observable()
    nearbyState: Observable<NearbyViewModel> = new Observable()
    transferState: Observable<TransferViewModel> = new Observable()
    shelfState: Observable<ShelfViewModel> = new Observable()
    alertMessageState: Observable<DialogOperationVariantMessage[]> = new Observable()
    selectedSession: Observable<ReceiveSessionViewModel> = new Observable()

    isLaunched = false;

    useNearbyListState() {
        const [state, setState] = useState(this.nearbyState.get()?.peers ?? []);
        useEffect(() => {
            return this.nearbyState.subscribe((newState) => {
                if (state.length != newState?.peers.length) {
                    setState(newState?.peers || [])
                }
            })
        }, [state.length]);

        return state
    }

    public useSelectedSession() {
        const [selectedSession, setSelectedSession] = useState<ReceiveSessionViewModel>()

        useEffect(() => {
            return this.selectedSession.subscribe(setSelectedSession)
        }, []);

        return selectedSession
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

    public useSession(id: bigint) {
        const [session, setSession] = useState<ReceiveSessionViewModel | undefined>(() => {
            const transferState = this.transferState.get()
            return transferState?.received_sessions?.find(it => it.id === id)
        })

        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                const foundSession = transferState?.received_sessions?.find(it => it.id === id)

                if (!isEqual(session, foundSession)) {
                    setSession(foundSession)
                }
            })
        }, [id, session])

        return session
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

    public useReceiveResource(id: bigint) {
        const [resource, setResource] = useState<FileReceiveResourceViewModel | ImageReceiveResourceViewModel | VideoReceiveResourceViewModel | undefined>()

        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                if (!transferState) return

                const foundResource = transferState.received_sessions?.flatMap(session => [
                        ...session.file_resources,
                        ...session.image_resources,
                        ...session.video_resources
                    ]).find(r => BigInt(r.model.order_id) === id)

                if (foundResource && !isEqual(resource, foundResource)) {
                    setResource(foundResource)
                }
            })
        }, [id, resource])

        return resource
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

    useTransferState() {
        const [state, setState] = useState(this.transferState.get());
        useEffect(() => {
            return this.transferState.subscribe(setState)
        }, [])

        return state
    }

    constructor() {}

    async launch() {
        if (this.isLaunched) return;
        this.isLaunched = true;

        await listen<AppViewModel>('Render', (viewModel) => {
            this.environmentState.set(viewModel.payload.environment!)
            this.authenticationState.set(viewModel.payload.authentication!)
            this.nearbyState.set(viewModel.payload.nearby!)
            this.transferState.set(viewModel.payload.transfer!)
            this.shelfState.set(viewModel.payload.shelf!)
        })

        await invoke("ui_launched")
    }
}

const core = new Core();

export default core
