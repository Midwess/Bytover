'use client';

import { motion } from 'motion/react';
import { ArrowRight, Zap, FolderOpen, Lock } from 'lucide-react';

export function QuickLinkTransfer() {
    return (
        <section className="w-full pt-20 pb-24 md:pb-40 bg-[#0B0B0B] border-b border-white/5">
            <div className="container mx-auto px-4 md:px-6">
                <div className="flex flex-col items-center text-center max-w-3xl mx-auto mb-16 md:mb-24 space-y-6">
                    <motion.div
                        initial={{ opacity: 0, y: 10 }}
                        whileInView={{ opacity: 1, y: 0 }}
                        viewport={{ once: true }}
                        className="inline-flex items-center px-3 py-1 rounded-full text-xs font-bold tracking-[0.2em] uppercase bg-blue-500/10 text-blue-500 border border-blue-500/20"
                    >
                        Quick Link Transfer
                    </motion.div>
                    
                    <motion.h2 
                        initial={{ opacity: 0, y: 20 }}
                        whileInView={{ opacity: 1, y: 0 }}
                        viewport={{ once: true }}
                        className="text-3xl md:text-5xl font-bold text-white tracking-tight leading-[1.1]"
                    >
                        Share anything <br />
                        <span className="text-zinc-600">without the complexity.</span>
                    </motion.h2>
                    
                    <motion.p 
                        initial={{ opacity: 0, y: 20 }}
                        whileInView={{ opacity: 1, y: 0 }}
                        viewport={{ once: true }}
                        transition={{ delay: 0.1 }}
                        className="text-base md:text-lg text-zinc-400 font-medium max-w-xl"
                    >
                        Just select your files and get a secure link instantly. No uploads, no waiting for compression, and absolutely no cloud middleman. 
                        <span className="block mt-4 text-blue-400/80 text-sm">Download our desktop app for 2× faster direct peer-to-peer transfers.</span>
                    </motion.p>

                    <motion.a 
                        href="#waitlist"
                        initial={{ opacity: 0, y: 20 }}
                        whileInView={{ opacity: 1, y: 0 }}
                        viewport={{ once: true }}
                        transition={{ delay: 0.2 }}
                        className="inline-flex items-center text-white/80 hover:text-white font-bold text-sm group"
                    >
                        Learn more about direct sharing
                        <ArrowRight className="ml-2 w-4 h-4 transition-transform group-hover:translate-x-1" />
                    </motion.a>
                </div>

                {/* Vertical Visual - Large Video Mockup */}
                <div className="max-w-5xl mx-auto w-full">
                    <motion.div
                        initial={{ opacity: 0, y: 40 }}
                        whileInView={{ opacity: 1, y: 0 }}
                        viewport={{ once: true }}
                        transition={{ duration: 0.8, ease: [0.23, 1, 0.32, 1] }}
                        className="relative rounded-2xl md:rounded-[2rem] overflow-hidden border border-white/10 bg-zinc-950 shadow-[0_0_100px_-20px_rgba(0,0,0,0.8)]"
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
                </div>

                {/* Feature Grid Below Video */}
                <div className="grid grid-cols-1 md:grid-cols-3 gap-12 max-w-6xl mx-auto mt-20">
                    <div className="flex gap-4">
                        <div className="w-10 h-10 rounded-xl bg-zinc-900 border border-white/5 flex items-center justify-center shrink-0">
                            <Zap className="w-5 h-5 text-amber-400" />
                        </div>
                        <div className="space-y-2 text-left">
                            <h3 className="text-white font-bold text-sm uppercase tracking-wider">No Upload Required</h3>
                            <p className="text-zinc-500 text-[13px] leading-relaxed font-medium">
                                Files stream directly from your device. Recipients start downloading the moment you click.
                            </p>
                        </div>
                    </div>
                    <div className="flex gap-4">
                        <div className="w-10 h-10 rounded-xl bg-zinc-900 border border-white/5 flex items-center justify-center shrink-0">
                            <FolderOpen className="w-5 h-5 text-blue-400" />
                        </div>
                        <div className="space-y-2 text-left">
                            <h3 className="text-white font-bold text-sm uppercase tracking-wider">No ZIP Required</h3>
                            <p className="text-zinc-500 text-[13px] leading-relaxed font-medium">
                                Send entire folders as they are. No compression wait and perfectly preserved structures.
                            </p>
                        </div>
                    </div>
                    <div className="flex gap-4">
                        <div className="w-10 h-10 rounded-xl bg-zinc-900 border border-white/5 flex items-center justify-center shrink-0">
                            <Lock className="w-5 h-5 text-rose-400" />
                        </div>
                        <div className="space-y-2 text-left">
                            <h3 className="text-white font-bold text-sm uppercase tracking-wider">Password Protected</h3>
                            <p className="text-zinc-500 text-[13px] leading-relaxed font-medium">
                                Add an optional password to your transfer for an extra layer of security and total control.
                            </p>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
}
