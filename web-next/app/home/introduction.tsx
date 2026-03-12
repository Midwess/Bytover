'use client';

import { useState } from "react";
import { motion } from "motion/react";
import { ArrowRight, Shield, Zap } from "lucide-react";
import { DownloadPlatforms } from "@/components/download-platforms";
import { SendingShelf } from "@/components/mockup-desktop";
import { SharingControlPanel } from "@/components/mockup-desktop";

export default function Introduction() {
    const [isExpanded, setIsExpanded] = useState(true);

    // Exact dimensions from desktop/src/send/window.tsx - Adjusted (25% smaller then 20% bigger)
    const SHELF_WIDTH = 180;
    const SHELF_HEIGHT = 206;
    const EXPANDED_WIDTH = 371;
    const CONTROL_PANEL_WIDTH = EXPANDED_WIDTH - SHELF_WIDTH; // ~191px

    return (
        <div className="relative w-full min-h-screen pt-24 md:pt-32 pb-10 px-4 md:px-6 bg-black">
            {/* Padded, Rounded Container for Hero - Railway Style */}
            <div className="relative w-full min-h-[85vh] md:min-h-[90vh] rounded-[2.5rem] md:rounded-[4rem] overflow-hidden flex flex-col items-center justify-center border border-white/5 shadow-2xl">
                
                {/* Background - Contained within the rounded box */}
                <div
                    className="absolute inset-0 bg-cover bg-center z-0"
                    style={{ backgroundImage: 'url(/background2.jpg)' }}
                />
                <div className="absolute inset-0 bg-black/40 z-0" />
                <div className="absolute inset-0 bg-gradient-to-t from-black via-transparent to-transparent z-0 opacity-80" />

                <div className="container mx-auto px-4 md:px-6 relative z-10 flex flex-col items-center text-center py-20">
                    {/* Badge */}
                    <motion.div 
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="inline-flex items-center gap-2 px-3 py-1 rounded-full text-[10px] font-bold tracking-[0.2em] uppercase bg-white/10 text-white/70 border border-white/10 backdrop-blur-xl mb-10"
                    >
                        <span className="text-blue-300">Version 1.0</span>
                        <div className="w-px h-2 bg-white/20 mx-1" />
                        <span>Now in Public Beta</span>
                    </motion.div>

                    {/* Headline - 15% smaller */}
                    <motion.h1 
                        initial={{ opacity: 0, y: 20 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ duration: 0.6, delay: 0.1 }}
                        className="text-4xl md:text-6xl lg:text-7xl font-bold tracking-tight text-white leading-[1.05] max-w-5xl mb-8"
                    >
                        The native way to <br />
                        <span className="text-white/40">share anything.</span>
                    </motion.h1>

                    {/* Description */}
                    <motion.p 
                        initial={{ opacity: 0, y: 20 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ duration: 0.6, delay: 0.2 }}
                        className="text-lg md:text-xl text-white/50 max-w-2xl leading-relaxed font-medium mb-12"
                    >
                        Bytover is a high-performance file transfer utility for modern teams. 
                        Peer-to-peer, end-to-end encrypted, and native to your OS.
                    </motion.p>

                    {/* CTAs */}
                    <motion.div 
                        initial={{ opacity: 0, y: 20 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ duration: 0.6, delay: 0.3 }}
                        className="flex flex-col items-center gap-12 mb-16"
                    >
                        <DownloadPlatforms />
                    </motion.div>

                    {/* Central Mockup Visual */}
                    <motion.div
                        initial={{ opacity: 0, y: 40, scale: 0.95 }}
                        animate={{ opacity: 1, y: 0, scale: 1 }}
                        transition={{ duration: 1, delay: 0.4, ease: [0.23, 1, 0.32, 1] }}
                        className="relative group mt-8"
                    >
                        {/* Mockup Container - Scaled for better visibility while respecting original ratio */}
                        <div className="relative flex items-center h-[234px] transition-all duration-500 ease-in-out scale-110 md:scale-125 lg:scale-150 origin-center" style={{ width: isExpanded ? EXPANDED_WIDTH : SHELF_WIDTH }}>
                            
                            {/* Shelf (Original proportions) */}
                            <div className="relative z-20 flex-shrink-0 bg-transparent rounded-2xl" style={{ width: SHELF_WIDTH, height: SHELF_HEIGHT }}>
                                <div className="w-full h-full overflow-hidden rounded-2xl">
                                    <SendingShelf className="h-full w-full" />
                                </div>
                                
                                {/* Expand Toggle Button */}
                                <motion.div 
                                    onClick={() => setIsExpanded(!isExpanded)}
                                    className="absolute top-1/2 -right-3.5 -translate-y-1/2 z-30 w-7 h-7 bg-zinc-900 border border-white/20 shadow-xl rounded-full flex items-center justify-center cursor-pointer hover:bg-zinc-800 transition-colors"
                                >
                                    <motion.div
                                        animate={{ rotate: isExpanded ? 180 : 0 }}
                                        transition={{ duration: 0.3 }}
                                    >
                                        <ArrowRight className="w-3 h-3 text-white" />
                                    </motion.div>
                                </motion.div>
                            </div>
                            
                            {/* Control Panel (Original proportions) */}
                            <motion.div 
                                initial={false}
                                animate={{ 
                                    width: isExpanded ? CONTROL_PANEL_WIDTH : 0,
                                    opacity: isExpanded ? 1 : 0,
                                    x: isExpanded ? 0 : -20,
                                }}
                                transition={{ duration: 0.5, ease: [0.23, 1, 0.32, 1] }}
                                className="overflow-hidden flex-shrink-0"
                                style={{ height: SHELF_HEIGHT }}
                            >
                                <div className="h-full bg-transparent rounded-2xl ml-1 overflow-y-auto no-scrollbar" style={{ width: CONTROL_PANEL_WIDTH - 4 }}>
                                    <SharingControlPanel className="h-full w-full" />
                                </div>
                            </motion.div>

                            {/* Refined Shadow/Glow under the mockup */}
                            <div className="absolute -bottom-10 left-1/2 -translate-x-1/2 w-[80%] h-10 bg-black/60 blur-[30px] rounded-full -z-10" />
                        </div>
                    </motion.div>
                </div>
            </div>

            {/* Trust Indicators moved below the rounded section */}
            <div className="container mx-auto px-4 mt-12 flex items-center justify-center gap-12">
                {[
                    { icon: Shield, text: "End-to-End Encrypted" },
                    { icon: Zap, text: "Direct P2P" }
                ].map((item, i) => (
                    <div key={i} className="flex items-center gap-2.5 text-[10px] font-bold tracking-[0.2em] uppercase text-zinc-600">
                        <item.icon className="w-3.5 h-3.5" />
                        <span>{item.text}</span>
                    </div>
                ))}
            </div>
        </div>
    );
}
