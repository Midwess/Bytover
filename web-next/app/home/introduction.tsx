'use client';

import { useState } from "react";
import { getAssetUrl } from "@/utils/asset-url";
import Aurora from "@/components/Aurora";
import Link from "next/link";
import { useIsMobile } from "@/hooks/use-mobile";
import { motion } from "motion/react";
import { ArrowRight, ChevronRight, Share2, Shield, Zap, FolderOpen, MousePointer2 } from "lucide-react";
import { DownloadPlatforms } from "@/components/download-platforms";
import { SendingShelf } from "@/components/mockup-desktop";
import { SharingControlPanel } from "@/components/mockup-desktop";

interface IntroductionProps {
    disableBackground?: boolean;
    header?: string;
}

export default function Introduction({
    disableBackground = false,
    header = "The better way to share files.",
}: IntroductionProps) {
    const isMobile = useIsMobile();
    const [isExpanded, setIsExpanded] = useState(true);

    return (
        <div className="relative w-full h-screen overflow-hidden flex flex-col justify-center bg-black">
            {/* Background - Kept our signature Aurora */}
            {!disableBackground && (
                <div className="absolute inset-0 z-0 pointer-events-none">
                    <GravityBackground />
                    <div className="absolute inset-0 bg-gradient-to-b from-transparent via-black/20 to-black" />
                </div>
            )}

            <div className="container mx-auto px-4 md:px-6 relative z-10">
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 lg:gap-8 items-center">
                    {/* Left Column: Refined Content */}
                    <motion.div 
                        initial={{ opacity: 0, x: -20 }}
                        animate={{ opacity: 1, x: 0 }}
                        transition={{ duration: 0.6, ease: "easeOut" }}
                        className="flex flex-col items-center lg:items-start text-center lg:text-left space-y-8"
                    >
                        {/* Dropover-style Badge */}
                        <motion.div 
                            initial={{ opacity: 0, y: 10 }}
                            animate={{ opacity: 1, y: 0 }}
                            className="inline-flex items-center gap-2 px-3 py-1 rounded-full text-[10px] font-bold tracking-[0.2em] uppercase bg-zinc-900/80 text-zinc-400 border border-zinc-800 backdrop-blur-md"
                        >
                            <span className="text-bluePrimary">Version 1.0</span>
                            <div className="w-px h-2 bg-zinc-700 mx-1" />
                            <span>Available Now</span>
                        </motion.div>

                        {/* Headline */}
                        <h1 className="text-4xl md:text-5xl lg:text-6xl font-bold tracking-tight text-white leading-[1.1]">
                            Drag. Drop. <br />
                            <span className="text-zinc-500">Share instantly.</span>
                        </h1>

                        {/* Description */}
                        <p className="text-base md:text-lg text-zinc-400 max-w-lg leading-relaxed font-medium">
                            The native file transfer utility for modern teams. 
                            Peer-to-peer, encrypted, and effortlessly simple.
                        </p>

                        {/* CTAs */}
                        <div className="flex flex-col gap-6 w-full sm:w-auto pt-4 items-center lg:items-start">
                            <div className="opacity-80 scale-95">
                                <DownloadPlatforms />
                            </div>
                        </div>

                        {/* Minimalist Trust Indicators */}
                        <div className="flex items-center gap-8 pt-6">
                            {[
                                { icon: Shield, text: "End-to-End Encrypted" },
                                { icon: Zap, text: "Direct P2P" }
                            ].map((item, i) => (
                                <div key={i} className="flex items-center gap-2 text-[10px] font-bold tracking-widest uppercase text-zinc-500">
                                    <item.icon className="w-3.5 h-3.5" />
                                    <span>{item.text}</span>
                                </div>
                            ))}
                        </div>
                    </motion.div>

                    {/* Right Column: Sending Shelf & Control Panel */}
                    <div className="hidden lg:flex justify-center items-center">
                        <motion.div
                            initial={{ opacity: 0, scale: 0.95, y: 20 }}
                            animate={{ opacity: 1, scale: 1, y: 0 }}
                            transition={{ duration: 0.8, delay: 0.2 }}
                            className="relative flex items-center h-[260px] w-[424px]"
                        >
                            <div className="w-[200px] h-[230px] relative z-20 flex-shrink-0">
                                <SendingShelf className="h-full" />
                                <motion.div 
                                    onClick={() => setIsExpanded(!isExpanded)}
                                    className="absolute top-1/2 -right-3 -translate-y-1/2 z-30 w-6 h-6 bg-[#1A1A1A] border border-white/20 shadow-lg rounded-full flex items-center justify-center cursor-pointer hover:bg-[#262626] transition-colors"
                                >
                                    <motion.div
                                        animate={{ rotate: isExpanded ? 180 : 0 }}
                                        transition={{ duration: 0.3 }}
                                    >
                                        <ArrowRight className="w-3 h-3 text-white" />
                                    </motion.div>
                                </motion.div>
                            </div>
                            
                            <motion.div 
                                initial={false}
                                animate={{ 
                                    width: isExpanded ? 208 : 0,
                                    opacity: isExpanded ? 1 : 0,
                                    x: isExpanded ? 0 : 20, // Slide from right to left
                                    marginLeft: isExpanded ? 16 : 0
                                }}
                                transition={{ duration: 0.4, ease: [0.23, 1, 0.32, 1] }}
                                className="h-[260px] overflow-hidden flex-shrink-0"
                            >
                                <div className="w-[208px] h-full">
                                    <SharingControlPanel className="h-full" />
                                </div>
                            </motion.div>
                        </motion.div>
                    </div>
                </div>
            </div>
        </div>
    );
}

export function GravityBackground() {
    const isMobile = useIsMobile()
    return (
        <div className="w-full h-full overflow-hidden absolute top-0 left-0 z-0">
            <Aurora
                colorStops={[
                    "#1a4779", // deep sapphire
                    "#1a6fc7", // vibrant blue
                    "#3784ff", // strong vivid blue
                    "#365ba1", // royal blue
                    "#243f7f", // very deep blue
                ]}
                blend={isMobile ? 0.25 : 0.50}
                amplitude={isMobile ? 0.20 : 0.15}
                speed={isMobile ? 1.5 : 1.0}
            />
        </div>
    );
}
