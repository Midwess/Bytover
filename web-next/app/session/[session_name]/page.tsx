'use client';

import * as React from "react";
import { useEffect, useState, useMemo } from "react";
import { useParams } from "next/navigation";
import {
    AppEventVariantTransfer,
    ReceiveResourceViewModel,
    ReceiveSessionViewModel,
    ResourceTypeVariantFolder,
    ResourceTypeVariantImage,
    ResourceTypeVariantVideo,
    TransferEventVariantFindSession,
    TransferEventVariantViewSession,
    TransferTypeVariantReceive
} from 'shared_types/types/shared_types';
import {
    LoaderCircle,
    Play,
    ShieldAlert,
    Download,
    FileText,
    Folder,
    Check
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from "@/components/ui/input";
import core from "@/wasm/wasm_core";
import { useDownloadResource } from "@/app/transfer/hooks/use-download-resource";
import { formatFileSize } from "@/utils/format-file-size";
import Header from "@/components/web/header";
import Footer from "@/components/web/footer";
import { SignallingAnimation, DEFAULT_WORDS } from "@/components/ui/signalling-animation";
import { useIsMobile } from "@/hooks/use-mobile";
import { Avatar, AvatarImage } from "@/components/ui/avatar";
import { cn } from "@/lib/utils";
import { ResourceCard } from "./components/resource-card";

export default function SessionPage() {
    const params = useParams();
    const sessionName = params.session_name as string;
    const coreReady = core.useCoreReady();
    const coreCompatible = core.useIsCoreCompatible();
    
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

    const [enteredPassword, setEnteredPassword] = useState<string>('');

    const handlePasswordSubmit = () => {
        if (session) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession(
                enteredPassword || null,
                BigInt(session.id),
                new TransferTypeVariantReceive()
            )));
        }
    };

    const images = useMemo(() => session?.resources.filter(r => r.model.type instanceof ResourceTypeVariantImage) || [], [session?.resources]);
    const videos = useMemo(() => session?.resources.filter(r => r.model.type instanceof ResourceTypeVariantVideo) || [], [session?.resources]);
    const files = useMemo(() => session?.resources.filter(r => !(r.model.type instanceof ResourceTypeVariantImage) && !(r.model.type instanceof ResourceTypeVariantVideo)) || [], [session?.resources]);

    return (
        <div className="min-h-screen bg-[#0F0F0F] text-[#E0E0E0] flex flex-col font-sans selection:bg-bluePrimary/30">
            <Header isFullWidth={true} theme="dark" />
            
            <main className="flex-1 flex flex-col pt-40 pb-32 max-w-[1360px] mx-auto px-6 w-full min-h-screen">
                {!coreCompatible ? (
                    <div className="flex-1 flex flex-col items-center justify-center gap-6 text-center max-w-sm mx-auto">
                        <div className="w-16 h-16 rounded-3xl bg-[#1A1A1A] flex items-center justify-center border border-white/5 shadow-2xl">
                            <ShieldAlert className="w-8 h-8 text-zinc-600" />
                        </div>
                        <div className="space-y-2">
                            <h2 className="text-xl font-bold text-white">Access Denied</h2>
                            <p className="text-sm text-zinc-500 leading-relaxed">Secure P2P requires a modern browser context. Please use Chrome or Safari.</p>
                        </div>
                    </div>
                ) : !session ? (
                    <div className="flex-1 flex flex-col items-center justify-center gap-4">
                        <LoaderCircle className="animate-spin w-6 h-6 text-zinc-800" />
                        <p className="text-sm text-zinc-700 font-medium tracking-tight">Locating drop instance...</p>
                    </div>
                ) : (
                    <div className="space-y-6 animate-in fade-in duration-700">
                        {/* Elegant Header */}
                        <div className="flex flex-col md:flex-row md:items-center justify-between gap-6">
                            <div className="flex items-center gap-4">
                                <div className="shrink-0">
                                    <Avatar className="w-12 h-12 rounded-2xl border border-white/5 shadow-xl bg-[#1A1A1A]">
                                        <AvatarImage src={session.sender_avatar} />
                                    </Avatar>
                                </div>
                                <div className="space-y-0.5">
                                    <h1 className="text-2xl font-bold tracking-tight text-white">
                                        {session.sender_name || 'Anonymous'}
                                    </h1>
                                    <div className="flex items-center gap-2.5 text-[13px] text-zinc-500 font-medium">
                                        <span>{session.resources.length} {session.resources.length === 1 ? 'item' : 'items'}</span>
                                        <span className="w-1 h-1 rounded-full bg-zinc-800" />
                                        <span className="text-bluePrimary font-bold uppercase tracking-[0.1em] text-[9px]">{sessionName}</span>
                                    </div>
                                </div>
                            </div>
                            
                            {session.resources?.length > 0 && (
                                <div className="flex items-center gap-5">
                                    {(session as ReceiveSessionViewModel).display_download_speed && (
                                        <span className="text-[9px] font-bold text-zinc-600 uppercase tracking-[0.2em]">
                                            {(session as ReceiveSessionViewModel).display_download_speed}
                                        </span>
                                    )}
                                    <CustomDownloadAllButton session={session as ReceiveSessionViewModel} />
                                </div>
                            )}
                        </div>

                        {/* Files Content */}
                        {isLoading && (!session.resources || session.resources.length === 0) ? (
                            <div className="py-32 flex flex-col items-center justify-center">
                                {session.password_required && !session.password ? (
                                    <div className="w-full max-w-sm p-10 rounded-[40px] bg-[#1A1A1A] border border-white/5 shadow-2xl space-y-8">
                                        <div className="text-center space-y-2">
                                            <h2 className="text-xl font-bold text-white">Locked Drop</h2>
                                            <p className="text-sm text-zinc-500">Provide the encryption key to decrypt metadata.</p>
                                        </div>
                                        <div className="space-y-4">
                                            <Input
                                                type="password"
                                                placeholder="Enter password"
                                                value={enteredPassword}
                                                onChange={(e) => setEnteredPassword(e.target.value)}
                                                onKeyDown={(e) => e.key === 'Enter' && handlePasswordSubmit()}
                                                className="bg-black/40 border-white/5 h-14 rounded-2xl text-center text-lg focus:border-bluePrimary/50 transition-all text-white"
                                            />
                                            <Button onClick={handlePasswordSubmit} className="w-full bg-bluePrimary text-white hover:bg-bluePrimary/90 h-14 rounded-2xl font-bold text-base transition-transform active:scale-95 shadow-lg shadow-bluePrimary/20">
                                                Decrypt & Open
                                            </Button>
                                            {session.error_message && <p className="text-red-500 text-xs text-center font-medium">{session.error_message}</p>}
                                        </div>
                                    </div>
                                ) : (
                                    <div className="flex flex-col items-center gap-6">
                                        <LoaderCircle className="animate-spin w-8 h-8 text-zinc-800" />
                                        <p className="text-zinc-600 font-bold uppercase tracking-[0.2em] text-[10px]">
                                            <SignallingAnimation words={[session.loading_status || 'Synchronizing', ...DEFAULT_WORDS]} />
                                        </p>
                                    </div>
                                )}
                            </div>
                        ) : (
                            <div className="flex flex-col gap-8">
                                {images.length > 0 && (
                                    <div className="flex flex-col md:grid md:grid-cols-3 lg:grid-cols-4 gap-8">
                                        {images.map(image => (
                                            <div key={image.model.order_id} className="h-[300px]">
                                                <ResourceCard id={image.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                                            </div>
                                        ))}
                                    </div>
                                )}
                                {videos.length > 0 && (
                                    <div className="flex flex-col md:grid md:grid-cols-3 lg:grid-cols-4 gap-8">
                                        {videos.map(video => (
                                            <div key={video.model.order_id} className="h-[300px]">
                                                <ResourceCard id={video.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                                            </div>
                                        ))}
                                    </div>
                                )}
                                {files.length > 0 && (
                                    <div className="flex flex-col md:grid md:grid-cols-3 lg:grid-cols-4 gap-8">
                                        {files.map(file => (
                                            <div key={file.model.order_id} className="h-[300px]">
                                                <ResourceCard id={file.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                                            </div>
                                        ))}
                                    </div>
                                )}
                            </div>
                        )}
                        
                        {!isLoading && session.resources?.length === 0 && (
                            <div className="py-40 text-center">
                                <p className="text-zinc-700 font-medium uppercase tracking-[0.1em] text-xs">No assets found in this instance.</p>
                            </div>
                        )}
                    </div>
                )}
            </main>

            <Footer isFullWidth={true} theme="dark" />
        </div>
    );
}

function CustomDownloadAllButton({ session }: { session: ReceiveSessionViewModel }) {
    const resource = session.download_all_resource;
    const { handleDownload, handleCancel } = useDownloadResource({
        resource: resource ?? null,
        session,
        isDownloadAll: true
    });

    if (!resource) return null;

    const isInProgress = resource.completion > 0 && !resource.is_completed;

    return (
        <Button 
            onClick={isInProgress ? handleCancel : handleDownload}
            className={cn(
                "h-12 px-8 rounded-2xl font-bold transition-all active:scale-95 gap-3",
                isInProgress 
                    ? "bg-[#1A1A1A] text-white hover:bg-[#252525] border border-white/5" 
                    : "bg-bluePrimary text-white hover:bg-bluePrimary/90 shadow-[0_8px_20px_-4px_rgba(59,130,246,0.4)]"
            )}
        >
            {isInProgress ? (
                <>
                    <LoaderCircle className="w-4 h-4 animate-spin text-bluePrimary" />
                    <span className="font-mono text-sm">{Math.round(resource.completion * 100)}%</span>
                </>
            ) : resource.is_completed ? (
                <>
                    <Check className="w-4 h-4" />
                    <span>Completed</span>
                </>
            ) : (
                <>
                    <Download className="w-4 h-4" />
                    <span>Download All</span>
                </>
            )}
        </Button>
    )
}

