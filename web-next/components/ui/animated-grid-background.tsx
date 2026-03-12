'use client';

import React from 'react';

export function AnimatedGridBackground() {
    return (
        <div className="fixed inset-0 z-0 overflow-hidden pointer-events-none">
            {/* Base Background */}
            <div className="absolute inset-0 bg-black" />
            
            {/* Noise Overlay */}
            <div className="absolute inset-0 opacity-[0.15] mix-blend-overlay" style={{ backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/svg%3E#noiseFilter")` }}>
                <svg className="hidden">
                    <filter id="noise">
                        <feTurbulence type="fractalNoise" baseFrequency="0.8" numOctaves="4" stitchTiles="stitch" />
                        <feColorMatrix type="saturate" values="0" />
                    </filter>
                </svg>
                <div className="absolute inset-0" style={{ filter: 'url(#noise)' }} />
            </div>

            {/* Main grid container */}
            <div className="absolute inset-0">
                {/* Animated gradient overlay - Railway Style */}
                <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_50%,rgba(56,189,248,0.08),transparent_50%)]" />
                <div className="absolute inset-0 bg-[radial-gradient(circle_at_20%_30%,rgba(59,130,246,0.05),transparent_40%)]" />
                <div className="absolute inset-0 bg-[radial-gradient(circle_at_80%_70%,rgba(139,92,246,0.05),transparent_40%)]" />

                {/* Grid intersection dots - More subtle */}
                <div className="absolute inset-0" style={{ backgroundImage: 'radial-gradient(circle, rgba(255,255,255,0.05) 1px, transparent 1px)', backgroundSize: '40px 40px' }} />

                {/* Animated glow spots */}
                <div className="absolute inset-0 overflow-hidden">
                    <div className="absolute top-[-10%] left-[-10%] w-[40%] h-[40%] bg-blue-500/10 rounded-full blur-[120px] animate-pulse-slow" />
                    <div className="absolute bottom-[-10%] right-[-10%] w-[40%] h-[40%] bg-purple-500/10 rounded-full blur-[120px] animate-pulse-slow-delayed" />
                </div>
            </div>

            <style jsx>{`
                @keyframes pulse-slow {
                    0%, 100% {
                        opacity: 0.4;
                        transform: scale(1) translate(0, 0);
                    }
                    50% {
                        opacity: 0.6;
                        transform: scale(1.1) translate(20px, 20px);
                    }
                }

                @keyframes pulse-slow-delayed {
                    0%, 100% {
                        opacity: 0.3;
                        transform: scale(1) translate(0, 0);
                    }
                    50% {
                        opacity: 0.5;
                        transform: scale(1.15) translate(-20px, -20px);
                    }
                }

                .animate-pulse-slow {
                    animation: pulse-slow 10s ease-in-out infinite;
                }

                .animate-pulse-slow-delayed {
                    animation: pulse-slow-delayed 12s ease-in-out infinite;
                    animation-delay: 2s;
                }
            `}</style>
        </div>
    );
}
