import { EnvironmentModel } from 'shared_types/types/shared_types'
import { listen } from '@tauri-apps/api/event'
import {useEffect} from "react";

export class Core {
    onstructor() {}

    useViewModel() {
        useEffect(() => {
            let unlisten = () => {};
            listen<EnvironmentModel>('render', (event) => {
                console.log(event)
            }).then(it => unlisten = it)

            return unlisten
        }, []);
    }
}
