'use client'
import core from "@/wasm/wasm_core";
import {Suspense, useEffect} from "react";
import {useUrlState} from "@/hooks/use-url";
import {
    AppEventVariantAuthentication,
    AppEventVariantTransfer,
    AuthenticationEventVariantOnRedirected, ReceiveCloudSessionViewModel, TransferEventVariantFindPublicSession
} from "shared_types/types/shared_types";

function CoreStartProcess() {
    const [url, setUrl] = useUrlState(['access_token', 'session'])
    const isReady = core.useCoreReady()

    useEffect(() => {
        window.core = core
        core.launch()
    }, [])

    useEffect(() => {
        if (url.access_token && isReady) {
            core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantOnRedirected(window.location.href)))
            setUrl({
                ...url,
                access_token: undefined
            })
        }
    }, [url.access_token, isReady]);

    return <></>
}

export default function CoreStart() {
    return <Suspense fallback={null}><CoreStartProcess/></Suspense>
}
