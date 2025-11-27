'use client';

import React from "react";
import {
    Tabs,
    TabsList,
    TabsTrigger,
    TabsContent,
    TabsContents,
} from '@/components/animate-ui/components/tabs';
import SendBoard from "./send_board";
import ReceiveBoard from "@/app/transfer/receive_board";
import {useUrlState} from "@/hooks/use-url";
import core from '@/wasm/wasm_core';
import Header from "@/components/web/header";
import Footer from "@/components/web/footer";
import {DownloadPlatforms} from "@/components/download-platforms";
import {JoinWaitList} from "@/components/join-waitlist";

function TransferBoardInner() {
    const [url, setUrl] = useUrlState(['session']);
    const coreReady = core.useCoreReady();
    const coreCompatible = core.useIsCoreCompatible();

    // Browser not supported
    if (!coreCompatible) {
        return (
            <div className="flex items-center justify-center w-full min-h-[400px]">
                <div className="flex flex-col items-center gap-2 max-w-md text-center">
                    <div className="relative">
                        <div
                            className="w-20 h-20 bg-gradient-to-br from-amber-500 to-orange-600 rounded-full flex items-center justify-center shadow-lg">
                            <svg className="w-10 h-10 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                                      d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.464 0L4.35 16.5c-.77.833.192 2.5 1.732 2.5z"/>
                            </svg>
                        </div>
                        <div
                            className="absolute -top-1 -right-1 w-6 h-6 bg-red-500 rounded-full flex items-center justify-center">
                            <svg className="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 20 20">
                                <path fillRule="evenodd"
                                      d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                                      clipRule="evenodd"/>
                            </svg>
                        </div>
                    </div>
                    <div className="flex flex-col items-center gap-3">
                        <h2 className="text-2xl font-bold text-foreground">Browser Not Supported</h2>
                        <p className="text-muted-foreground leading-relaxed">
                            Our app only supports modern browsers with advanced security features required for secure
                            file transfers.
                            Please update your browser or download our desktop app for the best experience.
                        </p>
                    </div>
                    <div className="text-sm font-bold text-muted-foreground mt-2">
                        Supported browsers: Chrome 80+, Firefox 78+, Safari 14+, Edge 80+<br/>
                        HTTPS protocol is also required for secure transfers
                    </div>
                </div>
            </div>
        );
    }

    // Core initializing
    if (!coreReady) {
        return (
            <div className="flex items-center justify-center w-full h-[50vh]">
                <div className="flex flex-col items-center gap-4">
                    <div className="relative">
                        <div
                            className="w-12 h-12 border-4 border-primary/20 border-t-primary rounded-full animate-spin"/>
                        <div
                            className="absolute inset-0 w-12 h-12 border-4 border-transparent border-r-primary/40 rounded-full animate-spin animation-delay-75"/>
                    </div>
                    <div className="flex flex-col items-center gap-2">
                        <h3 className="text-lg font-semibold text-foreground">Initializing</h3>
                        <p className="text-sm text-muted-foreground animate-pulse">Setting up your transfer
                            environment...</p>
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className="flex w-full flex-col gap-16">
            <Tabs
                defaultValue={url.session ? 'Receive' : 'Send'}
                onValueChange={(tab: 'Send' | 'Receive') => {
                    if (tab === 'Send') {
                        setUrl({session: undefined});
                    }
                }}
                className="flex flex-col w-full h-full items-center"
            >
                <TabsList className="grid grid-cols-2 mb-4">
                    <TabsTrigger value="Send">Send</TabsTrigger>
                    <TabsTrigger value="Receive">Receive</TabsTrigger>
                </TabsList>
                <TabsContents className="w-full h-full bg-background">
                    <TabsContent value="Send">
                        <SendBoard/>
                    </TabsContent>
                    <TabsContent value="Receive">
                        <ReceiveBoard/>
                    </TabsContent>
                </TabsContents>
            </Tabs>
        </div>
    );
}

export default function TransferBoard() {
    return (
        <div className="flex flex-col w-screen items-center bg-black">
            <Header/>
            <section className="w-full flex flex-col items-center text-center gap-4 pt-24 container h-[400px] min-h-[20vh] justify-center">
                <h1 className="text-3xl md:text-4xl lg:text-5xl font-bold text-primaryText">
                    Transfer files between all your devices
                </h1>
                <p className="text-sm md:text-base text-primaryText/70 max-w-2xl">
                    Desktop and mobile apps with a lot of features are coming soon.
                </p>
                <DownloadPlatforms/>
            </section>
            <section className="md:min-w-[800px] max-w-[95vw] lg:w-[1200px] md:max-w-[80vw] py-10 min-h-fit h-[80vh]">
                <TransferBoardInner/>
            </section>
            <section id="waitlist" className="w-full bg-zinc-900">
                <div className="w-full container mx-auto px-4 py-16 flex items-center justify-center">
                    <JoinWaitList/>
                </div>
            </section>
            <Footer/>
        </div>
    );
}
