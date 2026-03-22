'use client'

import React, {useEffect, useRef, useState} from "react";
import {motion, AnimatePresence} from "motion/react";
import {
    Plus,
    X,
    Upload,
    FileIcon,
    Settings2,
    Check,
    Copy,
    FolderIcon,
    Play
} from 'lucide-react';
import Image from "next/image";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@/components/animate-ui/radix/dropdown-menu";
import {Button} from "@/components/ui/button";
import {Input} from "@/components/ui/input";
import {Label} from "@/components/ui/label";
import {MultiEmailInput} from "@/components/ui/multi-email-input";
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from "@/components/animate-ui/radix/tooltip";
import {
    AppEventVariantTransfer,
    TransferEventVariantStartPublicTransfer,
    TransferEventVariantCancelTransfer, 
    TransferTypeVariantSend,
    ShelfEventVariantAddResources,
    AppEventVariantShelf,
    ShelfEventVariantRemoveResource,
    ResourceTypeVariantFolder,
    LocalResourcePathVariantAbsolutePath,
    ResourceTypeVariantVideo,
    ResourceTypeVariantImage,
    SelectedResourceViewModel
} from 'shared_types/types/shared_types';
import {useFileUpload} from "@/hooks/use-file-upload";
import core from "@/wasm/wasm_core";
import {formatFileSize} from "@/utils/format-file-size";

function ResourceView({ resource, onRemove, isRemoveAllowed }: { resource: SelectedResourceViewModel, onRemove: (id: number) => void, isRemoveAllowed: boolean }) {
    const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);
    const isVideo = resource.type instanceof ResourceTypeVariantVideo;
    const isImage = resource.type instanceof ResourceTypeVariantImage;
    const isFolder = resource.type instanceof ResourceTypeVariantFolder;

    useEffect(() => {
        if ((isVideo || isImage) && resource.thumbnail_path) {
            core.getDownloadUrl(resource.thumbnail_path).then((url) => {
                if (url) setThumbnailUrl(url);
            });
        } else if (resource.thumbnail_path instanceof LocalResourcePathVariantAbsolutePath) {
            const thumbnailValue = resource.thumbnail_path.value;
            requestAnimationFrame(() => setThumbnailUrl(thumbnailValue));
        }
    }, [resource.thumbnail_path, isVideo, isImage]);

    const displayThumbnail = thumbnailUrl || (isFolder ? "/folder.svg" : "/file.svg");

    return (
        <motion.div 
            layout
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            className="aspect-square rounded-2xl bg-white/10 backdrop-blur-xl border border-white/10 flex flex-col items-center justify-center gap-3 relative group overflow-hidden"
        >
            {isRemoveAllowed && (
                <button 
                    onClick={() => onRemove(Number(resource.order_id))}
                    className="absolute top-2 right-2 p-1.5 rounded-lg bg-black/40 text-white/40 opacity-0 group-hover:opacity-100 hover:text-white hover:bg-red-500 transition-all z-10"
                >
                    <X className="w-3 h-3" />
                </button>
            )}

            <div className="w-full h-full absolute inset-0 flex items-center justify-center">
                <Image 
                    src={displayThumbnail} 
                    alt={resource.name} 
                    fill 
                    className={`${thumbnailUrl ? 'object-cover' : 'object-contain p-8'} opacity-60 group-hover:opacity-80 transition-opacity`}
                />
            </div>

            {isVideo && (
                <div className="absolute inset-0 flex items-center justify-center bg-black/20 pointer-events-none">
                    <div className="w-8 h-8 rounded-full bg-white/20 backdrop-blur-md flex items-center justify-center">
                        <Play className="w-4 h-4 text-white fill-white" />
                    </div>
                </div>
            )}

            <div className="absolute bottom-0 left-0 right-0 p-3 bg-gradient-to-t from-black/80 to-transparent">
                <p className="text-white text-[11px] font-bold truncate">{resource.name}</p>
                <p className="text-white/40 text-[9px] font-bold">{formatFileSize(resource)}</p>
            </div>
        </motion.div>
    );
}

