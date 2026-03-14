'use client';

import React from 'react';

const DOT_PATTERN = "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24'%3E%3Ccircle cx='12' cy='12' r='1' fill='rgba(255,255,255,0.08)'/%3E%3C/svg%3E\")";
const DOT_PATTERN_LIGHT = "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24'%3E%3Ccircle cx='12' cy='12' r='1' fill='rgba(255,255,255,0.04)'/%3E%3C/svg%3E\")";
const DASHED_BORDER_V = "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='1' height='18' viewBox='0 0 1 18'%3E%3Cline x1='0.5' y1='0' x2='0.5' y2='12' stroke='rgba(255,255,255,0.1)' stroke-width='1'/%3E%3C/svg%3E\")";
const DASHED_BORDER_H = "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='18' height='1' viewBox='0 0 18 1'%3E%3Cline x1='0' y1='0.5' x2='12' y2='0.5' stroke='rgba(255,255,255,0.1)' stroke-width='1'/%3E%3C/svg%3E\")";

interface GridSectionWrapperProps {
    children: React.ReactNode;
    className?: string;
}

export function GridSectionWrapper({ children, className = '' }: GridSectionWrapperProps) {
    return (
        <div className={`relative flex mt-8 ${className}`}>
            <div className="absolute left-0 right-0 top-0 h-px" style={{ backgroundImage: DASHED_BORDER_H, backgroundRepeat: 'repeat-x' }} />
            <div className="absolute left-0 right-0 bottom-0 h-px" style={{ backgroundImage: DASHED_BORDER_H, backgroundRepeat: 'repeat-x' }} />

            <div
                className="hidden md:block relative flex-shrink-0 md:w-24 lg:w-32 xl:w-[120px]"
                style={{ backgroundImage: DOT_PATTERN }}
            >
                <div className="absolute left-0 top-0 bottom-0 w-px" style={{ backgroundImage: DASHED_BORDER_V, backgroundRepeat: 'repeat-y' }} />
                <div className="absolute right-0 top-0 bottom-0 w-px" style={{ backgroundImage: DASHED_BORDER_V, backgroundRepeat: 'repeat-y' }} />
            </div>

            <div
                className="flex-1 bg-bluePrimary/2 relative min-w-0"
                style={{ backgroundImage: DOT_PATTERN_LIGHT }}
            >
                <div className="absolute -top-1 -left-1 w-3 h-3 border-l border-t border-primaryText/30" />
                <div className="absolute -top-1 -right-1 w-3 h-3 border-r border-t border-primaryText/30" />
                <div className="absolute -bottom-1 -left-1 w-3 h-3 border-l border-b border-primaryText/30" />
                <div className="absolute -bottom-1 -right-1 w-3 h-3 border-r border-b border-primaryText/30" />
                {children}
            </div>

            <div
                className="hidden md:block relative flex-shrink-0 md:w-24 lg:w-32 xl:w-[120px]"
                style={{ backgroundImage: DOT_PATTERN }}
            >
                <div className="absolute left-0 top-0 bottom-0 w-px" style={{ backgroundImage: DASHED_BORDER_V, backgroundRepeat: 'repeat-y' }} />
                <div className="absolute right-0 top-0 bottom-0 w-px" style={{ backgroundImage: DASHED_BORDER_V, backgroundRepeat: 'repeat-y' }} />
            </div>
        </div>
    );
}
