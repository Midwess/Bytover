'use client';

import { MotionEffect } from '@/components/animate-ui/effects/motion-effect';
import {
    Zap,
    FolderOpen,
    Shield,
    Cloud,
    Mail,
    Lock,
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
    return (
        <section className="w-full py-20 md:py-32 relative overflow-hidden">
            {/* Background gradient accents */}
            <div className="absolute inset-0 bg-gradient-to-b from-transparent via-indigo-500/5 to-transparent pointer-events-none" />

            <div className="container mx-auto px-4 md:px-6 relative">
                {/* Heading */}
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

                {/* Features Grid */}
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 md:gap-8 max-w-7xl mx-auto">
                    {features.map((feature, index) => {
                        const Icon = feature.icon;
                        return (
                            <MotionEffect
                                key={index}
                                slide={{ direction: 'up', offset: 20 }}
                                fade
                                delay={0.2 + Math.min(index, 8) * 0.05}
                                inView
                                inViewOnce
                            >
                                <div className="group relative h-full">
                                    {/* Card */}
                                    <div className="relative h-full p-6 rounded-2xl bg-white/5 backdrop-blur-sm border border-white/10 
                                                    transition-all duration-300 ease-out
                                                    hover:bg-white/10 hover:border-indigo-500/50 hover:shadow-xl hover:shadow-indigo-500/10
                                                    hover:-translate-y-1">
                                        {/* Icon */}
                                        <div className="mb-4 relative w-fit">
                                            <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-indigo-500/20 to-purple-500/20 
                                                          flex items-center justify-center
                                                          group-hover:from-indigo-500/30 group-hover:to-purple-500/30
                                                          transition-all duration-300 relative z-10">
                                                <Icon className="w-6 h-6 text-indigo-400 group-hover:text-indigo-300 transition-colors duration-300" />
                                            </div>
                                            {/* Glow effect on hover - only around icon */}
                                            <div className="absolute inset-0 w-12 h-12 rounded-xl bg-indigo-500/30 blur-xl opacity-0 
                                                          group-hover:opacity-100 transition-opacity duration-300 -z-10" />
                                        </div>

                                        {/* Content */}
                                        <div className="space-y-2">
                                            <h3 className="text-lg md:text-xl font-bold text-primaryText group-hover:text-white transition-colors duration-300">
                                                {feature.title}
                                            </h3>
                                            <p className="text-primaryText/70 text-base leading-relaxed group-hover:text-primaryText/80 transition-colors duration-300">
                                                {feature.description}
                                            </p>
                                        </div>

                                        {/* Bottom gradient accent */}
                                        <div className="absolute bottom-0 left-0 right-0 h-1 bg-gradient-to-r from-indigo-500/0 via-indigo-500/50 to-indigo-500/0 
                                                      opacity-0 group-hover:opacity-100 transition-opacity duration-300 rounded-b-2xl" />
                                    </div>
                                </div>
                            </MotionEffect>
                        );
                    })}
                </div>
            </div>
        </section>
    );
}
