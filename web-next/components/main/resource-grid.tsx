'use client';

import { useMemo, useState, useEffect, useRef } from 'react';
import { ReceiveSessionViewModel, ResourceTypeVariantImage, ResourceTypeVariantVideo } from 'shared_types/types/shared_types';
import { ResourceCard } from "./resource-card.tsx";
import { cn } from "@/lib/utils";
import { useTransform, motion, useSpring, useMotionValue } from 'framer-motion';

interface ResourceGridProps {
    session: ReceiveSessionViewModel;
}

export function ResourceGrid({ session }: ResourceGridProps) {
    const containerRef = useRef<HTMLDivElement>(null);
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

    const scrollToId = (id: string) => {
        const el = document.getElementById(id);
        if (el) {
            const offset = 40;
            const elementPosition = el.getBoundingClientRect().top;
            const offsetPosition = elementPosition + window.pageYOffset - offset;

            window.scrollTo({
                top: offsetPosition,
                behavior: 'smooth'
            });
        }
    };

    const sections = useMemo(() => [
        { id: 'section-images', label: 'Images', count: images.length, data: images },
        { id: 'section-videos', label: 'Videos', count: videos.length, data: videos },
        { id: 'section-files', label: 'Files', count: files.length, data: files },
    ].filter(s => s.count > 0), [images, videos, files]);

    useEffect(() => {
        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    setActiveSection(entry.target.id);
                }
            });
        }, { 
            rootMargin: '-20% 0% -60% 0%',
            threshold: 0
        });

        sections.forEach(s => {
            const el = document.getElementById(s.id);
            if (el) observer.observe(el);
        });

        return () => observer.disconnect();
    }, [sections]);

    // Track scroll progress manually using requestAnimationFrame for maximum reliability
    const progressVal = useMotionValue(0);

    const smoothProgress = useSpring(progressVal, {
        stiffness: 100,
        damping: 30,
        restDelta: 0.001
    });

    useEffect(() => {
        if (!containerRef.current || sections.length <= 1) return;

        let active = true;
        const updateProgress = () => {
            if (!active || !containerRef.current) return;

            const rect = containerRef.current.getBoundingClientRect();
            const viewportHeight = window.innerHeight;
            
            // 0 when top is at bottom of viewport
            // 1 when bottom is at top of viewport
            const start = viewportHeight;
            const end = -rect.height;
            const current = rect.top;
            
            const p = (start - current) / (start - end);
            progressVal.set(Math.max(0, Math.min(1, p)));
            
            requestAnimationFrame(updateProgress);
        };

        const rafId = requestAnimationFrame(updateProgress);
        
        return () => {
            active = false;
            cancelAnimationFrame(rafId);
        };
    }, [sections.length]);

    // Transform scroll progress to dot position on the track (0% to 100%)
    const dotTop = useTransform(smoothProgress, [0, 1], ["0%", "100%"]);

    const hasMultipleSections = sections.length > 1;

    return (
        <div ref={containerRef} className="relative w-full">
            {/* 
                Vertical Track - Positioned OUTSIDE the container 
                xl: -24px, 2xl: -48px
            */}
            {hasMultipleSections && (
                <div className="absolute xl:-left-6 2xl:-left-12 top-6 bottom-8 w-px bg-white/10 hidden xl:block">
                    {/* The Moving Point - Progress indicator */}
                    <motion.div
                        style={{
                            top: dotTop,
                            marginLeft: '-4px',
                        }}
                        className="absolute w-2 h-2 bg-blue-600 rounded-full shadow-[0_0_12px_rgba(37,99,235,0.6)] z-30 -translate-y-1/2"
                    />
                </div>
            )}

            <div className="flex-1 space-y-32 w-full">
                {sections.map((section) => {
                    const isActive = activeSection === section.id;
                    return (
                        <div key={section.id} id={section.id} className="scroll-mt-10 relative">
                            {/* Navigation Header on the left of the line */}
                            {hasMultipleSections && (
                                <div className="absolute hidden xl:block xl:-left-6 2xl:-left-12 top-0 h-12 flex items-center overflow-visible">
                                    <button 
                                        onClick={() => scrollToId(section.id)}
                                        className="absolute right-0 flex items-center justify-end group outline-none"
                                        style={{ width: '400px' }}
                                    >
                                        <div className="flex flex-col items-end mr-8 transition-all">
                                            <span className={cn(
                                                "text-[10px] font-bold uppercase tracking-[0.2em] leading-none transition-opacity mb-1",
                                                isActive ? "text-blue-600 opacity-100" : "text-blue-600 opacity-40 group-hover:opacity-100"
                                            )}>
                                                {section.count.toString().padStart(2, '0')} Items
                                            </span>
                                            <span className={cn(
                                                "text-xl font-bold uppercase tracking-widest transition-all duration-300 whitespace-nowrap leading-none",
                                                isActive ? "text-white scale-105 origin-right" : "text-foreground/80 group-hover:text-white"
                                            )}>
                                                {section.label}
                                            </span>
                                        </div>
                                        {/* stationary checkpoint circle */}
                                        <div className={cn(
                                            "w-3.5 h-3.5 rounded-full bg-zinc-900 border-2 transition-all duration-300 z-20 shrink-0 shadow-sm",
                                            isActive 
                                                ? "border-blue-600/50 scale-110" 
                                                : "border-white/10 group-hover:border-blue-600/30",
                                            "relative right-[-7.5px]"
                                        )} />
                                    </button>
                                </div>
                            )}
                            
                            {/* Mobile Header */}
                            <div className="xl:hidden mb-10 flex flex-col gap-1.5">
                                 <span className="text-[10px] font-bold text-blue-600 uppercase tracking-widest leading-none">{section.count} Items</span>
                                 <h2 className="text-xl font-bold uppercase tracking-widest text-foreground/80 leading-none">{section.label}</h2>
                            </div>

                            <div className="flex-col md:grid md:grid-cols-3 lg:grid-cols-4 gap-8 flex">
                                {section.data.map(item => (
                                    <div key={item.model.order_id} className="h-[300px]">
                                        <ResourceCard id={item.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                                    </div>
                                ))}
                            </div>
                        </div>
                    );
                })}
            </div>
        </div>
    );
}
