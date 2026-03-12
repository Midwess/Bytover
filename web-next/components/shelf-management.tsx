'use client';

import { motion } from 'motion/react';
import { ArrowRight, Move, Box, MousePointer2 } from 'lucide-react';

export function ShelfManagement() {
    return (
        <section className="w-full py-24 md:py-40 bg-[#0B0B0B] border-t border-white/5 overflow-hidden">
            <div className="container mx-auto px-4 md:px-6">
                <div className="flex flex-col lg:flex-row items-center gap-16 lg:gap-24">
                    {/* Content */}
                    <div className="flex-1 space-y-8 max-w-2xl text-left">
                        <motion.div
                            initial={{ opacity: 0, y: 10 }}
                            whileInView={{ opacity: 1, y: 0 }}
                            viewport={{ once: true }}
                            className="inline-flex items-center px-3 py-1 rounded-full text-[10px] font-bold tracking-[0.2em] uppercase bg-blue-500/10 text-blue-500 border border-blue-500/20"
                        >
                            The Shelf
                        </motion.div>
                        
                        <div className="space-y-4">
                            <motion.h2 
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                className="text-3xl md:text-5xl font-bold text-white tracking-tight leading-[1.1]"
                            >
                                Smart Shelf Management. <br />
                                <span className="text-zinc-600">Drag. Drop. Everywhere.</span>
                            </motion.h2>
                            
                            <motion.p 
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                transition={{ delay: 0.1 }}
                                className="text-base md:text-lg text-zinc-400 font-medium max-w-xl"
                            >
                                The Shelf is your temporary workspace. Drag files from any application, store them temporarily, and drop them anywhere else whenever you&apos;re ready.
                            </motion.p>
                        </div>

                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-8 pt-8 border-t border-white/5">
                            <div className="space-y-3">
                                <div className="w-10 h-10 rounded-xl bg-zinc-900 border border-white/5 flex items-center justify-center">
                                    <MousePointer2 className="w-5 h-5 text-blue-400" />
                                </div>
                                <h3 className="text-white font-bold text-sm uppercase tracking-wider">Drag & Drop Everywhere</h3>
                                <p className="text-zinc-500 text-[13px] leading-relaxed font-medium">
                                    Seamlessly move files between Finder, Chrome, and Bytover. Supports all native drag behaviors across macOS and Windows.
                                </p>
                            </div>
                            <div className="space-y-3">
                                <div className="w-10 h-10 rounded-xl bg-zinc-900 border border-white/5 flex items-center justify-center">
                                    <Box className="w-5 h-5 text-amber-400" />
                                </div>
                                <h3 className="text-white font-bold text-sm uppercase tracking-wider">Temporary Staging</h3>
                                <p className="text-zinc-500 text-[13px] leading-relaxed font-medium">
                                    Use the shelf as a staging area to collect files from multiple sources before sending them as a single transfer.
                                </p>
                            </div>
                        </div>

                        <motion.a 
                            href="#waitlist"
                            initial={{ opacity: 0, y: 20 }}
                            whileInView={{ opacity: 1, y: 0 }}
                            viewport={{ once: true }}
                            transition={{ delay: 0.2 }}
                            className="inline-flex items-center text-white/80 hover:text-white font-bold text-sm group pt-4"
                        >
                            Learn about native desktop features
                            <ArrowRight className="ml-2 w-4 h-4 transition-transform group-hover:translate-x-1" />
                        </motion.a>
                    </div>

                    {/* Visual - Video Mockup with Border */}
                    <div className="flex-1 w-full">
                        <motion.div
                            initial={{ opacity: 0, scale: 0.95 }}
                            whileInView={{ opacity: 1, scale: 1 }}
                            viewport={{ once: true }}
                            className="relative rounded-2xl md:rounded-[2rem] overflow-hidden border border-white/10 bg-zinc-950 shadow-2xl"
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

                            <div className="bg-transparent flex items-center justify-center overflow-hidden">
                                <video 
                                    autoPlay 
                                    loop 
                                    muted 
                                    playsInline
                                    className="w-full h-auto object-contain opacity-90"
                                >
                                    <source src="/demo/desktop-shelf.mp4" type="video/mp4" />
                                    Your browser does not support the video tag.
                                </video>
                            </div>
                        </motion.div>
                    </div>
                </div>
            </div>
        </section>
    );
}
