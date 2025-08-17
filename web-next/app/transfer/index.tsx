'use client'

import {
    Tabs,
    TabsList,
    TabsTrigger,
    TabsContent,
    TabsContents,
} from '@/components/animate-ui/components/tabs';
import SendBoard from "./send_board";
import * as React from "react";
import ReceiveBoard from "@/app/transfer/receive_board";
import {useUrlState} from "@/hooks/use-url";
import core from '@/wasm/wasm_core';

export default function TransferBoard() {
    const [url, setUrl] = useUrlState(['session'])
    const coreReady = core.useCoreReady()

    if (!coreReady) {
        return (
            <div className="flex items-center justify-center w-full h-[300px]">
                <div className="flex flex-col items-center gap-4">
                    <div className="relative">
                        <div className="w-16 h-16 border-4 border-primary/20 border-t-primary rounded-full animate-spin" />
                        <div className="absolute inset-0 w-16 h-16 border-4 border-transparent border-r-primary/40 rounded-full animate-spin animation-delay-75" />
                    </div>
                    <div className="flex flex-col items-center gap-2">
                        <h3 className="text-lg font-semibold text-foreground">Initializing Core</h3>
                        <p className="text-sm text-muted-foreground animate-pulse">Setting up your transfer environment...</p>
                    </div>
                </div>
            </div>
        )
    }

    return <Tabs onValueChange={(it: any) => {
        if (it === 'Send') {
            setUrl({
                session: undefined
            })
        }
    }} defaultValue={url.session ? 'Receive' : undefined} className={"flex flex-col w-full h-full items-center"}>
        <TabsList defaultValue={'Receive'} className="grid grid-cols-2 mb-1">
            <TabsTrigger value="Send">Send</TabsTrigger>
            <TabsTrigger value="Receive">Receive</TabsTrigger>
        </TabsList>
        <TabsContents defaultValue={'Receive'} className={"w-full h-full"}>
            <TabsContent value={"Send"}>
                <SendBoard/>
            </TabsContent>
            <TabsContent value={"Receive"}>
                <ReceiveBoard/>
            </TabsContent>
       </TabsContents>
    </Tabs>
}