export default function SendBoard() {
    const [
        {files, folders, isDragging, supportsDirectories},
        {
            handleDragEnter,
            handleDragLeave,
            handleDragOver,
            handleDrop,
            openFileDialog,
            openDirectoryDialog,
            getInputProps,
            getDirectoryInputProps,
            clearFiles,
            clearFolders
        },
    ] = useFileUpload({
        accept: "*",
        multiple: true,
        allowDirectories: true,
    })

    const selectedResources = core.useSelectedResources()
    const defaultShelfId = core.useDefaultShelfId()
    const isResourceRemoveAllowed = core.useShelfRemoveResourceAllow()
    const [isSettingsOpen, setIsSettingsOpen] = useState(false)
    const [password, setPassword] = useState('')
    const [emails, setEmails] = useState<string[]>([])

    const cloudSession = core.useCloudSession(defaultShelfId)
    const [showTransferUI, setShowTransferUI] = useState(false)
    const [persistentCloudSession, setPersistentCloudSession] = useState(cloudSession)
    const [isInProgressDefer, setIsInProgressDefer] = useState(false)
    const progress = (cloudSession?.progress ?? 0) * 100
    const cloudRef = useRef(cloudSession)

    useEffect(() => {
        cloudRef.current = cloudSession
    }, [cloudSession])

    useEffect(() => {
        if (cloudSession) {
            requestAnimationFrame(() => {
                setShowTransferUI(true)
                setPersistentCloudSession(cloudSession)
            })
        }
    }, [cloudSession])

    useEffect(() => {
        if (!defaultShelfId) return

        if (files.length) {
            core.addFiles(files.map(file => file.file))
                .then((selections) => {
                    core.update(new AppEventVariantShelf(new ShelfEventVariantAddResources(
                        BigInt(defaultShelfId),
                        selections
                    )))
                })
            clearFiles()
        }

        if (folders.length) {
            core.addFolders(folders)
                .then((selections) => {
                    core.update(new AppEventVariantShelf(new ShelfEventVariantAddResources(
                        BigInt(defaultShelfId),
                        selections
                    )))
                })
            clearFolders()
        }
    }, [files, folders, defaultShelfId]);

    useEffect(() => {
        if (cloudSession?.is_in_progress) {
            requestAnimationFrame(() => setIsInProgressDefer(true))
        } else {
            setTimeout(() => {
                if (!cloudRef?.current?.is_in_progress) {
                    setIsInProgressDefer(false)
                }
            }, 2000)
        }
    }, [cloudSession?.is_in_progress]);

    const handleUpload = () => {
        if (defaultShelfId) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantStartPublicTransfer(
                BigInt(defaultShelfId), 
                password || null, 
                emails
            )));
        }
    };

    const handleRemove = async (orderId: number) => {
        if (!isResourceRemoveAllowed || !defaultShelfId) return;
        await core.update(new AppEventVariantShelf(new ShelfEventVariantRemoveResource(
            BigInt(defaultShelfId),
            BigInt(orderId)
        )));
    };

    const headerRef = useRef<HTMLDivElement>(null);

    const scrollToTop = (e: React.MouseEvent) => {
        e.preventDefault();
        headerRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    };

    return (
        <div 
            className="w-full h-full flex flex-col items-center justify-between min-h-[60vh] relative"
            onDragEnter={handleDragEnter}
            onDragLeave={handleDragLeave}
            onDragOver={handleDragOver}
            onDrop={handleDrop}
        >
            <input {...getInputProps()} className="sr-only" />
            <input {...getDirectoryInputProps()} className="sr-only" />
            
            {/* Drag Overlay - Full Page Blur Integrated Design */}
            <AnimatePresence>
                {isDragging && (
                    <motion.div
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                        onDragEnter={(e) => {
                            e.preventDefault()
                            e.stopPropagation()
                        }}
                        onDragOver={(e) => {
                            e.preventDefault()
                            e.stopPropagation()
                        }}
                        onDrop={(e) => {
                            e.preventDefault()
                            e.stopPropagation()
                            handleDrop(e)
                        }}
                        className="fixed inset-0 z-[2000] bg-black/20 backdrop-blur-2xl flex flex-col items-center justify-center"
                    >
                        <motion.div
                            initial={{ scale: 0.9, opacity: 0 }}
                            animate={{ scale: 1, opacity: 1 }}
                            className="flex flex-col items-center gap-6"
                        >
                            <div className="w-24 h-24 rounded-full border-2 border-white/20 flex items-center justify-center">
                                <Upload className="w-10 h-10 text-white" />
                            </div>
                            <h2 className="text-white text-3xl font-bold tracking-tight">Drop to transfer</h2>
                        </motion.div>
                    </motion.div>
                )}
            </AnimatePresence>

            {/* 1. Header Text */}
            <motion.div 
                ref={headerRef}
                initial={{ opacity: 0, y: -20 }}
                animate={{ opacity: 1, y: 0 }}
                className="text-center space-y-4 mb-8"
            >
                <h1 className="text-4xl md:text-7xl font-bold text-white tracking-tight leading-[1.1]">
                    Big transfers, <br />
                    <span className="opacity-40">bigger impact.</span>
                </h1>
                <p className="text-white/60 text-lg md:text-xl max-w-2xl font-medium">
                    The simplest way to send big ideas around the world.
                </p>
            </motion.div>

            {/* 2. Central Interaction Area */}
            <div className="relative w-full flex-1 flex flex-col items-center justify-center mb-24">
                <AnimatePresence mode="wait">
                    {selectedResources.length === 0 ? (
                        /* Empty State: Central Add Files/Folders Button */
                        <div className="relative aspect-square flex items-center justify-center p-8 md:p-24 max-w-[95vw] max-h-[75vh] group/orbit">
                            {/* The Satisfy Hexagon Orbit (720° Interior Sum) - Dashed Border Only */}
                            <div className="absolute inset-0 pointer-events-none flex items-center justify-center">
                                <svg 
                                    className="w-full h-full animate-[spin_80s_linear_infinite] overflow-visible opacity-40" 
                                    viewBox="0 0 100 100"
                                >
                                    <polygon 
                                        points="50 2, 91.5 26, 91.5 74, 50 98, 8.5 74, 8.5 26" 
                                        fill="none" 
                                        stroke="white" 
                                        strokeWidth="0.15" 
                                        strokeDasharray="0.8 1.2"
                                        className="group-hover/orbit:stroke-white/60 transition-colors duration-1000"
                                    />
                                </svg>
                            </div>
                            
                            {/* Inner Content with Rhythmic Spacing */}
                            <div className="flex flex-col md:flex-row gap-12 items-center justify-center relative z-10">
                                <motion.div
                                    key="empty-files"
                                    initial={{ opacity: 0, scale: 0.9 }}
                                    animate={{ opacity: 1, scale: 1 }}
                                    exit={{ opacity: 0, scale: 0.9 }}
                                    className="relative group/card cursor-pointer"
                                    onClick={openFileDialog}
                                >
                                    <div className="relative w-48 h-48 md:w-64 md:h-64 rounded-[24px] bg-white/10 backdrop-blur-3xl border border-white/10 flex flex-col items-center justify-center gap-6 transition-all duration-500 group-hover/card:bg-white/15 group-hover/card:scale-105 group-hover/card:border-white/20 shadow-2xl overflow-hidden">
                                        <div className="absolute inset-0 bg-gradient-to-br from-white/10 to-transparent pointer-events-none" />
                                        <div className="w-16 h-16 rounded-full bg-white flex items-center justify-center shadow-lg group-hover/card:rotate-90 transition-transform duration-700">
                                            <Plus className="w-8 h-8 text-[#555e68]" />
                                        </div>
                                        <div className="text-center px-4">
                                            <p className="text-white font-bold text-lg tracking-tight">Browse or Drop files</p>
                                        </div>
                                    </div>
                                </motion.div>

                                {supportsDirectories && (
                                    <motion.div
                                        key="empty-folders"
                                        initial={{ opacity: 0, scale: 0.9 }}
                                        animate={{ opacity: 1, scale: 1 }}
                                        exit={{ opacity: 0, scale: 0.9 }}
                                        transition={{ delay: 0.1 }}
                                        className="relative group/card cursor-pointer"
                                        onClick={openDirectoryDialog}
                                    >
                                        <div className="relative w-48 h-48 md:w-64 md:h-64 rounded-[24px] bg-white/5 backdrop-blur-2xl border border-white/5 flex flex-col items-center justify-center gap-6 transition-all duration-500 group-hover/card:bg-white/10 group-hover/card:scale-105 group-hover/card:border-white/15 shadow-2xl overflow-hidden">
                                            <div className="absolute inset-0 bg-gradient-to-br from-white/5 to-transparent pointer-events-none" />
                                            <div className="w-16 h-16 rounded-full bg-white/10 flex items-center justify-center shadow-lg group-hover/card:scale-110 transition-transform duration-700 border border-white/10">
                                                <FolderIcon className="w-8 h-8 text-white/60" />
                                            </div>
                                            <div className="text-center px-4">
                                                <p className="text-white/80 font-bold text-lg tracking-tight">Browse or Drop folders</p>
                                            </div>
                                        </div>
                                    </motion.div>
                                )}
                            </div>
                        </div>
                    ) : (
                        /* Files State: Grid of selected resources */
                        <motion.div
                            key="files"
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            className="w-full max-w-4xl grid grid-cols-2 md:grid-cols-4 gap-4 p-4"
                        >
                            {selectedResources.map((resource) => (
                                <ResourceView 
                                    key={resource.order_id} 
                                    resource={resource} 
                                    onRemove={handleRemove} 
                                    isRemoveAllowed={isResourceRemoveAllowed}
                                />
                            ))}
                            
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <motion.button
                                        whileHover={{ scale: 1.05 }}
                                        whileTap={{ scale: 0.95 }}
                                        className="aspect-square rounded-2xl border-2 border-dashed border-white/20 flex flex-col items-center justify-center gap-2 hover:bg-white/5 transition-colors"
                                    >
                                        <Plus className="w-6 h-6 text-white/40" />
                                        <span className="text-white/40 text-[10px] font-bold uppercase">Add more</span>
                                    </motion.button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="center" className="bg-zinc-900 border-white/10 text-white min-w-[160px] p-2 rounded-xl">
                                    <DropdownMenuItem 
                                        onClick={openFileDialog}
                                        className="flex items-center gap-2 p-2 rounded-lg hover:bg-white/10 cursor-pointer transition-colors"
                                    >
                                        <FileIcon className="w-4 h-4 text-white/60" />
                                        <span className="font-bold text-xs uppercase tracking-wider">Add Files</span>
                                    </DropdownMenuItem>
                                    {supportsDirectories && (
                                        <DropdownMenuItem 
                                            onClick={openDirectoryDialog}
                                            className="flex items-center gap-2 p-2 rounded-lg hover:bg-white/10 cursor-pointer transition-colors"
                                        >
                                            <FolderIcon className="w-4 h-4 text-white/60" />
                                            <span className="font-bold text-xs uppercase tracking-wider">Add Folder</span>
                                        </DropdownMenuItem>
                                    )}
                                </DropdownMenuContent>
                            </DropdownMenu>
                        </motion.div>
                    )}
                </AnimatePresence>
            </div>

            {/* 3. Floating Control Bar */}
            <div className="fixed bottom-12 left-1/2 -translate-x-1/2 z-[999] flex flex-col items-center gap-4">
                
                <AnimatePresence>
                    {(isInProgressDefer || showTransferUI) && (
                        <motion.div
                            initial={{ opacity: 0, y: 20 }}
                            animate={{ opacity: 1, y: 0 }}
                            exit={{ opacity: 0, y: 20 }}
                            className="w-[320px] md:w-[450px] p-4 rounded-2xl bg-zinc-900 border border-white/10 shadow-2xl backdrop-blur-xl"
                        >
                            {/* Progress bar - show only when uploading and progress > 0% */}
                            {isInProgressDefer && progress > 0 && (
                                <div className="space-y-3">
                                    <div className="flex items-center justify-between text-[10px] font-bold uppercase tracking-widest text-white/60">
                                        <span>{(cloudSession || persistentCloudSession)?.display_download_speed || 'Uploading to cloud'}</span>
                                        <span>{Math.round(progress)}%</span>
                                    </div>
                                    <div className="h-2 w-full bg-white/10 rounded-full overflow-hidden border border-white/20">
                                        <div
                                            className="h-full bg-white rounded-full transition-all duration-300"
                                            style={{ width: `${progress}%` }}
                                        />
                                    </div>
                                </div>
                            )}
                            {/* URL - always show when available */}
                            {(cloudSession?.access_url || persistentCloudSession?.access_url) && (
                                <div className={`flex items-center gap-3 ${isInProgressDefer && progress > 0 ? 'mt-4' : ''}`}>
                                    <UrlInputWithCopy url={cloudSession?.access_url ?? persistentCloudSession?.access_url ?? ''} />
                                </div>
                            )}
                        </motion.div>
                    )}
                </AnimatePresence>

                <motion.div 
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="flex items-center p-2 rounded-2xl bg-white/10 backdrop-blur-2xl border border-white/20 shadow-2xl"
                >
                    <TooltipProvider>
                        <Tooltip>
                            <TooltipTrigger asChild>
                                <Button 
                                    size="icon" 
                                    variant="ghost" 
                                    onClick={() => setIsSettingsOpen(!isSettingsOpen)}
                                    className={`h-12 w-12 rounded-xl transition-all ${isSettingsOpen ? 'bg-white text-zinc-900' : 'text-white hover:bg-white/10'}`}
                                >
                                    <Settings2 className="w-5 h-5" />
                                </Button>
                            </TooltipTrigger>
                            <TooltipContent side="top" className="bg-zinc-900 border-white/10 text-white font-bold text-[10px] uppercase">
                                More options
                            </TooltipContent>
                        </Tooltip>
                    </TooltipProvider>

                    <div className="w-px h-6 bg-white/10 mx-2" />

                    {/* Stats with Review Button */}
                    <div className="px-4 py-2 flex flex-col justify-center items-center">
                        <span className="text-white font-bold text-sm leading-none">
                            {selectedResources.length} {selectedResources.length === 1 ? 'file' : 'files'}
                        </span>
                        <button
                            disabled={selectedResources.length === 0}
                            onClick={scrollToTop}
                            className="text-[9px] font-bold uppercase tracking-wider mt-1.5 transition-all disabled:text-white/20 text-white/60 hover:text-white active:scale-95 underline decoration-white/20 hover:decoration-white underline-offset-4"
                        >
                            {selectedResources.length > 0 ? 'Review' : 'Selected'}
                        </button>
                    </div>

                    <div className="w-px h-6 bg-white/10 mx-2" />

                    <Button
                        disabled={selectedResources.length === 0 && !cloudSession && !showTransferUI}
                        onClick={() => {
                            if (cloudSession?.is_in_progress && defaultShelfId) {
                                // Cancel - stop the upload
                                core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(
                                    BigInt(cloudSession.session_id),
                                    new TransferTypeVariantSend(BigInt(defaultShelfId))
                                )))
                            } else if (cloudSession?.is_completed || (!cloudSession && showTransferUI)) {
                                // Continue - clear the session to close modal
                                if (defaultShelfId && cloudSession) {
                                    core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(
                                        BigInt(cloudSession.session_id),
                                        new TransferTypeVariantSend(BigInt(defaultShelfId))
                                    )))
                                }
                                setShowTransferUI(false)
                                setPersistentCloudSession(undefined)
                                setIsSettingsOpen(false)
                            } else {
                                handleUpload()
                            }
                        }}
                        className="h-12 px-6 rounded-xl bg-white text-zinc-900 hover:bg-zinc-200 font-bold transition-all disabled:opacity-50 disabled:bg-white/20 group min-w-[140px]"
                    >
                        {cloudSession?.is_in_progress ? null : (cloudSession?.is_completed || (!cloudSession && showTransferUI)) ? (
                            <Check className="w-4 h-4 mr-2" />
                        ) : (
                            <Upload className="w-4 h-4 mr-2 group-hover:-translate-y-0.5 transition-transform" />
                        )}
                        {cloudSession?.is_in_progress ? 'Cancel' : (cloudSession?.is_completed || (!cloudSession && showTransferUI)) ? 'Continue' : 'Send Files'}
                    </Button>
                </motion.div>

                <AnimatePresence>
                    {isSettingsOpen && (
                        <motion.div
                            initial={{ opacity: 0, y: 20, scale: 0.95 }}
                            animate={{ opacity: 1, y: 0, scale: 1 }}
                            exit={{ opacity: 0, y: 20, scale: 0.95 }}
                            className="absolute bottom-[110%] w-[320px] md:w-[400px] p-6 rounded-[32px] bg-zinc-900 border border-white/10 shadow-3xl overflow-hidden"
                        >
                            
                            <div className="relative space-y-6">
                                <div className="flex items-center justify-between">
                                    <h3 className="text-white font-bold tracking-tight">More options</h3>
                                    <button onClick={() => setIsSettingsOpen(false)} className="text-white/40 hover:text-white transition-colors">
                                        <X className="w-4 h-4" />
                                    </button>
                                </div>

                                <div className="space-y-4">
                                    <div className="space-y-2">
                                        <Label className="text-[10px] font-bold uppercase tracking-widest text-white/40 ml-1">Send to emails</Label>
                                        <MultiEmailInput
                                            emails={emails}
                                            onEmailsChange={setEmails}
                                            placeholder="recipient@example.com"
                                            maxEmails={10}
                                            className="bg-white/5 border-white/10 rounded-xl"
                                        />
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-[10px] font-bold uppercase tracking-widest text-white/40 ml-1">Secure Password</Label>
                                        <Input 
                                            type="password"
                                            value={password}
                                            onChange={(e) => setPassword(e.target.value)}
                                            placeholder="••••••••"
                                            className="bg-white/5 border-white/10 rounded-xl h-12 focus:border-white/30"
                                        />
                                    </div>
                                </div>
                            </div>
                        </motion.div>
                    )}
                </AnimatePresence>
            </div>
        </div>
    );
}

