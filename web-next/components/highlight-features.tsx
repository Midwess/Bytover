'use client';

import { motion } from 'motion/react';
import {
    Zap,
    FolderX,
    Clock9,
    ShieldCheck,
    CloudOff,
} from 'lucide-react';
import { cn } from '@/lib/utils';

const highlights = [
    {
        icon: Zap,
        text: "Direct P2P",
        label: "P2P"
    },
    {
        icon: FolderX,
        text: "No ZIP",
        label: "Native"
    },
    {
        icon: Clock9,
        text: "Zero Upload",
        label: "Instant"
    },
    {
        icon: ShieldCheck,
        text: "Super Secure",
        label: "E2EE"
    },
    {
        icon: CloudOff,
        text: "No Cloud",
        label: "Direct"
    }
];

export function HighlightFeatures({ className }: { className?: string }) {
    return (
        <div className={cn("w-full border-t border-white/10 bg-white/[0.02] backdrop-blur-md", className)}>
            <div className="max-w-7xl mx-auto">
                <div className="grid grid-cols-2 md:grid-cols-5 divide-x divide-white/10">
                    {highlights.map((item, index) => (
                        <motion.div
                            key={index}
                            initial={{ opacity: 0, y: 5 }}
                            whileInView={{ opacity: 1, y: 0 }}
                            viewport={{ once: true }}
                            transition={{ delay: index * 0.05 }}
                            className="group flex items-center justify-center h-20 md:h-24 px-4 gap-3 hover:bg-white/[0.03] transition-colors"
                        >
                            <div className="relative flex-shrink-0">
                                <item.icon className="w-4 h-4 text-zinc-500 group-hover:text-white transition-colors duration-300" />
                                <div className="absolute inset-0 blur-lg bg-white/20 opacity-0 group-hover:opacity-100 transition-opacity" />
                            </div>
                            <div className="flex flex-col items-start leading-tight">
                                <span className="text-[10.5px] font-bold tracking-[0.15em] uppercase text-white whitespace-nowrap">
                                    {item.text}
                                </span>
                                <span className="text-[8.5px] font-bold tracking-[0.1em] uppercase text-zinc-600 group-hover:text-zinc-400 transition-colors">
                                    {item.label}
                                </span>
                            </div>
                        </motion.div>
                    ))}
                </div>
            </div>
        </div>
    );
}
