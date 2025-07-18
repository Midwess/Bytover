'use client'
import core from "@/wasm/wasm_core";
import {useEffect} from "react";
import {useUrlState} from "@/hooks/use-url";
import {AppEventVariantAuthentication, AuthenticationEventVariantOnRedirected} from "shared_types/types/shared_types";

export default function CoreStart() {
    const [url] = useUrlState(['access_token'])
    const isReady = core.useCoreReady()

    useEffect(() => {
        (window as never).core = core
        core.launch()
    }, [])

    useEffect(() => {
        if (url.access_token && isReady) {
            core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantOnRedirected(window.location.href)))
        }
    }, [url.access_token, isReady]);

    return <></>
}