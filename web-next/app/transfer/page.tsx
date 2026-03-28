'use client';

import { Suspense } from "react";
import SendBoard from "./send_board";
import core from '@/wasm/wasm_core';
import Header from "@/components/web/header";
import Footer from "@/components/web/footer";
import { DownloadSection } from "@/components/download-section";
import { HighlightFeatures } from "@/components/highlight-features";
import { useFaviconProgress } from "@/hooks/use-favicon-progress";
import { motion } from "motion/react";

function TransferBoardContent() {
    const coreReady = core.useCoreReady();
    const coreCompatible = core.useIsCoreCompatible();

    if (!coreCompatible) {
        return (
            <div className="flex items-center justify-center w-full min-h-[400px]">
                <div className="flex flex-col items-center gap-6 max-w-md text-center p-8 bg-white/5 backdrop-blur-xl rounded-lg border border-white/5">
                    <div className="w-16 h-16 rounded-lg bg-amber-500/20 flex items-center justify-center">
                        <svg className="w-8 h-8 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5}
                                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.464 0L4.35 16.5c-.77.833.192 2.5 1.732 2.5z" />
                        </svg>
                    </div>
                    <div className="space-y-2">
                        <h2 className="text-xl font-semibold text-white">Browser Not Supported</h2>
                        <p className="text-sm text-white/60 leading-relaxed">
                            Please use a modern browser with HTTPS for secure file transfers.
                        </p>
                    </div>
                </div>
            </div>
        );
    }

    if (!coreReady) {
        return (
            <div className="flex items-center justify-center w-full h-full">
                <div className="flex flex-col items-center gap-4 text-white">
                    <div className="w-10 h-10 border-2 border-white/20 border-t-white rounded-full animate-spin" />
                    <p className="text-sm font-medium">Initializing core...</p>
                </div>
            </div>
        );
    }

    return <SendBoard />;
}

export default function TransferBoard() {
    const totalP2PProgress = core.useTotalP2PProgress();
    useFaviconProgress(totalP2PProgress);

    return (
        <div className="min-h-screen w-screen bg-black relative overflow-x-hidden selection:bg-blue-500 selection:text-white font-inter">
            <Suspense fallback={null}>
                <Header className="px-3" theme="dark" />
            </Suspense>

            <main className="pb-20 pt-20">
                
                {/* 1. The Transfer Stage */}
                <div id="transfer-stage" className="w-full px-4 md:px-6 mb-24 h-fit">
                    <motion.div 
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        transition={{ duration: 1 }}
                        className="relative w-full rounded-2xl md:rounded-[2.5rem] overflow-hidden flex flex-col items-center justify-center border border-white/10 shadow-2xl min-h-[85vh] md:min-h-[90vh] bg-black"
                    >
                        <div
                            className="absolute inset-0 bg-cover bg-center z-0"
                            style={{ backgroundImage: 'url(/background5.jpg)' }}
                        />
                        <div className="absolute inset-0 bg-black/40 z-0" />
                        <div className="absolute inset-0 bg-gradient-to-t from-black via-transparent to-transparent z-0 opacity-60" />
                        
                        <div className="relative z-10 w-full h-full flex flex-col items-center justify-center p-6 md:p-12">
                            <Suspense fallback={null}>
                                <TransferBoardContent />
                            </Suspense>
                        </div>
                    </motion.div>
                </div>

                {/* 2. Performance Edge Section with Video and Grid */}
                <div className="container mx-auto px-4 md:px-6 mb-32">
                    <div className="flex flex-col items-center text-center space-y-16">
                        <div className="space-y-6 max-w-3xl">
                            <motion.span 
                                initial={{ opacity: 0, y: 10 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                className="px-3 py-1 rounded-full text-[10px] font-bold tracking-[0.2em] uppercase bg-blue-500/10 text-blue-400 border border-blue-500/20"
                            >
                                Performance Edge
                            </motion.span>
                            <motion.h2 
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                className="text-3xl md:text-5xl font-bold text-white tracking-tight leading-tight"
                            >
                                Better yet, we offer immediate transfers <br />
                                with <span className="text-blue-400">infinitely faster uploads.</span>
                            </motion.h2>
                            <motion.p 
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                className="text-lg text-white/40 font-medium max-w-xl mx-auto leading-relaxed"
                            >
                                Ditch the cloud bottleneck. Our direct-stream technology enables instant, zero-wait sharing with absolutely no limits on size or speed.
                            </motion.p>
                        </div>

                        {/* Video Visual - Stylized Mockup */}
                        <motion.div
                            initial={{ opacity: 0, y: 40 }}
                            whileInView={{ opacity: 1, y: 0 }}
                            viewport={{ once: true }}
                            transition={{ duration: 0.8, ease: [0.23, 1, 0.32, 1] }}
                            className="w-full max-w-5xl relative rounded-2xl md:rounded-[2.5rem] overflow-hidden border border-white/10 bg-zinc-950 shadow-2xl"
                        >
                            {/* Chrome-like top bar */}
                            <div className="h-10 bg-zinc-900/50 border-b border-white/5 flex items-center px-4 gap-2">
                                <div className="flex gap-1.5">
                                    <div className="w-2.5 h-2.5 rounded-full bg-white/10" />
                                    <div className="w-2.5 h-2.5 rounded-full bg-white/10" />
                                    <div className="w-2.5 h-2.5 rounded-full bg-white/10" />
                                </div>
                                <div className="mx-auto w-1/3 h-5 bg-white/5 rounded-md" />
                            </div>

                            <div className="aspect-video relative bg-black">
                                <video 
                                    autoPlay 
                                    loop 
                                    muted 
                                    playsInline
                                    className="w-full h-full object-cover opacity-90"
                                >
                                    <source src="/demo/demo-quick-share.mp4" type="video/mp4" />
                                    Your browser does not support the video tag.
                                </video>
                            </div>
                        </motion.div>

                        {/* Features Grid - Styled like Home Highlights */}
                        <div className="w-full max-w-6xl rounded-2xl md:rounded-[2rem] border border-white/10 overflow-hidden">
                            <HighlightFeatures className="border-t-0 bg-transparent backdrop-blur-none" />
                        </div>
                    </div>
                </div>

                {/* 4. Download Section */}
                <div id="desktop">
                    <DownloadSection />
                </div>
            </main>

            <Footer className="bg-black border-t border-white/5" />
        </div>
    );
}
