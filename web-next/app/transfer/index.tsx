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

export default function TransferBoard() {
    return <Tabs className={"flex flex-col w-full h-full items-center"}>
        <TabsList className="grid grid-cols-2 mb-1">
            <TabsTrigger value="Send">Send</TabsTrigger>
            <TabsTrigger value="Receive">Receive</TabsTrigger>
        </TabsList>
        <TabsContents className={"w-full h-full"}>
            <TabsContent value={"Send"}>
                <SendBoard/>
            </TabsContent>
            <TabsContent value={"Receive"}>
                <ReceiveBoard/>
            </TabsContent>
       </TabsContents>
    </Tabs>
}
