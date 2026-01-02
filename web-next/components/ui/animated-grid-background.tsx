'use client';

import React from 'react';

export function AnimatedGridBackground() {
    return (
        <div className="fixed inset-0 z-0 overflow-hidden pointer-events-none">
            {/* Main grid container */}
            <div className="absolute inset-0 bg-black">
                {/* Animated gradient overlay */}
                <div className="absolute inset-0 bg-gradient-to-br from-blue-950/20 via-black to-purple-950/20 animate-gradient" />

                {/* Grid intersection dots */}
                <div className="absolute inset-0">
                    {Array.from({ length: 20 }).map((_, i) =>
                        Array.from({ length: 20 }).map((_, j) => (
                            <div
                                key={`dot-${i}-${j}`}
                                className="absolute w-[2px] h-[2px] rounded-full bg-blue-500/20"
                                style={{
                                    left: `${(i + 1) * 5}%`,
                                    top: `${(j + 1) * 5}%`,
                                    transform: 'translate(-50%, -50%)',
                                }}
                            />
                        ))
                    )}
                </div>

                {/* Animated glow spots */}
                <div className="absolute inset-0">
                    <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-blue-500/10 rounded-full blur-[100px] animate-pulse-slow" />
                    <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-purple-500/10 rounded-full blur-[100px] animate-pulse-slow-delayed" />
                    <div className="absolute top-1/2 right-1/3 w-64 h-64 bg-cyan-500/10 rounded-full blur-[80px] animate-float" />
                </div>
            </div>

            <style jsx>{`
                @keyframes gradient {
                    0%, 100% {
                        opacity: 1;
                    }
                    50% {
                        opacity: 0.8;
                    }
                }

                @keyframes pulse-slow {
                    0%, 100% {
                        opacity: 0.3;
                        transform: scale(1);
                    }
                    50% {
                        opacity: 0.5;
                        transform: scale(1.1);
                    }
                }

                @keyframes pulse-slow-delayed {
                    0%, 100% {
                        opacity: 0.2;
                        transform: scale(1);
                    }
                    50% {
                        opacity: 0.4;
                        transform: scale(1.15);
                    }
                }

                @keyframes float {
                    0%, 100% {
                        transform: translate(0, 0);
                    }
                    25% {
                        transform: translate(20px, -20px);
                    }
                    50% {
                        transform: translate(-10px, 30px);
                    }
                    75% {
                        transform: translate(30px, 10px);
                    }
                }

                .animate-gradient {
                    animation: gradient 8s ease-in-out infinite;
                }

                .animate-pulse-slow {
                    animation: pulse-slow 6s ease-in-out infinite;
                }

                .animate-pulse-slow-delayed {
                    animation: pulse-slow-delayed 7s ease-in-out infinite;
                    animation-delay: 1s;
                }

                .animate-float {
                    animation: float 12s ease-in-out infinite;
                }
            `}</style>
        </div>
    );
}
