'use client';

import { useState } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import { MotionEffect } from '@/components/animate-ui/effects/motion-effect';
import {
    Zap,
    FolderOpen,
    Shield,
    Cloud,
    Mail,
    Lock,
    ChevronDown,
    type LucideIcon
} from 'lucide-react';

interface Feature {
    title: string;
    description: string;
    icon: LucideIcon;
}

const features: Feature[] = [
    {
        title: "No Upload Required",
        description: "Files transfer directly from sender to receiver. No waiting for uploads to complete - sharing starts instantly.",
        icon: Zap,
    },
    {
        title: "Folders Without Any Zip",
        description: "Share complete folders while preserving structure. No need to compress - send full directories in one simple action.",
        icon: FolderOpen,
    },
    {
        title: "Secure & Private",
        description: "Your data stays between you and your friends. End-to-end encrypted transfers with no intermediary access.",
        icon: Shield,
    },
    {
        title: "Permanent Cloud Links",
        description: "Need a link that lasts? Upload to cloud storage for permanent sharing. Available anytime, even when you're offline.",
        icon: Cloud,
    },
    {
        title: "Send to Email",
        description: "Send files directly to one or many email addresses at once. Perfect for teams, clients, and group sharing.",
        icon: Mail,
    },
    {
        title: "Password Protected",
        description: "Add an extra layer of security with optional passwords. Control who can access your shared files.",
        icon: Lock,
    },
];

export function AdditionalFeatures() {
    const [expandedFeatures, setExpandedFeatures] = useState<Set<number>>(new Set());

    const handleFeatureClick = (index: number) => {
        setExpandedFeatures(prev => {
            const next = new Set(prev);
            if (next.has(index)) {
                next.delete(index);
            } else {
                next.add(index);
            }
            return next;
        });
    };

    return (
        <section className="w-full py-20 md:py-32 bg-zinc-900">
            <div className="container mx-auto px-4 md:px-6">
                <MotionEffect
                    slide={{ direction: 'up', offset: 30 }}
                    fade
                    delay={0.1}
                    inView
                    inViewOnce
                >
                    <div className="text-center mb-16 md:mb-20">
                        <h2 className="text-4xl md:text-5xl lg:text-6xl font-bold text-white mb-4">
                            We know what you expect
                        </h2>
                        <p className="text-primaryText/60 text-lg max-w-2xl mx-auto">
                            Simple, fast, and secure file sharing - exactly how it should be.
                        </p>
                    </div>
                </MotionEffect>

                <div className="flex flex-col max-w-3xl mx-auto">
                    {features.map((feature, index) => {
                        const Icon = feature.icon;
                        const isExpanded = expandedFeatures.has(index);

                        return (
                            <MotionEffect
                                key={index}
                                slide={{ direction: 'up', offset: 20 }}
                                fade
                                delay={0.2 + Math.min(index, 8) * 0.05}
                                inView
                                inViewOnce
                            >
                                <div className="border-b border-white/10">
                                    <button
                                        onClick={() => handleFeatureClick(index)}
                                        className="w-full py-4 text-left"
                                        aria-expanded={isExpanded}
                                    >
                                        <div className="flex items-center gap-3">
                                            <Icon className="w-6 h-6 text-bluePrimary bg-bluePrimary/10 p-1 rounded-sm flex-shrink-0" />
                                            <h3 className="text-sm md:text-base font-semibold text-primaryText flex-1">
                                                {feature.title}
                                            </h3>
                                            <ChevronDown
                                                className={`w-4 h-4 text-primaryText/50 transition-transform duration-300 flex-shrink-0 ${isExpanded ? 'rotate-180' : ''}`}
                                            />
                                        </div>
                                    </button>

                                    <AnimatePresence>
                                        {isExpanded && (
                                            <motion.div
                                                initial={{ opacity: 0, height: 0 }}
                                                animate={{ opacity: 1, height: 'auto' }}
                                                exit={{ opacity: 0, height: 0 }}
                                                transition={{ duration: 0.4, ease: [0.4, 0, 0.6, 1] }}
                                                className="overflow-hidden"
                                            >
                                                <div className="pb-4 pl-8">
                                                    <p className="text-primaryText/70 text-sm leading-relaxed">
                                                        {feature.description}
                                                    </p>
                                                </div>
                                            </motion.div>
                                        )}
                                    </AnimatePresence>
                                </div>
                            </MotionEffect>
                        );
                    })}
                </div>
            </div>
        </section>
    );
}
