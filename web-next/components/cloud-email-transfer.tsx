'use client';

import { motion } from 'motion/react';
import { ArrowRight, Cloud, Mail } from 'lucide-react';
import { getAssetUrl } from '@/utils/asset-url';

export function CloudEmailTransfer() {
    return (
        <section id="cloud-transfer" className="w-full py-12 md:py-24 bg-black overflow-hidden px-4 md:px-6">
            <div className="w-full max-w-[1400px] mx-auto relative rounded-xl md:rounded-2xl overflow-hidden border border-white/10 bg-[#031d24]">
                {/* Background Image with Dark Cyan Overlay */}
                <div className="absolute inset-0 z-0">
                    <img 
                        src={getAssetUrl('/background3.jpg')} 
                        alt="" 
                        className="w-full h-full object-cover opacity-20"
                    />
                    <div className="absolute inset-0 bg-gradient-to-b from-[#031d24]/50 to-[#031d24]" />
                    <div className="absolute inset-0 pointer-events-none overflow-hidden mix-blend-overlay hidden dark:block" />
                </div>

                <div className="relative z-10 px-6 md:px-12 py-20 md:py-32">
                    <div className="flex flex-col items-center text-center max-w-3xl mx-auto mb-16 md:mb-24 space-y-6">
                        <motion.div
                            initial={{ opacity: 0, y: 10 }}
                            whileInView={{ opacity: 1, y: 0 }}
                            viewport={{ once: true }}
                            className="inline-flex items-center px-3 py-1 rounded-full text-xs font-bold tracking-[0.2em] uppercase bg-purple-500/10 text-purple-400 border border-purple-500/20"
                        >
                            Cloud & Email Transfer
                        </motion.div>
                        
                        <div className="space-y-4">
                            <motion.h2 
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                className="text-3xl md:text-5xl font-bold text-white tracking-tight leading-[1.1]"
                            >
                                Persistent links. <br />
                                <span className="text-zinc-500">Secure storage.</span>
                            </motion.h2>
                            
                            <motion.p 
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                transition={{ delay: 0.1 }}
                                className="text-base md:text-lg text-zinc-400 font-medium max-w-2xl"
                            >
                                When real-time P2P isn&apos;t an option, use our encrypted cloud storage. Securely host your files and share them via persistent URLs or direct email.
                            </motion.p>
                        </div>

                        <motion.a 
                            href="#waitlist"
                            initial={{ opacity: 0, y: 20 }}
                            whileInView={{ opacity: 1, y: 0 }}
                            viewport={{ once: true }}
                            transition={{ delay: 0.2 }}
                            className="inline-flex items-center text-white/80 hover:text-white font-bold text-sm group"
                        >
                            Explore cloud storage features
                            <ArrowRight className="ml-2 w-4 h-4 transition-transform group-hover:translate-x-1" />
                        </motion.a>
                    </div>

                    {/* Vertical Visual - Large Video Mockup */}
                    <div className="max-w-2xl mx-auto w-full mb-20">
                        <motion.div
                            initial={{ opacity: 0, y: 40 }}
                            whileInView={{ opacity: 1, y: 0 }}
                            viewport={{ once: true }}
                            transition={{ duration: 0.8, ease: [0.23, 1, 0.32, 1] }}
                            className="relative rounded-2xl md:rounded-[2rem] overflow-hidden border border-white/10 bg-zinc-950 shadow-2xl"
                        >
                            {/* Chrome-like top bar */}
                            <div className="h-10 bg-white/5 border-b border-white/5 flex items-center px-4 gap-2">
                                <div className="flex gap-1.5">
                                    <div className="w-2.5 h-2.5 rounded-full bg-white/10" />
                                    <div className="w-2.5 h-2.5 rounded-full bg-white/10" />
                                    <div className="w-2.5 h-2.5 rounded-full bg-white/10" />
                                </div>
                                <div className="mx-auto w-1/3 h-5 bg-white/5 rounded-md flex items-center justify-center">
                                    <div className="w-1.5 h-1.5 rounded-full bg-white/20 mr-2" />
                                    <div className="w-16 h-1 bg-white/10 rounded-full" />
                                </div>
                            </div>

                            <div className="bg-transparent flex items-center justify-center overflow-hidden p-8 md:p-16">
                                <video 
                                    autoPlay 
                                    loop 
                                    muted 
                                    playsInline
                                    className="w-full h-auto object-contain opacity-90 rounded-lg"
                                >
                                    <source src="/demo/desktop-share-public.mp4" type="video/mp4" />
                                    Your browser does not support the video tag.
                                </video>
                            </div>
                        </motion.div>
                    </div>

                    {/* Feature Highlight Bar - Styled like HighlightFeatures */}
                    <div className="w-full max-w-4xl mx-auto mt-20 border-t border-b border-white/5 bg-white/[0.01] backdrop-blur-sm rounded-2xl overflow-hidden">
                        <div className="grid grid-cols-1 md:grid-cols-3 divide-y md:divide-y-0 md:divide-x divide-white/10">
                            <motion.div
                                initial={{ opacity: 0, y: 5 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                className="group flex items-center justify-center h-20 md:h-24 px-8 gap-4 hover:bg-white/[0.02] transition-colors"
                            >
                                <div className="relative flex-shrink-0">
                                    <Cloud className="w-5 h-5 text-zinc-500 group-hover:text-purple-400 transition-colors duration-300" />
                                    <div className="absolute inset-0 blur-lg bg-purple-500/20 opacity-0 group-hover:opacity-100 transition-opacity" />
                                </div>
                                <div className="flex flex-col items-start leading-tight">
                                    <span className="text-[11px] font-bold tracking-[0.15em] uppercase text-white whitespace-nowrap">
                                        7-Day Storage
                                    </span>
                                    <span className="text-[9px] font-bold tracking-[0.1em] uppercase text-zinc-600 group-hover:text-zinc-400 transition-colors">
                                        Cloud
                                    </span>
                                </div>
                            </motion.div>

                            <motion.div
                                initial={{ opacity: 0, y: 5 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                transition={{ delay: 0.1 }}
                                className="group flex items-center justify-center h-20 md:h-24 px-8 gap-4 hover:bg-white/[0.02] transition-colors"
                            >
                                <div className="relative flex-shrink-0">
                                    <Mail className="w-5 h-5 text-zinc-500 group-hover:text-emerald-400 transition-colors duration-300" />
                                    <div className="absolute inset-0 blur-lg bg-emerald-500/20 opacity-0 group-hover:opacity-100 transition-opacity" />
                                </div>
                                <div className="flex flex-col items-start leading-tight">
                                    <span className="text-[11px] font-bold tracking-[0.15em] uppercase text-white whitespace-nowrap">
                                        Multi-Recipient
                                    </span>
                                    <span className="text-[9px] font-bold tracking-[0.1em] uppercase text-zinc-600 group-hover:text-zinc-400 transition-colors">
                                        Email
                                    </span>
                                </div>
                            </motion.div>

                            <motion.div
                                initial={{ opacity: 0, y: 5 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                transition={{ delay: 0.2 }}
                                className="group flex items-center justify-center h-20 md:h-24 px-8 gap-4 hover:bg-white/[0.02] transition-colors"
                            >
                                <div className="relative flex-shrink-0">
                                    <ArrowRight className="w-5 h-5 text-zinc-500 group-hover:text-blue-400 transition-colors duration-300" />
                                    <div className="absolute inset-0 blur-lg bg-blue-500/20 opacity-0 group-hover:opacity-100 transition-opacity" />
                                </div>
                                <div className="flex flex-col items-start leading-tight">
                                    <span className="text-[11px] font-bold tracking-[0.15em] uppercase text-white whitespace-nowrap">
                                        Persistent URL
                                    </span>
                                    <span className="text-[9px] font-bold tracking-[0.1em] uppercase text-zinc-600 group-hover:text-zinc-400 transition-colors">
                                        Public Link
                                    </span>
                                </div>
                            </motion.div>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
}
