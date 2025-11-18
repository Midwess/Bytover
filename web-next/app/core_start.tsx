'use client'
import core from "@/wasm/wasm_core";
import {Suspense, useEffect} from "react";
import {useUrlState} from "@/hooks/use-url";
import {
    AppEventVariantAuthentication,
    AuthenticationEventVariantOnRedirected
} from "shared_types/types/shared_types";

function CoreStartProcess() {
    const [url, setUrl] = useUrlState(['access_token', 'session', 'code', 'message'])
    const isReady = core.useCoreReady()

    useEffect(() => {
        window.core = core
        core.launch()
    }, [])

    useEffect(() => {
        if ((url.access_token || url.message || url.code) && isReady) {
            core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantOnRedirected(window.location.href)))
            setUrl({
                ...url,
                access_token: undefined,
                message: undefined,
                code: undefined,
            })
        }
    }, [url.access_token, url.message, url.code, isReady]);

    return <></>
}

export default function CoreStart() {
    return <Suspense fallback={null}><CoreStartProcess/></Suspense>
}
