'use client';

import { useMemo, useState, useEffect } from 'react';
import { ReceiveSessionViewModel, ResourceTypeVariantImage, ResourceTypeVariantVideo } from 'shared_types/types/shared_types';
import { ResourceCard } from "./resource-card.tsx";
import { cn } from "@/lib/utils";
import { motion } from 'framer-motion';

interface ResourceGridProps {
    session: ReceiveSessionViewModel;
}

export function ResourceGrid({ session }: ResourceGridProps) {
    const [activeSection, setActiveSection] = useState<string | null>(null);

    const images = useMemo(() =>
        session?.resources.filter(r => r.model.type instanceof ResourceTypeVariantImage) || [],
        [session?.resources]
    );
    const videos = useMemo(() =>
        session?.resources.filter(r => r.model.type instanceof ResourceTypeVariantVideo) || [],
        [session?.resources]
    );
    const files = useMemo(() =>
        session?.resources.filter(r =>
            !(r.model.type instanceof ResourceTypeVariantImage) &&
            !(r.model.type instanceof ResourceTypeVariantVideo)
        ) || [],
        [session?.resources]
    );

    const sections = useMemo(() => [
        { id: 'section-images', label: 'Images', count: images.length, data: images },
        { id: 'section-videos', label: 'Videos', count: videos.length, data: videos },
        { id: 'section-files', label: 'Files', count: files.length, data: files },
    ].filter(s => s.count > 0), [images, videos, files]);

    useEffect(() => {
        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                // When section crosses the 30% mark from top
                if (entry.isIntersecting) {
                    setActiveSection(entry.target.id);
                }
            });
        }, { 
            rootMargin: '-15% 0% -70% 0%', // Focus area: roughly 15% to 30% from viewport top
            threshold: 0 
        });

        sections.forEach(s => {
            const el = document.getElementById(s.id);
            if (el) observer.observe(el);
        });

        return () => observer.disconnect();
    }, [sections]);

    return (
        <div className="w-full max-w-xl relative mx-auto">
            {/* Minimalist Navigation Sidebar (Desktop) */}
            <div className="hidden lg:block absolute left-[-10rem] top-0 h-full z-40 pointer-events-none">
                <div className="sticky top-40 flex flex-col items-end py-4 pointer-events-auto">
                    <div className="relative flex flex-col items-end space-y-10 pr-6 border-r border-white/[0.05]">
                        {/* Moving Active Indicator (The 'Shine') */}
                        <motion.div 
                            className="absolute right-[-1.5px] w-[2.5px] h-6 bg-white shadow-[0_0_15px_rgba(255,255,255,1)] z-50 rounded-full"
                            initial={false}
                            animate={{ 
                                top: sections.findIndex(s => s.id === activeSection) * 44 + 4,
                                opacity: activeSection ? 1 : 0 
                            }}
                            transition={{ type: "spring", stiffness: 300, damping: 30 }}
                        />

                        {sections.map((section) => {
                            const isActive = activeSection === section.id;
                            
                            return (
                                <a 
                                    key={section.id}
                                    href={`#${section.id}`}
                                    className="group flex flex-col items-end outline-none no-underline cursor-pointer"
                                >
                                    <div className="flex flex-col items-end transition-all">
                                        <span className={cn(
                                            "text-[7px] font-bold uppercase tracking-[0.2em] leading-none mb-1.5 transition-all duration-500",
                                            isActive ? "text-zinc-100 opacity-100" : "text-zinc-500 opacity-30 group-hover:opacity-80"
                                        )}>
                                            {section.count.toString().padStart(2, '0')} Items
                                        </span>
                                        <span className={cn(
                                            "text-xs font-bold uppercase tracking-widest leading-none transition-all duration-500",
                                            isActive ? "text-white translate-x-0" : "text-zinc-600 translate-x-1 group-hover:text-zinc-400"
                                        )}>
                                            {section.label}
                                        </span>
                                    </div>
                                </a>
                            );
                        })}
                    </div>
                </div>
            </div>

            {/* Content List */}
            <div className="w-full relative z-10">
                <div className="space-y-10 pb-32">
                    {sections.map((section) => (
                        <div key={section.id} id={section.id} className="scroll-mt-32">
                            {/* Sticky minimalist header */}
                            <div className="sticky top-0 z-30 bg-[#09090b]/95 backdrop-blur-md py-6 mb-4 border-b border-white/[0.02]">
                                <h2 className="text-[11px] font-bold uppercase tracking-[0.3em] text-white/40 leading-none border-l border-white/10 pl-4">
                                    {section.label}
                                </h2>
                            </div>

                            <div className="flex flex-col w-full divide-y divide-white/[0.015]">
                                {section.data.map(item => (
                                    <ResourceCard 
                                        key={item.model.order_id} 
                                        id={item.model.order_id} 
                                        isCloud={session.is_cloud} 
                                        sessionId={session.id} 
                                    />
                                ))}
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}
