'use client';

import * as React from "react";
import { useEffect, useMemo, useState } from "react";
import { useParams } from "next/navigation";
import {
    AppEventVariantTransfer,
    MessageReasonVariantFailedToFindPublicSession,
    ReceiveSessionViewModel,
    TransferEventVariantFindSession,
    TransferEventVariantViewSession,
    TransferTypeVariantReceive,
} from 'shared_types/types/shared_types';
import { LoaderCircle } from 'lucide-react';
import core from "@/wasm/wasm_core";
import Footer from "@/components/web/footer";
import StaticHeader from "@/components/web/static-header";
import { Avatar, AvatarImage, AvatarFallback } from "@/components/ui/avatar";
import { ResourceGrid } from "@/components/main/resource-grid";
import { DownloadAllButton } from "@/components/main/download-all-button";
import { motion } from "framer-motion";
import Aurora from "@/components/ui/aurora";
import {
    IncompatibleBrowser,
    EmptyState,
    LoadingState,
    PasswordPrompt,
} from "../../../components/main";

export default function SessionPage() {
    const params = useParams();
    const sessionName = params.session_name as string;
    const coreReady = core.useCoreReady();
    const coreCompatible = core.useIsCoreCompatible();
    const findSessionFailedMessage = core.useMessage(new MessageReasonVariantFailedToFindPublicSession())
    
    const [accentColor, setAccentColor] = useState<string>("59, 130, 246");

    const auroraColors = useMemo(() => {
        const [r, g, b] = accentColor.split(',').map(v => Number(v) / 255);

        // RGB to HSL conversion
        const max = Math.max(r, g, b), min = Math.min(r, g, b);
        const l = (max + min) / 2;
        let h = 0, s: number;

        if (max === min) {
            h = s = 0; // achromatic
        } else {
            const d = max - min;
            s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
            switch (max) {
                case r: h = (g - b) / d + (g < b ? 6 : 0); break;
                case g: h = (b - r) / d + 2; break;
                case b: h = (r - g) / d + 4; break;
            }
            h /= 6;
        }

        // HSL to Hex helper
        const hslToHex = (h: number, s: number, l: number) => {
            h = h % 1;
            const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
            const p = 2 * l - q;
            const f = (t: number) => {
                if (t < 0) t += 1;
                if (t > 1) t -= 1;
                if (t < 1/6) return p + (q - p) * 6 * t;
                if (t < 1/2) return q;
                if (t < 2/3) return p + (q - p) * (2/3 - t) * 6;
                return p;
            };
            const toHex = (x: number) => Math.round(x * 255).toString(16).padStart(2, '0');
            return `#${toHex(f(h + 1/3))}${toHex(f(h))}${toHex(f(h - 1/3))}`;
        };

        // Generate 3 colors: Tight monochromatic + Slightly Brighter
        const refinedL = l * 0.8; // Refine base lightness
        return [
            hslToHex(h, s, refinedL), // Base
            hslToHex(h + 0.03, s, refinedL * 0.8), // Very slight shift + darker
            hslToHex(h - 0.03, s, refinedL * 0.9)  // Very slight shift back
        ];
    }, [accentColor]);

    useEffect(() => {
        if (coreReady && coreCompatible) {
            core.launchNearby();
        }
    }, [coreReady, coreCompatible]);

    const searchSessions = core.useSearchSessionsList();
    const allSessions = core.useAllSessionsList();
    const selectedSession = core.useSelectedSession();

    useEffect(() => {
        if (coreReady && sessionName) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantFindSession(sessionName)));
        }
    }, [coreReady, sessionName]);

    const targetSessionFromList = useMemo(() => {
        return searchSessions.find(s => s.alias === sessionName) ||
            allSessions.find(s => s.alias === sessionName);
    }, [searchSessions, allSessions, sessionName]);

    const session = useMemo(() => {
        if (selectedSession?.alias === sessionName) return selectedSession;
        return targetSessionFromList;
    }, [selectedSession, targetSessionFromList, sessionName]);

    const isLoading = session?.is_loading;

    // Color extraction logic from URL params
    useEffect(() => {
        if (session?.sender_avatar) {
            try {
                const url = new URL(session.sender_avatar);
                const r = url.searchParams.get('r');
                const g = url.searchParams.get('g');
                const b = url.searchParams.get('b');

                if (r && g && b) {
                    requestAnimationFrame(() => setAccentColor(`${r}, ${g}, ${b}`));
                }
            } catch {
                // Keep default accent color
            }
        }
    }, [session?.sender_avatar]);

    useEffect(() => {
        if (session && !session.resources?.length && !session.error_message) {
            if (!session.password_required || session.password) {
                core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession(
                    session.password || null,
                    BigInt(session.id),
                    new TransferTypeVariantReceive(),
                )));
            }
        }
    }, [session?.id, session?.password_required, session?.password, session?.resources?.length, session?.error_message]);

    const handlePasswordSubmit = (password: string) => {
        if (session) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession(
                password || null,
                BigInt(session.id),
                new TransferTypeVariantReceive()
            )));
        }
    };

    return (
        <div className="min-h-screen bg-[#09090b] text-[#fafafa] flex flex-col font-sans selection:bg-white/10 relative">
            {/* Aurora Background */}
            <div className="absolute inset-0 z-0 pointer-events-none overflow-hidden h-[40vh]">
                <Aurora
                    colorStops={auroraColors}
                    blend={0.5}
                    amplitude={0.55}
                    speed={0.3}
                />
                
                {/* Dotted Overlay */}
                <div 
                    className="absolute inset-0 opacity-[0.04]" 
                    style={{ 
                        backgroundImage: 'radial-gradient(circle, #ffffff 1px, transparent 1px)', 
                        backgroundSize: '40px 40px' 
                    }} 
                />
                
                {/* Bottom Fade */}
                <div className="absolute inset-x-0 bottom-0 h-32 bg-gradient-to-t from-[#09090b] to-transparent" />
            </div>

            {!coreCompatible ? (
                <main className="flex items-center justify-center p-6 relative z-10 min-h-screen">
                    <IncompatibleBrowser />
                </main>
            ) : !session ? (
                <main className="flex flex-col items-center justify-center gap-8 py-20 relative z-10">
                    {findSessionFailedMessage.message ? (
                        <p className="text-zinc-100 font-medium text-xl">{findSessionFailedMessage.message}</p>
                    ) : (
                        <>
                            <div className="w-14 h-14 flex items-center justify-center rounded-2xl bg-zinc-900/50 border border-white/5 backdrop-blur-xl">
                                <LoaderCircle className="animate-spin w-5 h-5 text-zinc-500" />
                            </div>
                            <p className="text-[11px] text-zinc-500 font-bold tracking-[0.2em]">INITIALIZING SESSION</p>
                        </>
                    )}
                </main>
            ) : (
                <div className="flex flex-col flex-1 shrink-0 pb-10 relative z-10 min-h-screen">
                    <div className="h-fit gap-10 mb-10 flex flex-col shrink-0 relative z-20">
                        <StaticHeader theme="dark" className="pt-6 md:pt-10" />
                        
                        <div className="flex-1 flex flex-col items-center justify-center text-center space-y-6 md:space-y-8 px-6 mt-12 md:mt-0">
                            <motion.div
                                initial={{ scale: 0.9, opacity: 0 }}
                                animate={{ scale: 1, opacity: 1 }}
                                transition={{ duration: 0.8, ease: "easeOut" }}
                            >
                                <Avatar 
                                    className="w-16 h-16 md:w-20 md:h-20 rounded-[1.5rem] md:rounded-[2rem] border border-white/10 shadow-2xl p-1 md:p-1.5 ring-1 ring-white/5 transition-colors duration-1000"
                                    style={{ backgroundColor: `rgba(${accentColor}, 0.2)` }}
                                >
                                    <AvatarImage src={session.sender_avatar} className="rounded-[1.2rem] md:rounded-[1.6rem] object-cover" />
                                    <AvatarFallback className="bg-zinc-800 text-zinc-500">
                                        {session.sender_name?.charAt(0) || 'A'}
                                    </AvatarFallback>
                                </Avatar>
                            </motion.div>
                            
                            <div className="space-y-8 flex flex-col items-center">
                                <h1 className="text-2xl md:text-3xl font-medium tracking-tight text-white">
                                    {session.sender_name || 'Anonymous'} shared some files
                                </h1>
                                
                                <div className="flex flex-wrap items-center justify-center gap-6 md:gap-10">
                                    <div className="flex items-center gap-3.5">
                                        <div className="flex flex-col items-end">
                                            <span className="text-[10px] font-bold text-white uppercase tracking-[0.2em] leading-none mb-1">
                                                {session.resources.length.toString().padStart(2, '0')} Items
                                            </span>
                                            <span className="text-[8px] font-bold text-zinc-600 uppercase tracking-widest leading-none">
                                                Available
                                            </span>
                                        </div>

                                        {session.display_download_speed && (
                                            <>
                                                <div className="w-px h-6 bg-white/10" />
                                                <div className="flex flex-col items-start">
                                                    <span className="text-[10px] font-bold text-white uppercase tracking-[0.2em] leading-none mb-1">
                                                        {session.display_download_speed}
                                                    </span>
                                                    <span className="text-[8px] font-bold text-zinc-600 uppercase tracking-widest leading-none">
                                                        Network Speed
                                                    </span>
                                                </div>
                                            </>
                                        )}
                                    </div>

                                    {session.resources?.length > 0 && (
                                        <div className="flex items-center">
                                            <DownloadAllButton
                                                session={session as ReceiveSessionViewModel}
                                                containerClass="rounded-full bg-gradient-to-r from-bluePrimary to-bluePrimary/80 text-white hover:opacity-90 transition-all duration-500 px-6 py-2.5 h-auto text-[10px] font-bold tracking-[0.2em] uppercase border-0"
                                            />
                                        </div>
                                    )}
                                </div>
                            </div>
                        </div>
                    </div>

                    <main className="flex flex-col items-center px-6 relative z-10 pb-12 md:pb-0">
                        <div className="w-full max-w-6xl flex flex-col animate-in fade-in duration-1000 slide-in-from-bottom-8">
                            <div className="w-full">
                                {isLoading && (!session.resources || session.resources.length === 0) ? (
                                    <div className="py-24 flex flex-col items-center justify-center">
                                        {session.password_required && !session.password ? (
                                            <PasswordPrompt
                                                theme="dark"
                                                errorMessage={session.error_message ?? undefined}
                                                onSubmit={handlePasswordSubmit}
                                            />
                                        ) : (
                                            <LoadingState status={session.loading_status ?? undefined} />
                                        )}
                                    </div>
                                ) : session.resources?.length === 0 ? (
                                    <div className="py-24 flex items-center justify-center">
                                        <EmptyState />
                                    </div>
                                ) : (
                                    <ResourceGrid session={session as ReceiveSessionViewModel} />
                                )}
                            </div>
                        </div>
                    </main>
                </div>
            )}

            <Footer theme="dark" className="bg-transparent border-0 opacity-40 hover:opacity-100 transition-opacity pb-8 md:pb-12 shrink-0 relative z-20" />
        </div>
    );
}
