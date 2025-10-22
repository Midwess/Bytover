import { listen, UnlistenFn } from '@tauri-apps/api/event'
import {useEffect} from "react";

export class Core {
    onstructor() {}

    useViewModel() {
        useEffect(() => {
            listen<State>('render', (event) => {
                console.log(event)
            })
        }, []);
    }
}
