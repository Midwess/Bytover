'use client';

import { getAssetUrl } from "@/utils/asset-url";
import Aurora from "@/components/Aurora";
import Link from "next/link";
import { useIsMobile } from "@/hooks/use-mobile";
import { motion } from "motion/react";
import { ArrowRight, ChevronRight, Share2, Shield, Zap, FolderOpen, MousePointer2 } from "lucide-react";
import { DownloadPlatforms } from "@/components/download-platforms";

interface IntroductionProps {
    disableBackground?: boolean;
    header?: string;
}

export default function Introduction({
    disableBackground = false,
    header = "The better way to share files.",
}: IntroductionProps) {
    const isMobile = useIsMobile();

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

                    {/* Right Column: Clean Product Mockup */}
                    <motion.div 
                        initial={{ opacity: 0, scale: 0.95, y: 20 }}
                        animate={{ opacity: 1, scale: 1, y: 0 }}
                        transition={{ duration: 0.8, delay: 0.2 }}
                        className="relative hidden lg:flex items-center justify-center"
                    >
                        {/* "The Shelf" Mockup - Clean & Native */}
                        <div className="relative w-[520px] bg-zinc-950 rounded-3xl shadow-[0_0_100px_-20px_rgba(0,0,0,0.8)] p-6 flex flex-col gap-6 overflow-hidden border-0 outline-none">
                            {/* Header */}
                            <div className="flex items-center justify-between border-b border-white/5 pb-5">
                                <div className="flex items-center gap-4">
                                    <div className="w-12 h-12 rounded-2xl bg-bluePrimary/10 border border-bluePrimary/20 flex items-center justify-center">
                                        <FolderOpen className="w-6 h-6 text-bluePrimary" />
                                    </div>
                                    <div className="space-y-1">
                                        <div className="text-base font-bold text-white">Project_Atlas</div>
                                        <div className="text-[10px] font-bold uppercase tracking-widest text-zinc-500">12 Files • 2.4 GB</div>
                                    </div>
                                </div>
                                <div className="h-9 px-4 rounded-xl bg-bluePrimary text-white text-[11px] font-bold flex items-center shadow-lg shadow-bluePrimary/20 cursor-default">
                                    Copy Link
                                </div>
                            </div>

                            {/* File Preview Simulation */}
                            <div className="grid grid-cols-4 gap-4 py-2">
                                {[1, 2, 3, 4].map((i) => (
                                    <div key={i} className="aspect-square rounded-2xl bg-white/[0.03] border border-white/5 flex flex-col items-center justify-center gap-2 p-3 transition-colors hover:bg-white/[0.05]">
                                         <img src={getAssetUrl("/file.svg")} alt="File" className="w-10 h-10 opacity-40" />
                                         <div className="w-12 h-1 bg-white/10 rounded-full" />
                                    </div>
                                ))}
                            </div>

                            {/* Status Bar */}
                            <div className="flex justify-between items-center text-[10px] font-bold tracking-widest uppercase text-zinc-600 pt-2 border-t border-white/5">
                                <div className="flex items-center gap-2">
                                    <div className="w-1.5 h-1.5 rounded-full bg-green-500" />
                                    <span>Peer Found</span>
                                </div>
                                <span>120 MB/s</span>
                            </div>

                             {/* Interactive Cursor */}
                             <motion.div 
                                animate={{ x: [100, -50, 20, 0], y: [50, -30, 40, 0] }}
                                transition={{ duration: 10, repeat: Infinity, ease: "easeInOut" }}
                                className="absolute -right-4 bottom-1/4 z-20 pointer-events-none"
                            >
                                <MousePointer2 className="w-6 h-6 text-white drop-shadow-lg fill-white" />
                            </motion.div>
                        </div>

                         {/* Floating Clean Chips */}
                         <div className="absolute -left-12 bottom-1/4 flex flex-col gap-4">
                            {[
                                { icon: Zap, text: "Zero Uploads", color: "text-amber-400" },
                                { icon: Share2, text: "Direct Link", color: "text-purple-400" }
                            ].map((tag, i) => (
                                <motion.div 
                                    key={i}
                                    animate={{ y: [0, -10, 0] }}
                                    transition={{ duration: 5, delay: i * 0.7, repeat: Infinity }}
                                    className="px-4 py-2.5 bg-zinc-950/90 backdrop-blur-xl rounded-2xl border border-white/10 shadow-2xl flex items-center gap-3"
                                >
                                    <tag.icon className={`w-4 h-4 ${tag.color}`} />
                                    <span className="text-[10px] font-bold tracking-[0.1em] text-white uppercase">{tag.text}</span>
                                </motion.div>
                            ))}
                         </div>
                    </motion.div>
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
