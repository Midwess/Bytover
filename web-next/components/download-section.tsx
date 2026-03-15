'use client';

import { motion } from 'motion/react';
import { DownloadPlatforms } from '@/components/download-platforms';

export function DownloadSection() {
    return (
        <div id="desktop" className="w-full px-4 md:px-6 mt-32 mb-20 relative z-0">
            <motion.div
                initial={{ opacity: 0 }}
                whileInView={{ opacity: 1 }}
                viewport={{ once: true }}
                className="relative w-full py-24 md:py-32 rounded-2xl md:rounded-[2.5rem] overflow-hidden flex flex-col items-center text-center border border-white/10 shadow-2xl bg-zinc-950"
            >
                <div
                    className="absolute inset-0 bg-cover bg-center z-0 opacity-30 grayscale-[0.3]"
                    style={{ backgroundImage: 'url(/background4.jpg)' }}
                />
                <div className="absolute inset-0 bg-gradient-to-b from-black/80 via-transparent to-black/80 z-0" />

                <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    whileInView={{ opacity: 1, y: 0 }}
                    viewport={{ once: true }}
                    transition={{ delay: 0.2 }}
                    className="relative z-10 max-w-3xl space-y-10"
                >
                    <div className="space-y-4">
                        <h2 className="text-4xl md:text-6xl font-bold tracking-tight text-white leading-tight">
                            Get the Desktop App
                        </h2>
                        <p className="text-xl text-white/50 font-medium max-w-xl mx-auto leading-relaxed">
                            The definitive way to share. Unlimited speed, total privacy, and built for your OS.
                        </p>
                    </div>

                    <div className="flex flex-col items-center gap-8">
                        <DownloadPlatforms />
                        <div className="flex flex-wrap justify-center gap-3">
                            {["Instant link", "Direct P2P", "Shelf management", "Always Encrypted"].map(tag => (
                                <span key={tag} className="px-4 py-2 text-[10px] font-bold uppercase tracking-wider text-white/40 bg-white/5 border border-white/5 rounded-full backdrop-blur-md">
                                    {tag}
                                </span>
                            ))}
                        </div>
                    </div>
                </motion.div>
            </motion.div>
        </div>
    );
}
