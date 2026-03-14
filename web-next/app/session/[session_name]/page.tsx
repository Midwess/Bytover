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
                            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-12">
                                {images.map(image => (
                                    <ResourceCard key={image.model.order_id} id={image.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                                ))}
                                {videos.map(video => (
                                    <ResourceCard key={video.model.order_id} id={video.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                                ))}
                                {files.map(file => (
                                    <ResourceCard key={file.model.order_id} id={file.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                                ))}
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

function ResourceCard({ id, isCloud, sessionId }: { id: string, isCloud: boolean, sessionId: string }) {
    const resource = core.useReceiveResource(id, isCloud);
    const session = core.useSession(sessionId);
    const model = resource?.model;
    const [thumbnailSource, setThumbnailSource] = useState<string | undefined>();

    useEffect(() => {
        if (model?.thumbnail_path) {
            core.getDownloadUrl(model.thumbnail_path).then(setThumbnailSource);
        }
    }, [model?.thumbnail_path]);

    const { handleDownload, handleCancel } = useDownloadResource({
        resource: resource ?? null,
        session: session as ReceiveSessionViewModel ?? null
    });

    if (!resource || !model || !session) return null;

    const isVideo = model.type instanceof ResourceTypeVariantVideo;
    const isFolder = model.type instanceof ResourceTypeVariantFolder;
    const displaySize = formatFileSize(model);
    const isInProgress = resource.completion > 0 && !resource.is_completed;

    return (
        <div className="group flex flex-col bg-transparent overflow-hidden transition-all duration-500 ease-out hover:-translate-y-1">
            {/* Thumbnail Area */}
            <div className="relative aspect-[16/10] bg-[#161616] border border-white/[0.03] rounded-[28px] flex items-center justify-center overflow-hidden transition-all duration-500 group-hover:border-white/[0.08] group-hover:shadow-[0_32px_64px_-16px_rgba(0,0,0,0.6)]">
                {thumbnailSource ? (
                    /* eslint-disable-next-line @next/next/no-img-element */
                    <img 
                        src={thumbnailSource} 
                        alt={model.name} 
                        className="w-full h-full object-cover transition-transform duration-1000 group-hover:scale-110 opacity-80 group-hover:opacity-100"
                    />
                ) : (
                    <div className="flex flex-col items-center gap-3 opacity-[0.08] group-hover:opacity-[0.15] transition-opacity duration-500 text-white">
                        {isFolder ? <Folder className="w-14 h-14 stroke-[1px]" /> : <FileText className="w-14 h-14 stroke-[1px]" />}
                    </div>
                )}

                {/* Overlays */}
                {isVideo && (
                    <div className="absolute inset-0 flex items-center justify-center">
                        <div className="w-14 h-14 rounded-full bg-black/40 backdrop-blur-xl flex items-center justify-center border border-white/10 shadow-2xl group-hover:scale-110 transition-transform duration-500">
                            <Play className="w-5 h-5 text-white fill-white ml-0.5" />
                        </div>
                    </div>
                )}

                {/* Progress Overlay */}
                {isInProgress && (
                    <div className="absolute inset-x-8 bottom-8 h-1.5 bg-black/40 rounded-full overflow-hidden backdrop-blur-md border border-white/5">
                        <div className="h-full bg-bluePrimary shadow-[0_0_12px_rgba(59,130,246,0.5)] transition-all duration-300" style={{ width: `${resource.completion * 100}%` }} />
                    </div>
                )}
            </div>

            {/* Info Area */}
            <div className="py-5 px-2 flex items-center justify-between gap-4">
                <div className="min-w-0 space-y-1">
                    <h3 className="text-[15px] font-bold truncate text-zinc-200 group-hover:text-white transition-colors">
                        {model.name}
                    </h3>
                    <div className="flex items-center gap-2.5 text-[10px] font-bold text-zinc-600 uppercase tracking-[0.12em]">
                        <span>{displaySize}</span>
                        <span className="w-1 h-1 rounded-full bg-zinc-800" />
                        <span className="text-zinc-700">{isFolder ? 'Folder' : (isVideo ? 'Video' : 'Asset')}</span>
                    </div>
                </div>

                <div className="shrink-0">
                    <Button
                        onClick={isInProgress ? handleCancel : handleDownload}
                        size="icon"
                        className={cn(
                            "w-11 h-11 rounded-[16px] transition-all active:scale-90",
                            isInProgress
                                ? "bg-[#1F1F1F] text-white hover:bg-[#252525] border border-white/5"
                                : resource.is_completed
                                    ? "bg-emerald-500/10 text-emerald-500 border border-emerald-500/20"
                                    : "bg-bluePrimary text-white hover:bg-bluePrimary/90 shadow-xl shadow-bluePrimary/20"
                        )}
                    >
                        {isInProgress ? (
                            <span className="text-[10px] font-bold font-mono">{Math.round(resource.completion * 100)}%</span>
                        ) : resource.is_completed ? (
                            <Check className="w-5 h-5" />
                        ) : (
                            <Download className="w-5 h-5" />
                        )}
                    </Button>
                </div>
            </div>
        </div>
    );
}
