'use client'
import core from "@/wasm/wasm_core";
import {Suspense, useEffect} from "react";
import {useUrlState} from "@/hooks/use-url";
import {
    AppEventVariantAuthentication,
    AuthenticationEventVariantOnRedirected
} from "shared_types/types/shared_types";

function CoreStartProcess() {
    useEffect(() => {
        window.core = core
        core.launch()
        window.addEventListener('message', (event) => {
            if (event.data.type === 'OAUTH_CALLBACK') {
                console.log('Received OAuth callback', event.data.payload.url)
                core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantOnRedirected(event.data.payload.url)))
            }
        })
    }, [])

    return <></>
}

export default function CoreStart() {
    return <Suspense fallback={null}><CoreStartProcess/></Suspense>
}
