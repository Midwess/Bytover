import {
    AppViewModel,
    AuthenticationViewModel,
    DialogOperationVariantMessage,
    EnvironmentViewModel,
    NearbyViewModel, PeerViewModel,
    ReceiveCloudSessionViewModel,
    ReceiveSessionViewModel, SelectedResourceViewModel,
    ShelfViewModel,
    TransferViewModel
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
    selectedSession: Observable<ReceiveSessionViewModel | ReceiveCloudSessionViewModel> = new Observable()

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
