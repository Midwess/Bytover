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
    const scrollContainerRef = useRef<HTMLDivElement>(null);
    const [activeSection, setActiveSection] = useState<string | null>(null);
    const [trackBounds, setTrackBounds] = useState({ top: 0, height: 0 });

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

    const FOCUS_POINT_Y = 104; // py-12 (48px) + top-14 (56px)
    const SECTION_DOT_OFFSET = 56; // top-14

    useEffect(() => {
        const updateTrack = () => {
            const firstEl = document.getElementById(sections[0]?.id);
            const lastEl = document.getElementById(sections[sections.length - 1]?.id);
            
            if (firstEl && lastEl) {
                const start = firstEl.offsetTop + SECTION_DOT_OFFSET;
                const end = lastEl.offsetTop + SECTION_DOT_OFFSET;
                setTrackBounds({ top: start, height: Math.max(0, end - start) });
            }
        };

        updateTrack();
        const timer = setTimeout(updateTrack, 100);
        window.addEventListener('resize', updateTrack);
        return () => {
            window.removeEventListener('resize', updateTrack);
            clearTimeout(timer);
        };
    }, [sections]);

    const scrollToId = (id: string) => {
        const el = document.getElementById(id);
        const container = scrollContainerRef.current;
        if (el && container) {
            // Scroll so the section dot aligns with the focus point
            // dotPos = el.offsetTop + SECTION_DOT_OFFSET
            // desiredScroll = dotPos - FOCUS_POINT_Y + containerPadding? 
            // Wait, el.offsetTop is relative to the scroll content (which has py-12)
            // So dot is at el.offsetTop + 56.
            // We want this to be at FOCUS_POINT_Y from the top of the container.
            container.scrollTo({
                top: el.offsetTop + SECTION_DOT_OFFSET - (FOCUS_POINT_Y - 48), 
                behavior: 'smooth'
            });
        }
    };

    useEffect(() => {
        const container = scrollContainerRef.current;
        if (!container) return;

        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    setActiveSection(entry.target.id);
                }
            });
        }, { 
            root: container,
            rootMargin: '-20% 0% -60% 0%',
            threshold: 0
        });

        sections.forEach(s => {
            const el = document.getElementById(s.id);
            if (el) observer.observe(el);
        });

        return () => observer.disconnect();
    }, [sections]);

    const progressVal = useMotionValue(0);
    const smoothProgress = useSpring(progressVal, { stiffness: 100, damping: 30, restDelta: 0.001 });

    useEffect(() => {
        const scrollContainer = scrollContainerRef.current;
        if (!scrollContainer || sections.length <= 1) return;

        let active = true;
        const updateProgress = () => {
            if (!active || !scrollContainer) return;

            const firstSectionEl = document.getElementById(sections[0].id);
            const lastSectionEl = document.getElementById(sections[sections.length - 1].id);
            if (!firstSectionEl || !lastSectionEl) return;

            const checkpointTrackLength = Math.max(1, lastSectionEl.offsetTop - firstSectionEl.offsetTop);
            const p = scrollContainer.scrollTop / checkpointTrackLength;
            
            progressVal.set(Math.max(0, Math.min(1, p)));
            requestAnimationFrame(updateProgress);
        };

        const rafId = requestAnimationFrame(updateProgress);
        return () => { active = false; cancelAnimationFrame(rafId); };
    }, [sections]);

    const hasMultipleSections = sections.length > 1;

    // The track is 2.5rem (40px) to the left of the max-w-xl (576px) list.
    // List left = 50% - 288px.
    // Track left = (50% - 288px) - 40px = 50% - 328px.
    const TRACK_LEFT_OFFSET = "calc(50% - 328px)";

    return (
        <div ref={containerRef} className="relative w-full md:h-full flex flex-col items-center">
            {/* Moving Dot (Sticky focus point indicator) */}
            {hasMultipleSections && (
                <div 
                    style={{ left: TRACK_LEFT_OFFSET }}
                    className="hidden md:block absolute top-[104px] z-30 pointer-events-none"
                >
                    <div className="w-1.5 h-1.5 bg-white rounded-full shadow-[0_0_8px_rgba(255,255,255,0.6)] -ml-[0.5px] -mt-[3px]" />
                </div>
            )}

            <div 
                ref={scrollContainerRef} 
                className="grid-scroll-container w-full md:h-full md:overflow-y-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
            >
                <div className="relative w-full max-w-xl mx-auto py-12 px-6 md:px-0">
                    {/* Track (Inside, scrolls with content) */}
                    {hasMultipleSections && (
                        <div 
                            style={{ 
                                height: `${trackBounds.height}px`, 
                                top: `${trackBounds.top}px`,
                                left: '-2.5rem'
                            }}
                            className="absolute w-[1px] bg-white/[0.05] hidden md:block"
                        />
                    )}

                    <div className="space-y-20 w-full">
                        {sections.map((section) => {
                            const isActive = activeSection === section.id;
                            return (
                                <div key={section.id} id={section.id} className="scroll-mt-10 relative">
                                    {/* Navigation Header */}
                                    {hasMultipleSections && (
                                        <div className="absolute hidden md:flex left-[-2.5rem] top-14 h-6 items-center justify-end overflow-visible">
                                            <button 
                                                onClick={() => scrollToId(section.id)}
                                                className="absolute right-0 flex items-center justify-end outline-none group"
                                                style={{ width: '200px' }}
                                            >
                                                <div className="flex flex-col items-end mr-4 transition-all opacity-40 group-hover:opacity-100">
                                                    <span className={cn(
                                                        "text-[7px] font-bold uppercase tracking-[0.2em] leading-none mb-0.5 transition-colors",
                                                        isActive ? "text-white" : "text-zinc-600"
                                                    )}>
                                                        {section.count.toString().padStart(2, '0')}
                                                    </span>
                                                    <span className={cn(
                                                        "text-[10px] font-bold uppercase tracking-widest transition-all duration-300 whitespace-nowrap leading-none",
                                                        isActive ? "text-white" : "text-zinc-600"
                                                    )}>
                                                        {section.label}
                                                    </span>
                                                </div>
                                                <div className={cn(
                                                    "w-1 h-1 rounded-full transition-all duration-300 z-20 shrink-0",
                                                    isActive ? "bg-white scale-125 shadow-[0_0_8px_rgba(255,255,255,0.4)]" : "bg-zinc-800 group-hover:bg-zinc-600",
                                                    "relative right-[-0.5px]" 
                                                )} />
                                            </button>
                                        </div>
                                    )}
                                    
                                    {/* Mobile Header */}
                                    <div className="md:hidden mb-6 flex flex-col gap-1.5 px-2">
                                         <span className="text-[9px] font-bold text-zinc-600 uppercase tracking-widest leading-none">{section.count} Items</span>
                                         <h2 className="text-sm font-bold uppercase tracking-widest text-white leading-none">{section.label}</h2>
                                    </div>

                                    <div className="flex flex-col w-full divide-y divide-white/[0.015]">
                                        {section.data.map(item => (
                                            <div key={item.model.order_id} className="w-full">
                                                <ResourceCard id={item.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                </div>
            </div>
        </div>
    );
}
