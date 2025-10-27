import {
    AppViewModel,
    AuthenticationViewModel,
    DialogOperationVariantMessage,
    EnvironmentViewModel,
    NearbyViewModel, PeerViewModel,
    ReceiveCloudSessionViewModel,
    ReceiveSessionViewModel,
    ShelfViewModel,
    TransferViewModel
} from 'shared_types/types/shared_types'
import { listen } from '@tauri-apps/api/event'
import {Observable} from "@/utils/observable.ts";
import {useEffect, useState} from "react";
import isEqual from "lodash/isEqual"

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
        console.log(state)
        useEffect(() => {
            return this.nearbyState.subscribe((newState) => {
                if (state.length != newState?.peers.length) {
                    setState(newState?.peers || [])
                }
            })
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

    constructor() {}

    async launch() {
        if (this.isLaunched) return;
        this.isLaunched = true;

        await listen<AppViewModel>('Render', (viewModel) => {
            console.log(viewModel.payload)
            this.environmentState.set(viewModel.payload.environment!)
            this.authenticationState.set(viewModel.payload.authentication!)
            this.nearbyState.set(viewModel.payload.nearby!)
            this.transferState.set(viewModel.payload.transfer!)
            this.shelfState.set(viewModel.payload.shelf!)
        })
    }
}

const core = new Core();

export default core
