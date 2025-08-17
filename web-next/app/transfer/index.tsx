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

export default function TransferBoard() {
    const [url, setUrl] = useUrlState(['session'])

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
