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

            const container = containerRef.current;
            const rect = container.getBoundingClientRect();
            const viewportHeight = window.innerHeight;

            // Focus point: 20% from top of screen
            const focusPoint = viewportHeight * 0.2; 

            // Boundary calculations for the track
            const firstSectionEl = document.getElementById(sections[0].id);
            const lastSectionEl = document.getElementById(sections[sections.length - 1].id);
            
            if (!firstSectionEl || !lastSectionEl) return;

            const firstOffset = firstSectionEl.offsetTop;
            const lastOffset = lastSectionEl.offsetTop;
            
            // The distance between the first and last checkpoint centers
            const checkpointTrackLength = Math.max(1, lastOffset - firstOffset);

            // Calculate progress: 0 when center of first checkpoint is at focusPoint
            // Using 80px midline (top-14 header center)
            let p = (focusPoint - (rect.top + firstOffset + 80)) / checkpointTrackLength;
            p = Math.max(0, Math.min(1, p));


            // Magnetic snap to sections (rounding) with "Sticky" behavior
            let snappedP = p;

            for (const section of sections) {
                const el = document.getElementById(section.id);
                if (el) {
                    // snapP is relative to the checkpoint-to-checkpoint track
                    const snapP = (el.offsetTop - firstOffset) / checkpointTrackLength;
                    const distance = Math.abs(p - snapP);
                    
                    const snapWindow = 0.15; // Range where the dot starts pulling
                    const deadZone = 0.08;   // "Round" behavior: stay in center when close

                    if (distance < snapWindow) {
                        if (distance < deadZone) {
                            snappedP = snapP;
                        } else {
                            const t = (distance - deadZone) / (snapWindow - deadZone);
                            const ease = t * t * (3 - 2 * t);
                            snappedP = snapP + (p - snapP) * ease;
                        }
                        break;
                    }
                }
            }

            progressVal.set(snappedP);
            requestAnimationFrame(updateProgress);
        };

        const rafId = requestAnimationFrame(updateProgress);

        return () => {
            active = false;
            cancelAnimationFrame(rafId);
        };
    }, [sections]);

    // Transform scroll progress to dot position on the track 
    // Maps 0-1 progress to the actual pixel positions of the checkpoints (at 80px midline)
    const firstPointPos = (sections.length > 0 ? (document.getElementById(sections[0].id)?.offsetTop || 0) : 0) + 80;
    const lastPointPos = (sections.length > 0 ? (document.getElementById(sections[sections.length - 1].id)?.offsetTop || 0) : 0) + 80;
    
    // We use a MotionValue for the numeric top position to avoid CSS centering issues
    const dotTop = useTransform(smoothProgress, [0, 1], [firstPointPos, lastPointPos]);

    const hasMultipleSections = sections.length > 1;

    return (
        <div ref={containerRef} className="relative w-full max-w-4xl mx-auto px-4 md:px-0">
            {/* 
                Vertical Track - Positioned right next to the list 
            */}
            {hasMultipleSections && (
                <div 
                    style={{ 
                        top: '0px', 
                        bottom: 'auto',
                        height: `${lastPointPos}px`
                    }}
                    className="absolute left-4 md:left-[calc(50%-18.5rem)] w-[1px] bg-white/[0.05] hidden md:block"
                >
                    {/* The Moving Point - Progress indicator */}
                    <motion.div
                        style={{
                            top: dotTop,
                            y: -2,
                            marginLeft: '-1px',
                        }}
                        className="absolute w-1 h-1 bg-white rounded-full shadow-[0_0_6px_rgba(255,255,255,0.4)] z-30"
                    />
                </div>
            )}

            <div className="flex-1 space-y-12 w-full">
                {sections.map((section) => {
                    const isActive = activeSection === section.id;
                    return (
                        <div key={section.id} id={section.id} className="scroll-mt-10 relative">
                            {/* Navigation Header right on the left edge */}
                            {hasMultipleSections && (
                                <div className="absolute hidden md:block left-4 md:left-[calc(50%-18.5rem)] top-14 h-6 flex items-center overflow-visible">
                                    <button 
                                        onClick={() => scrollToId(section.id)}
                                        className="absolute right-0 flex items-center justify-end group outline-none h-full"
                                        style={{ width: '200px' }}
                                    >
                                        <div className="flex flex-col items-end mr-4 transition-all opacity-40 group-hover:opacity-100">
                                            <span className={cn(
                                                "text-[8px] font-bold uppercase tracking-[0.2em] leading-none mb-0.5 transition-colors",
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
                                        {/* stationary checkpoint circle */}
                                        <div className={cn(
                                            "w-1.5 h-1.5 rounded-full transition-all duration-300 z-20 shrink-0",
                                            isActive 
                                                ? "bg-white scale-110" 
                                                : "bg-zinc-800 group-hover:bg-zinc-600",
                                            "relative right-[-3px]"
                                        )} />
                                    </button>
                                </div>
                            )}
                            
                            {/* Mobile Header */}
                            <div className="md:hidden mb-6 flex flex-col gap-1.5 px-2">
                                 <span className="text-[9px] font-bold text-zinc-600 uppercase tracking-widest leading-none">{section.count} Items</span>
                                 <h2 className="text-sm font-bold uppercase tracking-widest text-white leading-none">{section.label}</h2>
                            </div>

                            <div className="flex flex-col w-full max-w-xl mx-auto divide-y divide-white/[0.02]">
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
    );
}
