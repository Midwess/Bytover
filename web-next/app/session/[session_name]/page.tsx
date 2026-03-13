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
    ImageUpIcon,
    ShieldAlert
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from "@/components/ui/input";
import core from "@/wasm/wasm_core";
import { ResourceDownload } from "@/app/transfer/components/resource-download";
import { DownloadAllButton } from "@/app/transfer/components/download-all-button";
import { formatFileSize } from "@/utils/format-file-size";
import Header from "@/components/web/header";
import Footer from "@/components/web/footer";
import { SignallingAnimation, DEFAULT_WORDS } from "@/components/ui/signalling-animation";
import { useIsMobile } from "@/hooks/use-mobile";

export default function SessionPage() {
    const params = useParams();
    const sessionName = params.session_name as string;
    const coreReady = core.useCoreReady();
    const coreCompatible = core.useIsCoreCompatible();
    
    // Launch core if not already done
    useEffect(() => {
        if (coreReady && coreCompatible) {
            core.launchNearby();
        }
    }, [coreReady, coreCompatible]);
    
    // Track search results and all sessions to find our target
    const searchSessions = core.useSearchSessionsList();
    const allSessions = core.useAllSessionsList();
    const selectedSession = core.useSelectedSession();
    
    // Find session by alias/name
    useEffect(() => {
        if (coreReady && sessionName) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantFindSession(sessionName)));
        }
    }, [coreReady, sessionName]);

    // Target session from lists
    const targetSessionFromList = useMemo(() => {
        return searchSessions.find(s => s.alias === sessionName) || 
               allSessions.find(s => s.alias === sessionName);
    }, [searchSessions, allSessions, sessionName]);

    // Use either the explicitly selected session (if it matches our alias) or the one found from lists
    const session = useMemo(() => {
        if (selectedSession?.alias === sessionName) return selectedSession;
        return targetSessionFromList;
    }, [selectedSession, targetSessionFromList, sessionName]);

    const isLoading = session?.is_loading;

    // Automatically try to view the session if found and not yet viewing/loading content
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
        <div className="min-h-screen bg-black text-white flex flex-col">
            <Header />
            
            <main className="flex-1 flex flex-col pt-24 pb-12 container mx-auto px-4 md:px-6 w-full">
                {!coreCompatible ? (
                    <div className="flex-1 flex flex-col items-center justify-center gap-6 text-center max-w-md mx-auto">
                        <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-amber-500/20 to-orange-500/20 flex items-center justify-center">
                            <svg className="w-8 h-8 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5}
                                      d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.464 0L4.35 16.5c-.77.833.192 2.5 1.732 2.5z" />
                            </svg>
                        </div>
                        <div className="space-y-2">
                            <h2 className="text-xl font-semibold text-foreground">Browser Not Supported</h2>
                            <p className="text-sm text-muted-foreground leading-relaxed">
                                Please use a modern browser with HTTPS for secure file transfers.
                            </p>
                        </div>
                    </div>
                ) : !session ? (
                    <div className="flex-1 flex flex-col items-center justify-center gap-4">
                        <LoaderCircle className="animate-spin w-8 h-8 text-bluePrimary" />
                        <p className="text-muted-foreground">Finding session "{sessionName}"...</p>
                    </div>
                ) : (
                    <div className="space-y-12 flex-1 flex flex-col">
                        {/* Session Header */}
                        <div className="flex flex-col md:flex-row md:items-end justify-between gap-6 border-b border-white/10 pb-8 shrink-0">
                            <div className="space-y-2">
                                <div className="flex items-center gap-3">
                                    <h1 className="text-3xl md:text-4xl font-bold tracking-tight text-white">
                                        {session.sender_name || 'Session'}
                                    </h1>
                                    <span className="px-2 py-0.5 rounded text-xs font-medium bg-bluePrimary/20 text-bluePrimary border border-bluePrimary/30">
                                        {sessionName}
                                    </span>
                                </div>
                                {session.sender_description && (
                                    <p className="text-zinc-400 max-w-2xl text-sm leading-relaxed">
                                        {session.sender_description}
                                    </p>
                                )}
                            </div>
                            
                            {session.resources?.length > 0 && (
                                <div className="flex items-center gap-4">
                                    <div className="text-right">
                                        <p className="text-sm text-zinc-500">
                                            {session.resources.length} {session.resources.length === 1 ? 'item' : 'items'}
                                        </p>
                                        {(session as ReceiveSessionViewModel).display_download_speed && (
                                            <p className="text-xs text-bluePrimary font-medium">
                                                {(session as ReceiveSessionViewModel).display_download_speed}
                                            </p>
                                        )}
                                    </div>
                                    <DownloadAllButton session={session as ReceiveSessionViewModel} />
                                </div>
                            )}
                        </div>

                        {/* Session Content */}
                        {isLoading && (!session.resources || session.resources.length === 0) ? (
                            <div className="flex-1 flex flex-col items-center justify-center gap-6">
                                {session.password_required && !session.password ? (
                                    <div className="w-full max-w-md p-8 rounded-2xl border border-white/10 bg-zinc-900/50 backdrop-blur-sm space-y-6">
                                        <div className="flex items-center gap-3 text-amber-500">
                                            <ShieldAlert className="w-6 h-6" />
                                            <h2 className="text-xl font-semibold">Password Protected</h2>
                                        </div>
                                        <div className="space-y-4">
                                            <Input
                                                type="password"
                                                placeholder="Enter password to access files"
                                                value={enteredPassword}
                                                onChange={(e) => setEnteredPassword(e.target.value)}
                                                onKeyDown={(e) => e.key === 'Enter' && handlePasswordSubmit()}
                                                className="bg-black/50 border-white/10 h-12"
                                            />
                                            {session.error_message && (
                                                <p className="text-red-500 text-sm">{session.error_message}</p>
                                            )}
                                            <Button 
                                                onClick={handlePasswordSubmit}
                                                className="w-full bg-bluePrimary hover:bg-bluePrimary/90 h-12 font-medium"
                                            >
                                                Access Session
                                            </Button>
                                        </div>
                                    </div>
                                ) : (
                                    <>
                                        <LoaderCircle className="animate-spin w-8 h-8 text-bluePrimary" />
                                        {session.loading_status && (
                                            <p className="text-muted-foreground text-center">
                                                <SignallingAnimation words={[session.loading_status, ...DEFAULT_WORDS]} />
                                            </p>
                                        )}
                                        {session.error_message && (
                                            <div className="flex flex-col items-center gap-4">
                                                <p className="text-red-500">{session.error_message}</p>
                                                <Button onClick={() => window.location.reload()} variant="outline">
                                                    Retry
                                                </Button>
                                            </div>
                                        )}
                                    </>
                                )}
                            </div>
                        ) : (
                            <div className="space-y-12">
                                {images.length > 0 && (
                                    <div className="space-y-6">
                                        <h2 className="text-xl font-bold text-zinc-100 flex items-center gap-2">
                                            {images.length} Image{images.length !== 1 ? 's' : ''}
                                        </h2>
                                        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
                                            {images.map(image => (
                                                <MediaView key={image.model.order_id} id={image.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                                            ))}
                                        </div>
                                    </div>
                                )}

                                {videos.length > 0 && (
                                    <div className="space-y-6">
                                        <h2 className="text-xl font-bold text-zinc-100 flex items-center gap-2">
                                            {videos.length} Video{videos.length !== 1 ? 's' : ''}
                                        </h2>
                                        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
                                            {videos.map(video => (
                                                <MediaView key={video.model.order_id} id={video.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                                            ))}
                                        </div>
                                    </div>
                                )}

                                {files.length > 0 && (
                                    <div className="space-y-6">
                                        <h2 className="text-xl font-bold text-zinc-100 flex items-center gap-2">
                                            {files.length} File{files.length !== 1 ? 's' : ''}
                                        </h2>
                                        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
                                            {files.map(file => (
                                                <FileView key={file.model.order_id} id={file.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                                            ))}
                                        </div>
                                    </div>
                                )}
                            </div>
                        )}
                        
                        {!isLoading && session.resources?.length === 0 && (
                            <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground">
                                <p>No files found in this session.</p>
                            </div>
                        )}
                    </div>
                )}
            </main>

            <Footer />
        </div>
    );
}

function FileView(props: { id: string, isCloud: boolean, sessionId: string }) {
    const { id, isCloud, sessionId } = props;
    const file = core.useReceiveResource(id, isCloud);
    const session = core.useSession(sessionId);
    const model = file?.model;

    const isFolder = model?.type instanceof ResourceTypeVariantFolder;
    const fallbackThumbnail = isFolder ? "/folder.svg" : "/file.svg";

    const [thumbnailSource, setThumbnailSource] = useState<string | undefined>();

    useEffect(() => {
        if (!model?.thumbnail_path) {
            setThumbnailSource(undefined);
            return;
        }

        if (model.thumbnail_path && !thumbnailSource) {
            core.getDownloadUrl(model.thumbnail_path).then(setThumbnailSource);
        }
    }, [model, model?.thumbnail_path, thumbnailSource]);

    if (!file || !model || !session) return null;

    const displaySize = formatFileSize(model);

    return (
        <div className="w-full h-[220px] flex flex-col rounded-xl border border-white/10 bg-zinc-900/40 hover:bg-zinc-900/80 hover:border-bluePrimary/50 transition-all duration-300 overflow-hidden group pointer-events-auto shadow-lg shadow-black/20">
            {/* Card Thumbnail Area (Rectangle) */}
            <div className="relative bg-black/40 h-[calc(100%-80px)] overflow-hidden flex items-center justify-center">
                <div className="w-full h-full flex flex-col items-center justify-center relative p-8">
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img 
                        src={thumbnailSource || fallbackThumbnail} 
                        alt={model.name}
                        className="w-16 h-16 object-contain opacity-40 group-hover:opacity-70 transition-opacity"
                        onError={() => setThumbnailSource(fallbackThumbnail)}
                    />
                </div>
                
                {/* Progress bar overlay if downloading */}
                {file.completion > 0 && file.completion < 1 && (
                    <div className="absolute inset-x-0 bottom-0 h-1.5 bg-white/10">
                        <div 
                            className="h-full bg-bluePrimary transition-all duration-300" 
                            style={{ width: `${file.completion * 100}%` }}
                        />
                    </div>
                )}
            </div>

            {/* File info */}
            <div className="p-4 border-t border-white/5 flex items-center gap-3 h-[80px] shrink-0 bg-zinc-950/20">
                <div className="flex-1 min-w-0">
                    <p className="text-sm font-semibold truncate text-zinc-100 mb-1 group-hover:text-bluePrimary transition-colors">
                        {model.name}
                    </p>
                    <div className="flex items-center gap-2">
                        <p className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest">
                            {displaySize}
                        </p>
                        <span className="text-zinc-800">•</span>
                        <p className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest">
                            {isFolder ? "Folder" : "File"}
                        </p>
                    </div>
                </div>

                {/* Download Button / Progress */}
                <div className="shrink-0 scale-110">
                    <ResourceDownload
                        resource={file}
                        session={session as ReceiveSessionViewModel}
                        size={36}
                        strokeWidth={2.5}
                    />
                </div>
            </div>
        </div>
    );
}

function MediaView(props: { id: string, isCloud: boolean, sessionId: string }) {
    const { id, isCloud, sessionId } = props;
    const media = core.useReceiveResource(id, isCloud);
    const session = core.useSession(sessionId);

    const model = media?.model;
    const isVideo = model?.type instanceof ResourceTypeVariantVideo;
    const isImage = model?.type instanceof ResourceTypeVariantImage;
    const [thumbnailSource, setThumbnailSource] = useState<string | undefined>();

    useEffect(() => {
        if (model?.thumbnail_path) {
            core.getDownloadUrl(model.thumbnail_path).then(setThumbnailSource);
        }
    }, [model?.thumbnail_path]);

    if (!media || !model || !session) return null;

    const displaySize = formatFileSize(model);
    const fallbackThumbnail = isVideo ? "/file-video.svg" : "/file-image.svg";

    return (
        <div className="w-full h-[220px] flex flex-col rounded-xl border border-white/10 bg-zinc-900/40 hover:bg-zinc-900/80 hover:border-bluePrimary/50 transition-all duration-300 overflow-hidden group pointer-events-auto shadow-lg shadow-black/20">
            {/* Thumbnail */}
            <div className="relative bg-black/40 h-[calc(100%-80px)] overflow-hidden flex items-center justify-center">
                {thumbnailSource ? (
                    /* eslint-disable-next-line @next/next/no-img-element */
                    <img
                        className="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
                        alt={model.name}
                        src={thumbnailSource}
                    />
                ) : (
                    <div className="w-full h-full flex flex-col items-center justify-center relative p-8">
                        {/* eslint-disable-next-line @next/next/no-img-element */}
                        <img 
                            src={fallbackThumbnail} 
                            alt={model.name}
                            className="w-16 h-16 object-contain opacity-40 group-hover:opacity-70 transition-opacity"
                        />
                    </div>
                )}

                {/* Video play icon overlay */}
                {isVideo && (
                    <div className="absolute top-3 right-3 bg-black/60 backdrop-blur-md rounded-full p-2 border border-white/10">
                        <Play className="w-3 h-3 text-white fill-white" />
                    </div>
                )}
                
                {/* Progress bar overlay if downloading */}
                {media.completion > 0 && media.completion < 1 && (
                    <div className="absolute inset-x-0 bottom-0 h-1.5 bg-white/10">
                        <div 
                            className="h-full bg-bluePrimary transition-all duration-300" 
                            style={{ width: `${media.completion * 100}%` }}
                        />
                    </div>
                )}
            </div>

            {/* File info */}
            <div className="p-4 border-t border-white/5 flex items-center gap-3 h-[80px] shrink-0 bg-zinc-950/20">
                <div className="flex-1 min-w-0">
                    <p className="text-sm font-semibold truncate text-zinc-100 mb-1 group-hover:text-bluePrimary transition-colors">
                        {model.name}
                    </p>
                    <div className="flex items-center gap-2">
                        <p className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest">
                            {displaySize}
                        </p>
                        <span className="text-zinc-800">•</span>
                        <p className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest">
                            {isVideo ? "Video" : "Image"}
                        </p>
                    </div>
                </div>

                {/* Download Button / Progress */}
                <div className="shrink-0 scale-110">
                    <ResourceDownload
                        resource={media}
                        session={session as ReceiveSessionViewModel}
                        size={36}
                        strokeWidth={2.5}
                    />
                </div>
            </div>
        </div>
    );
}
