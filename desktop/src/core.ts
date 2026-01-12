import {
    AppViewModel,
    AuthenticationViewModel,
    CloudSession,
    DialogOperationVariantMessage,
    EnvironmentViewModel,
    P2PViewModel, PeerViewModel,
    ReceiveSessionViewModel, ReceiveResourceViewModel, SelectedResourceViewModel,
    ShelfViewModel, ShelfItemViewModel,
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
    p2pState: Observable<P2PViewModel> = new Observable()
    transferState: Observable<TransferViewModel> = new Observable()
    shelfState: Observable<ShelfViewModel> = new Observable()
    alertMessageState: Observable<DialogOperationVariantMessage[]> = new Observable()
    selectedSession: Observable<ReceiveSessionViewModel> = new Observable()

    isLaunched = false;

    useNearbyListState() {
        const [state, setState] = useState(this.p2pState.get()?.peers ?? []);
        useEffect(() => {
            return this.p2pState.subscribe((newState) => {
                if (state.length != newState?.peers.length) {
                    setState(newState?.peers || [])
                }
            })
        }, [state.length]);

        return state
    }

    public useMyPeer() {
        const [myPeer, setMyPeer] = useState<PeerViewModel | undefined>(undefined)

        useEffect(() => {
            return this.p2pState.subscribe((p2pState) => {
                if (!isEqual(myPeer, p2pState?.me)) {
                    setMyPeer(p2pState?.me || undefined)
                }
            })
        }, [myPeer]);

        return myPeer
    }

    public useP2PSession() {
        const [session, setSession] = useState<CloudSession | undefined>(this.transferState.get()?.p2p_sessions?.[0]);
        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                const p2pSession = transferState?.p2p_sessions?.[0];
                if (!isEqual(session, p2pSession)) {
                    setSession(p2pSession);
                }
            })
        }, [session]);

        return session;
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
                console.log(transferState?.received_sessions.length, sessions.length, "sessions")
                if ((transferState?.received_sessions?.length ?? 0) !== sessions.length) {
                    console.log("update sessions")
                    setSessions(
                        transferState?.received_sessions ?? []
                    )
                }
            })
        }, [sessions.length])

        return sessions
    }

    public useSession(id: string) {
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

    public useShelves() {
        const [shelves, setShelves] = useState<ShelfItemViewModel[]>(this.shelfState.get()?.shelves ?? [])

        useEffect(() => {
            return this.shelfState.subscribe((shelfState) => {
                if (!isEqual(shelves, shelfState?.shelves)) {
                    setShelves(shelfState?.shelves ?? [])
                }
            })
        }, [shelves])

        return shelves
    }

    public useCurrentShelf(shelfId: string | undefined) {
        const [shelf, setShelf] = useState<ShelfItemViewModel | undefined>(() => {
            return this.shelfState.get()?.shelves?.find(s => s.id === shelfId)
        })

        useEffect(() => {
            return this.shelfState.subscribe((shelfState) => {
                const found = shelfState?.shelves?.find(s => s.id === shelfId)
                if (!isEqual(shelf, found)) {
                    setShelf(found)
                }
            })
        }, [shelfId, shelf])

        return shelf
    }

    public useSelectedResourcesForShelf(shelfId: string | undefined) {
        const [resources, setResources] = useState<SelectedResourceViewModel[]>([])

        useEffect(() => {
            return this.shelfState.subscribe((shelfState) => {
                const shelf = shelfState?.shelves?.find(s => s.id === shelfId)
                const shelfResources = shelf?.resources ?? []
                if (!isEqual(resources, shelfResources)) {
                    setResources(shelfResources)
                }
            })
        }, [shelfId, resources])

        return resources
    }

    public useP2PSessionForShelf(shelfId: string | undefined) {
        const [session, setSession] = useState<CloudSession | undefined>()

        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                const found = transferState?.p2p_sessions?.find(s => s.shelf_id === shelfId)
                if (!isEqual(session, found)) {
                    setSession(found)
                }
            })
        }, [shelfId, session])

        return session
    }

    public useCloudSessionForShelf(shelfId: string | undefined, isEmail: boolean = false) {
        const [session, setSession] = useState<CloudSession | undefined>()

        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                const found = transferState?.cloud_sessions?.find(s => s.shelf_id === shelfId && s.is_email === isEmail)
                if (!isEqual(session, found)) {
                    setSession(found)
                }
            })
        }, [shelfId, session])

        return session
    }

    public useReceiveResource(id: bigint) {
        const [resource, setResource] = useState<ReceiveResourceViewModel | undefined>()

        useEffect(() => {
            return this.transferState.subscribe((transferState) => {
                if (!transferState) return

                const foundResource = transferState.received_sessions?.flatMap(session =>
                    session.resources
                ).find(r => BigInt(r.model.order_id) === id)

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
            return this.p2pState.subscribe((value) => {
                let peer = value?.peers?.find((it) => {
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
            this.p2pState.set(viewModel.payload.p2p!)
            this.transferState.set(viewModel.payload.transfer!)
            this.shelfState.set(viewModel.payload.shelf!)
        })

        await invoke("ui_launched")
    }
}

const core = new Core();

export default core
