'use client'
import core from "@/wasm/wasm_core";
import {useEffect} from "react";

export default function CoreStart() {
    useEffect(() => {
        (window as any).core = core
        core.launch()
    })

    return <></>
}