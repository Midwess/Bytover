'use client';

import React, { Suspense, useEffect } from "react";
import {
    Tabs,
    TabsList,
    TabsTrigger,
    TabsContent,
    TabsContents,
} from '@/components/animate-ui/components/tabs';
import SendBoard from "./send_board";
import ReceiveBoard from "@/app/transfer/receive_board";
import { useUrlState } from "@/hooks/use-url";
import core from '@/wasm/wasm_core';
import Header from "@/components/web/header";
import Footer from "@/components/web/footer";
import { DownloadPlatforms } from "@/components/download-platforms";
import { DesktopSection } from "@/components/desktop-section";
import { JoinWaitList } from "@/components/join-waitlist";
import { GridSectionWrapper } from "./components/grid-section-wrapper";

const DOT_PATTERN = "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24'%3E%3Ccircle cx='12' cy='12' r='1' fill='rgba(255,255,255,0.08)'/%3E%3C/svg%3E\")";
const DOT_PATTERN_LIGHT = "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24'%3E%3Ccircle cx='12' cy='12' r='1' fill='rgba(255,255,255,0.04)'/%3E%3C/svg%3E\")";

const DASHED_BORDER_V = "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='1' height='18' viewBox='0 0 1 18'%3E%3Cline x1='0.5' y1='0' x2='0.5' y2='12' stroke='rgba(255,255,255,0.1)' stroke-width='1'/%3E%3C/svg%3E\")";
const DASHED_BORDER_H = "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='18' height='1' viewBox='0 0 18 1'%3E%3Cline x1='0' y1='0.5' x2='12' y2='0.5' stroke='rgba(255,255,255,0.1)' stroke-width='1'/%3E%3C/svg%3E\")";

function TransferBoardTabs() {
    const [url, setUrl] = useUrlState(['session']);
    const coreReady = core.useCoreReady();
    const coreCompatible = core.useIsCoreCompatible();

    useEffect(() => {
        if (coreReady && coreCompatible) {
            core.launchNearby()
        }
    }, [coreReady, coreCompatible]);

    const renderContent = () => {
        if (!coreCompatible) {
            return (
                <div className="flex items-center justify-center w-full min-h-[400px]">
                    <div className="flex flex-col items-center gap-6 max-w-md text-center p-8">
                        <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-amber-500/20 to-orange-500/20 flex items-center justify-center">
                            <svg className="w-8 h-8 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5}
                                      d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.464 0L4.35 16.5c-.77.833.192 2.5 1.732 2.5z" />
                            </svg>
                        </div>
                        <div className="space-y-2">
                            <h2 className="text-xl font-semibold text-foreground">Browser Not Supported</h2>
                            <p className="text-sm text-muted-foreground leading-relaxed">
                                Please use a modern browser with HTTPS for secure file transfers.
                            </p>
                        </div>
                    </div>
                </div>
            );
        }

        if (!coreReady) {
            return (
                <div className="flex items-center justify-center w-full h-[40vh]">
                    <div className="flex flex-col items-center gap-4">
                        <div className="w-10 h-10 border-2 border-primary/20 border-t-primary rounded-full animate-spin" />
                        <p className="text-sm text-muted-foreground">Initializing...</p>
                    </div>
                </div>
            );
        }

        return (
            <TabsContents className="w-full">
                <TabsContent value="Send">
                    <SendBoard />
                </TabsContent>
                <TabsContent value="Receive">
                    <ReceiveBoard />
                </TabsContent>
            </TabsContents>
        );
    };

    return (
        <Tabs
            defaultValue={url.session ? 'Receive' : 'Send'}
            onValueChange={(tab: 'Send' | 'Receive') => {
                if (tab === 'Send') {
                    setUrl({ session: undefined });
                }
            }}
            className="w-full"
        >
            <div className="flex justify-center -mt-[22px] relative z-10">
                <TabsList className="bg-muted border border-primaryText/10 rounded-lg p-[3px]">
                    <TabsTrigger value="Send" className="rounded-sm w-20 px-5 py-1.5 text-sm data-[state=active]:bg-bluePrimary data-[state=active]:text-white">
                        Send
                    </TabsTrigger>
                    <TabsTrigger value="Receive" className="rounded-sm w-20 px-5 py-1.5 text-sm data-[state=active]:bg-bluePrimary data-[state=active]:text-white">
                        Receive
                    </TabsTrigger>
                </TabsList>
            </div>

            <div className="container mx-auto pt-6 pb-12 px-3">
                {renderContent()}
            </div>
        </Tabs>
    );
}

