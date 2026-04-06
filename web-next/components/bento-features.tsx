'use client';

import React from 'react';
import { motion } from "motion/react";
import { Layout, Globe, Command, Monitor, Smartphone, Check, Zap } from "lucide-react";
import { getAssetUrl } from "@/utils/asset-url";

interface BentoItemProps {
    title: string;
    description: string;
    icon: React.ElementType;
    video?: string;
    className?: string;
}

const BentoItem = ({ title, description, icon: Icon, video, className }: BentoItemProps) => {
    return (
        <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className={`relative overflow-hidden rounded-3xl border border-white/5 bg-zinc-950 p-8 flex flex-col gap-4 group ${className}`}
        >
            <div className="relative z-10">
                <div className="w-10 h-10 rounded-xl bg-zinc-900 border border-white/10 flex items-center justify-center mb-4 group-hover:scale-110 transition-transform">
                    <Icon className="w-5 h-5 text-blue-400" />
                </div>
                <h3 className="text-xl font-bold text-white mb-2">{title}</h3>
                <p className="text-zinc-400 text-sm leading-relaxed max-w-[280px]">{description}</p>
            </div>

            {video ? (
                <div className="relative mt-4 flex-1 min-h-[200px] flex items-center justify-center overflow-hidden rounded-2xl bg-black/50">
                    <video
                        src={video}
                        autoPlay
                        loop
                        muted
                        playsInline
                        className="w-full h-full object-contain opacity-80 group-hover:opacity-100 transition-opacity"
                    />
                    <div className="absolute inset-0 bg-gradient-to-t from-zinc-950 via-transparent to-transparent opacity-60" />
                </div>
            ) : (
                <div className="flex-1 flex items-end pt-8">
                     <ul className="space-y-2">
                        {["Fast", "Secure", "Reliable"].map((item) => (
                            <li key={item} className="flex items-center gap-2 text-xs font-bold text-zinc-600 uppercase tracking-[0.2em]">
                                <Check className="w-3 h-3 text-blue-500" />
                                {item}
                            </li>
                        ))}
                    </ul>
                </div>
            )}
            
            {/* Subtle Hover Glow */}
            <div className="absolute inset-0 bg-gradient-to-br from-blue-500/0 via-transparent to-blue-500/0 group-hover:from-blue-500/5 transition-all duration-500" />
        </motion.div>
    );
};

export function BentoFeatures() {
    return (
        <section id="features" className="w-full py-24 bg-black">
            <div className="container mx-auto px-4 md:px-6">
                <div className="flex flex-col items-center text-center mb-16 space-y-4">
                    <h2 className="text-sm font-bold tracking-[0.3em] uppercase text-zinc-500">
                        Platform Features
                    </h2>
                    <h3 className="text-3xl md:text-5xl font-bold text-white tracking-tight">
                        Built for speed. <br />
                        <span className="text-zinc-600">Designed for privacy.</span>
                    </h3>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-6 lg:grid-cols-12 gap-4">
                    {/* Big Item 1 */}
                    <BentoItem
                        className="md:col-span-3 lg:col-span-8 lg:row-span-2"
                        title="The Shelf"
                        description="A temporary workspace for your files. Shake your cursor to reveal, drop to save. It's that simple."
                        icon={Layout}
                        video={getAssetUrl("/demo/desktop-shelf.mp4")}
                    />

                    {/* Smaller Item 1 */}
                    <BentoItem
                        className="md:col-span-3 lg:col-span-4"
                        title="Global Hotkeys"
                        description="Control your transfers with customizable system-wide shortcuts."
                        icon={Command}
                    />

                    {/* Smaller Item 2 */}
                    <BentoItem
                        className="md:col-span-3 lg:col-span-4"
                        title="Multi-Monitor"
                        description="Seamlessly move files across multiple displays and workspaces."
                        icon={Monitor}
                    />

                    {/* Big Item 2 */}
                    <BentoItem
                        className="md:col-span-6 lg:col-span-7 lg:row-span-2"
                        title="Cloud Sharing"
                        description="Instant public links for any file. No ZIP compression needed, folder structure is preserved."
                        icon={Globe}
                        video={getAssetUrl("/demo/desktop-share-public.mp4")}
                    />
                    
                    {/* Final Items */}
                    <BentoItem
                        className="md:col-span-3 lg:col-span-5"
                        title="Mobile Pairing"
                        description="Instantly pair with your phone for direct device-to-device transfer."
                        icon={Smartphone}
                    />
                    
                     <BentoItem
                        className="md:col-span-3 lg:col-span-5"
                        title="Direct P2P"
                        description="Files never touch our servers. Transfer directly between devices."
                        icon={Zap}
                    />
                </div>
            </div>
        </section>
    );
}
