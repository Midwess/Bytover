'use client';

import React from 'react';
import {Avatar, AvatarImage} from "@/components/ui/avatar.tsx";
import {ReceiveSessionViewModel} from '../../../shared_types/generated/typescript/types/shared_types.ts';
import {DownloadAllButton} from "@/components/main/download-all-button.tsx";
import StaticHeader from "@/components/web/static-header.tsx";

function getAvatarBackground(avatarUrl?: string): string {
    if (!avatarUrl) return '#1A1A1A';

    try {
        const url = new URL(avatarUrl);
        const r = url.searchParams.get('r');
        const g = url.searchParams.get('g');
        const b = url.searchParams.get('b');

        if (r && g && b) {
            return `rgb(${r}, ${g}, ${b})`;
        }
    } catch {
        return '#1A1A1A';
    }

    return '#1A1A1A';
}

interface SessionHeaderProps {
    session: ReceiveSessionViewModel;
    sessionName: string;
}

export function SessionHeader({session, sessionName}: SessionHeaderProps) {
    const avatarBg = getAvatarBackground(session.sender_avatar);
    const isColoredBg = avatarBg !== '#1A1A1A';

    return (
        <div className="w-full relative overflow-hidden">
            {session.sender_avatar && (
                <img
                    src={session.sender_avatar}
                    alt=""
                    className="absolute inset-0 w-[100vw] h-[40vh] object-cover object-center mx-auto my-auto opacity-80 bg-foreground"
                    style={{left: '50%', top: '50%', transform: 'translate(-50%, -50%)'}}
                />
            )}
            <div className="absolute inset-0" style={{
                backdropFilter: 'blur(100px) saturate(150%)',
                WebkitBackdropFilter: 'blur(100px) saturate(180%)'
            }}/>
            <div className="absolute inset-0 opacity-[0.03] pointer-events-none"
                 style={{backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)'/%3E%3C/svg%3E")`}}/>
            <div className="relative">
                <StaticHeader className={"pt-4 z-10 bg-none shadow-none"}/>
                <div className="flex flex-col gap-6 py-12 container mx-auto">
                    <div className="flex items-center justify-between gap-4">
                        <div className="flex items-center gap-4">
                            <div className="shrink-0">
                                <Avatar className="w-24 h-24 rounded-2xl border border-white/5 shadow-xl"
                                        style={{backgroundColor: isColoredBg ? 'rgba(255,255,255,0.2)' : avatarBg}}>
                                    <AvatarImage src={session.sender_avatar}/>
                                </Avatar>
                            </div>
                            <div className="space-1 flex flex-col gap-1.5">
                                <h1 className="text-3xl font-bold tracking-tight text-white">
                                    {session.sender_name || 'Anonymous'}
                                </h1>
                                <div className="flex items-center gap-3.5 text-[13px] font-medium"
                                     style={{color: isColoredBg ? 'rgba(255,255,255,0.7)' : undefined}}>
                                    <span>{session.resources.length} {session.resources.length === 1 ? 'item' : 'items'}</span>
                                    <span className="w-1 h-1 rounded-full bg-white/30"/>
                                    <span
                                        className="text-white font-bold uppercase tracking-[0.1em] text-[9px]">{sessionName}</span>
                                    <span className="w-1 h-1 rounded-full bg-white/30"/>
                                {session.display_download_speed && (
                                    <span className="text-[9px] font-bold uppercase tracking-[0.2em]"
                                          style={{color: isColoredBg ? 'rgba(255,255,255,0.7)' : undefined}}>
                                                {session.display_download_speed}
                                    </span>
                                )}
                                </div>
                                {session.resources?.length > 0 && (
                                    <DownloadAllButton session={session}/>
                                )}
                            </div>
                        </div>

                    </div>
                </div>
            </div>
        </div>
    );
}
