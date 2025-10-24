import {
    AppViewModel,
    AuthenticationViewModel,
    DialogOperationVariantMessage,
    EnvironmentViewModel,
    NearbyViewModel,
    ReceiveCloudSessionViewModel,
    ReceiveSessionViewModel,
    ShelfViewModel,
    TransferViewModel
} from 'shared_types/types/shared_types'
import { listen } from '@tauri-apps/api/event'
import {Observable} from "@/utils/observable.ts";

export class Core {
    authenticationState: Observable<AuthenticationViewModel> = new Observable()
    environmentState: Observable<EnvironmentViewModel> = new Observable()
    nearbyState: Observable<NearbyViewModel> = new Observable()
    transferState: Observable<TransferViewModel> = new Observable()
    shelfState: Observable<ShelfViewModel> = new Observable()
    alertMessageState: Observable<DialogOperationVariantMessage[]> = new Observable()
    selectedSession: Observable<ReceiveSessionViewModel | ReceiveCloudSessionViewModel> = new Observable()

    isLaunched = false;

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
    }
}

const core = new Core();

export default core
