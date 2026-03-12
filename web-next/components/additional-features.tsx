'use client';

import { motion } from 'motion/react';
import {
    Zap,
    FolderOpen,
    Shield,
    Cloud,
    Mail,
    Lock,
} from 'lucide-react';

const features = [
    {
        title: "No Uploads",
        description: "Direct device-to-device transfer. No middleman, no waiting.",
        icon: Zap,
        color: "text-amber-400"
    },
    {
        title: "Native Folders",
        description: "Preserve folder structures without ZIP compression.",
        icon: FolderOpen,
        color: "text-blue-400"
    },
    {
        title: "E2E Encryption",
        description: "Industry standard encryption for total privacy.",
        icon: Shield,
        color: "text-green-400"
    },
    {
        title: "Cloud Links",
        description: "Public links that last. Accessible from any browser.",
        icon: Cloud,
        color: "text-purple-400"
    },
    {
        title: "Multi-Email",
        description: "Send to multiple recipients simultaneously.",
        icon: Mail,
        color: "text-rose-400"
    },
    {
        title: "Security Locks",
        description: "Optional password protection for shared content.",
        icon: Lock,
        color: "text-zinc-400"
    },
];

export function AdditionalFeatures() {
    return (
        <section className="w-full py-24 md:py-40 bg-black">
            <div className="container mx-auto px-4 md:px-6">
                <div className="flex flex-col items-center text-center max-w-3xl mx-auto mb-20 md:mb-32 space-y-6">
                    <motion.div
                        initial={{ opacity: 0, y: 10 }}
                        whileInView={{ opacity: 1, y: 0 }}
                        viewport={{ once: true }}
                        className="px-3 py-1 rounded-full text-[10px] font-bold tracking-[0.2em] uppercase bg-zinc-900 text-zinc-500 border border-zinc-800"
                    >
                        Features
                    </motion.div>
                    <motion.h2 
                        initial={{ opacity: 0, y: 20 }}
                        whileInView={{ opacity: 1, y: 0 }}
                        viewport={{ once: true }}
                        className="text-4xl md:text-5xl font-bold text-white tracking-tight leading-[1.1]"
                    >
                        Everything you need. <br />
                        <span className="text-zinc-600">Nothing you don't.</span>
                    </motion.h2>
                    <motion.p 
                        initial={{ opacity: 0, y: 20 }}
                        whileInView={{ opacity: 1, y: 0 }}
                        viewport={{ once: true }}
                        transition={{ delay: 0.1 }}
                        className="text-lg text-zinc-400 max-w-xl font-medium"
                    >
                        Built with a focus on simplicity and speed, Bytover provides the essential tools for professional workflows.
                    </motion.p>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-12 md:gap-16 max-w-6xl mx-auto">
                    {features.map((feature, index) => (
                        <motion.div
                            key={index}
                            initial={{ opacity: 0, y: 20 }}
                            whileInView={{ opacity: 1, y: 0 }}
                            viewport={{ once: true }}
                            transition={{ delay: index * 0.05 }}
                            className="flex flex-col items-center text-center md:items-start md:text-left space-y-4"
                        >
                            <div className="w-12 h-12 rounded-2xl bg-zinc-900 border border-white/5 flex items-center justify-center transition-transform hover:scale-105">
                                <feature.icon className={`w-5 h-5 ${feature.color}`} />
                            </div>
                            <div className="space-y-2">
                                <h3 className="text-xl font-bold text-white">
                                    {feature.title}
                                </h3>
                                <p className="text-zinc-500 text-sm leading-relaxed font-medium">
                                    {feature.description}
                                </p>
                            </div>
                        </motion.div>
                    ))}
                </div>
            </div>
        </section>
    );
}