function UrlInputWithCopy({url}: { url: string }) {
    const [isCopied, setIsCopied] = useState(false)
    const inputRef = useRef<HTMLInputElement>(null)

    const handleCopy = async () => {
        try {
            await navigator.clipboard.writeText(url)
            setIsCopied(true)
            setTimeout(() => setIsCopied(false), 2000)
        } catch (err) {
            console.error('Failed to copy text: ', err)
        }
    }

    useEffect(() => {
        if (inputRef.current) {
            inputRef.current.scrollLeft = inputRef.current.scrollWidth
        }
    }, [url])

    return (
        <div className="flex-1 relative">
            <Input
                ref={inputRef}
                value={url}
                readOnly
                className="pr-12 cursor-default bg-black/40 text-white border-white/10 rounded-xl h-10 shadow-inner text-xs"
            />
            <button
                onClick={handleCopy}
                className="absolute right-1 top-1/2 transform -translate-y-1/2 p-1.5 rounded-lg hover:bg-white/10 transition-colors"
                title={isCopied ? "Copied!" : "Copy to clipboard"}
            >
                {isCopied ? (
                    <Check className="h-3.5 w-3.5 text-green-400"/>
                ) : (
                    <Copy className="h-3.5 w-3.5 text-white/40 hover:text-white"/>
                )}
            </button>
        </div>
    )
}