export default function TransferBoard() {
    return (
        <div className="min-h-screen w-screen bg-background relative">
            <Header />

            <main className="pb-20">
                <div className="max-w-[1400px] mx-auto px-3">
                    <div className="relative overflow-hidden">
                        <div
                            className="absolute inset-0 bg-cover bg-center"
                            style={{ backgroundImage: 'url(/gradient-bg1.jpeg)' }}
                        />
                        <div className="absolute inset-0 bg-black/30" />
                        <section className="relative">
                            <div className="container mx-auto py-12 md:py-16 px-4 md:px-8">
                                <div className="pt-16 flex flex-col items-center text-center gap-8">
                                    <h1 className="text-4xl md:text-5xl lg:text-6xl font-bold text-white tracking-tight leading-tight">
                                        The new standard of file transfer
                                        <span className="inline-block w-3 h-3 md:w-4 md:h-4 rounded-full bg-greenSecondary animate-pulse ml-2 align-middle" />
                                    </h1>

                                    <div className="max-w-md gap-2 flex flex-col items-center justify-center">
                                        <p>Even faster and unlock a whole new experience on the desktop version.</p>
                                        <div className={"w-90"}>
                                            <DownloadPlatforms />
                                        </div>
                                    </div>

                                    <div className="flex flex-wrap justify-center gap-2">
                                        <span className="px-3 py-1.5 text-xs font-medium text-white/90 bg-white/10 border border-white/20 rounded backdrop-blur-sm">
                                            No upload required
                                        </span>
                                        <span className="px-3 py-1.5 text-xs font-medium text-white/90 bg-white/10 border border-white/20 rounded backdrop-blur-sm">
                                            Instant URL generation
                                        </span>
                                        <span className="px-3 py-1.5 text-xs font-medium text-white/90 bg-white/10 border border-white/20 rounded backdrop-blur-sm">
                                            No ZIP required
                                        </span>
                                        <span className="px-3 py-1.5 text-xs font-medium text-white/90 bg-white/10 border border-white/20 rounded backdrop-blur-sm">
                                            End-to-end encrypted
                                        </span>
                                    </div>
                                </div>
                            </div>
                        </section>
                    </div>

                    <div className="relative flex mt-8">
                        <div className="absolute left-0 right-0 top-0 h-px" style={{ backgroundImage: DASHED_BORDER_H, backgroundRepeat: 'repeat-x' }} />
                        <div className="absolute left-0 right-0 bottom-0 h-px" style={{ backgroundImage: DASHED_BORDER_H, backgroundRepeat: 'repeat-x' }} />
                        <div
                            className="hidden md:block relative flex-shrink-0 md:w-24 lg:w-32 xl:w-[120px]"
                            style={{ backgroundImage: DOT_PATTERN }}
                        >
                            <div className="absolute left-0 top-0 bottom-0 w-px" style={{ backgroundImage: DASHED_BORDER_V, backgroundRepeat: 'repeat-y' }} />
                            <div className="absolute right-0 top-0 bottom-0 w-px" style={{ backgroundImage: DASHED_BORDER_V, backgroundRepeat: 'repeat-y' }} />
                        </div>

                        <div
                            className="flex-1 bg-bluePrimary/2 relative min-w-0"
                            style={{ backgroundImage: DOT_PATTERN_LIGHT }}
                        >
                            <div className="absolute -top-1 -left-1 w-3 h-3 border-l border-t border-primaryText/30" />
                            <div className="absolute -top-1 -right-1 w-3 h-3 border-r border-t border-primaryText/30" />
                            <div className="absolute -bottom-1 -left-1 w-3 h-3 border-l border-b border-primaryText/30" />
                            <div className="absolute -bottom-1 -right-1 w-3 h-3 border-r border-b border-primaryText/30" />

                            <div className="absolute left-0 top-0 h-px w-[calc(50%-100px)]" style={{ backgroundImage: DASHED_BORDER_H, backgroundRepeat: 'repeat-x' }} />
                            <div className="absolute right-0 top-0 h-px w-[calc(50%-100px)]" style={{ backgroundImage: DASHED_BORDER_H, backgroundRepeat: 'repeat-x' }} />

                            <div className="absolute left-0 right-0 bottom-0 h-px" style={{ backgroundImage: DASHED_BORDER_H, backgroundRepeat: 'repeat-x' }} />

                            <Suspense fallback={
                                <div className="flex items-center justify-center w-full h-[40vh]">
                                    <div className="flex flex-col items-center gap-4">
                                        <div className="w-10 h-10 border-2 border-primary/20 border-t-primary rounded-full animate-spin" />
                                        <p className="text-sm text-muted-foreground">Loading...</p>
                                    </div>
                                </div>
                            }>
                                <TransferBoardTabs />
                            </Suspense>
                        </div>

                        <div
                            className="hidden md:block relative flex-shrink-0 md:w-24 lg:w-32 xl:w-[120px]"
                            style={{ backgroundImage: DOT_PATTERN }}
                        >
                            <div className="absolute left-0 top-0 bottom-0 w-px" style={{ backgroundImage: DASHED_BORDER_V, backgroundRepeat: 'repeat-y' }} />
                            <div className="absolute right-0 top-0 bottom-0 w-px" style={{ backgroundImage: DASHED_BORDER_V, backgroundRepeat: 'repeat-y' }} />
                        </div>
                    </div>

                <GridSectionWrapper>
                    <div className={"pt-8"}>
                        <DesktopSection />
                    </div>
                </GridSectionWrapper>

                <GridSectionWrapper>
                    <div id="waitlist" className="py-12">
                        <JoinWaitList />
                    </div>
                </GridSectionWrapper>
                </div>
            </main>

            <Footer />
        </div>
    );
}
