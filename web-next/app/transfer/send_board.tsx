'use client'

import {
    DropdownMenuTrigger,
    DropdownMenu, DropdownMenuContent, DropdownMenuCheckboxItem, DropdownMenuItem
} from "@/components/animate-ui/radix/dropdown-menu";
import {
    Globe, ImageUpIcon, Play,
    Users, X, Copy, Check, FolderIcon, MoreVertical, Plus,
} from 'lucide-react'
import {Button} from "@/components/ui/button";
import {ChevronsUpDown} from "lucide-react";
import * as React from "react";
import {Input} from "@/components/ui/input";
import {MultiEmailInput} from "@/components/ui/multi-email-input";
import {Label} from "@/components/ui/label";
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from "@/components/animate-ui/radix/tooltip";
import {MotionEffect} from "@/components/animate-ui/effects/motion-effect";
import {
    AppEventVariantTransfer,
    LocalResourcePathVariantAbsolutePath,
    PeerViewModel,
    ResourceTypeVariantFile,
    ResourceTypeVariantImage,
    ResourceTypeVariantVideo,
    SelectedResourceViewModel,
    TransferEventVariantStartPublicTransfer,
    TransferEventVariantCancelTransfer, TransferTypeVariantSend,
    TransferEventVariantStartTransfer,
    ResourceTypeVariantFolder,
    ShelfEventVariantAddResources,
    AppEventVariantShelf,
    ShelfEventVariantRemoveResource
} from 'shared_types/types/shared_types'
import CircleProgress from "@/components/ui/progress";
import {Avatar, AvatarImage} from "@/components/ui/avatar";
import {useFileUpload} from "@/hooks/use-file-upload";
import {useEffect, useRef, useState} from "react";
import core from "@/wasm/wasm_core";
import {useIsMobile} from "@/hooks/use-mobile";
import clsx from "clsx";
import Image from "next/image";
import {Progress, ProgressTrack} from "@/components/animate-ui/base/progress";
import {
    SidebarProvider,
    SidebarInset,
    SidebarTrigger,
    Sidebar,
    SidebarHeader,
    SidebarContent,
    SidebarRail,
    SidebarMenu,
    SidebarMenuItem,
    SidebarMenuButton,
    useSidebar,
} from '@/components/animate-ui/components/radix/sidebar';
import { Separator } from '@/components/ui/separator';

enum TransferType {
    Public,
    People
}

const activeMethods = [
    {
        name: 'People',
        icon: Users,
        type: TransferType.People
    },
    {
        name: 'Public',
        icon: Globe,
        type: TransferType.Public
    },
]

export default function SendBoard() {
    const [activeMethod, setActiveMethod] = React.useState(activeMethods[0])

    return (
        <div className="rounded-xl border-2 overflow-hidden h-[950px] max-h-[75vh]">
            <SidebarProvider>
                <Sidebar collapsible="icon" className="h-full bg-card overflow-hidden border-2 border-muted rounded-xl mb-1">
                    <SidebarHeader className="rounded-tl-xl">
                        <TransferMethodSelector activeMethod={activeMethod} onActiveMethodChange={setActiveMethod} />
                    </SidebarHeader>
                    <SidebarContentWrapper activeMethod={activeMethod} />
                    <SidebarRail />
                </Sidebar>
                <SidebarInset className="flex flex-col h-[100%]">
                    <header className="flex h-10 md:h-16 shrink-0 items-center gap-2 transition-[width,height] ease-linear group-has-[[data-collapsible=icon]]/sidebar-wrapper:h-12">
                        <div className="flex items-center gap-2 px-4">
                            <SidebarTrigger className="-ml-1" />
                            <Separator orientation="vertical" className="mr-2 h-4" />
                        </div>
                    </header>
                    <div className="flex flex-1 flex-col min-h-0 px-2 pt-0">
                        <FileSelections />
                    </div>
                </SidebarInset>
            </SidebarProvider>
        </div>
    );
}

function FileSelections() {
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

    useEffect(() => {
        if (files.length) {
            core.addFiles(files.map(file => file.file))
                .then((selections) => {
                    core.update(new AppEventVariantShelf(new ShelfEventVariantAddResources(
                        selections
                    )))
                })
            clearFiles()
        }

        if (folders.length) {
            core.addFolders(folders)
                .then((selections) => {
                    core.update(new AppEventVariantShelf(new ShelfEventVariantAddResources(
                        selections
                    )))
                })
            clearFolders()
        }
    }, [files, folders]);

    const isMobile = useIsMobile();

    return (
        <div className="flex flex-col w-full h-full">
            {/* Resource Selection Area */}
            {isMobile ? (
                // Mobile: Dropdown Button
                <div className="relative w-full h-10 flex-shrink-0">
                    <input {...getInputProps()} className="sr-only" aria-label="Upload files" />
                    <input {...getDirectoryInputProps()} className="sr-only" aria-label="Upload folder" />
                    <div className="absolute top-2 right-2">
                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <Button 
                                    size="sm"
                                    className="h-8 w-8 rounded-full bg-bluePrimary text-primaryText hover:bg-bluePrimary/90 p-0"
                                >
                                    <Plus className="h-4 w-4"/>
                                </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                                <DropdownMenuItem
                                    onClick={openFileDialog}
                                >
                                    <ImageUpIcon className="w-4 h-4 mr-2"/>
                                    <span>Select file</span>
                                </DropdownMenuItem>
                                {supportsDirectories && (
                                    <DropdownMenuItem
                                        onClick={openDirectoryDialog}
                                    >
                                        <FolderIcon className="w-4 h-4 mr-2"/>
                                        <span>Select folder</span>
                                    </DropdownMenuItem>
                                )}
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </div>
                </div>
            ) : (
                // Desktop: Two separate drop zones
                <div className="flex gap-2 w-full flex-shrink-0 h-32 md:h-50">
                    <div
                        role="button"
                        onClick={openFileDialog}
                        onDragEnter={handleDragEnter}
                        onDragLeave={handleDragLeave}
                        onDragOver={handleDragOver}
                        onDrop={handleDrop}
                        data-dragging={isDragging || undefined}
                        className="flex-1 flex flex-col items-center justify-center border border-dashed rounded-xl transition-colors cursor-pointer hover:bg-muted-foreground/10 data-[dragging=true]:bg-muted-foreground/10 h-full"
                    >
                        <input {...getInputProps()} className="sr-only" aria-label="Upload files" />
                        <ImageUpIcon className="size-4 opacity-60 mb-2" aria-hidden="true"/>
                        <p className="text-sm font-medium">Drop files or click</p>
                    </div>

                    {supportsDirectories && (
                        <div
                            role="button"
                            onClick={openDirectoryDialog}
                            onDragEnter={handleDragEnter}
                            onDragLeave={handleDragLeave}
                            onDragOver={handleDragOver}
                            onDrop={handleDrop}
                            data-dragging={isDragging || undefined}
                            className="flex-1 flex flex-col items-center justify-center border border-dashed rounded-xl transition-colors cursor-pointer hover:bg-muted-foreground/10 data-[dragging=true]:bg-muted-foreground/10 h-full"
                        >
                            <input
                                {...getDirectoryInputProps()}
                                className="sr-only"
                                aria-label="Upload folder"
                            />
                            <FolderIcon className="size-4 opacity-60 mb-2" aria-hidden="true"/>
                            <p className="text-sm font-medium">Drop folders or click</p>
                        </div>
                    )}
                </div>
            )}

            {/* Resource List with Shadow */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
                {/* Top shadow */}
                <div className="sticky top-0 left-0 right-0 h-8 bg-gradient-to-b from-background to-transparent z-10 pointer-events-none"/>

                {/* Resource grid - single column on mobile, grid on desktop */}
                <div className="flex flex-col md:grid md:grid-cols-2 md:sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-4 gap-2 md:gap-x-3 md:gap-y-1 p-2 md:p-0">
                    {selectedResources.map((resource) => (
                        <div className="md:p-0.5 md:h-[220px] flex items-start flex-row" key={resource.order_id}>
                            <ResourceView model={resource}/>
                        </div>
                    ))}
                    <div className="h-[80px] md:h-[350px] md:block"></div>
                </div>
            </div>
        </div>
    )
}


function ResourceView(props: {
    model: SelectedResourceViewModel
}) {
    const {model} = props;

    const isFile = model.type.constructor == ResourceTypeVariantFile ||
        model.type.constructor == ResourceTypeVariantFolder

    if (isFile) {
        return <FileView model={model}/>
    } else {
        return <MediaView model={model}/>
    }
}

function FileView(props: {
    model: SelectedResourceViewModel
}) {
    const {model} = props;
    const isMobile = useIsMobile();

    let thumbnailPath = (model.thumbnail_path as LocalResourcePathVariantAbsolutePath)?.value;
    const isFolder = model.type instanceof ResourceTypeVariantFolder;
    if (!thumbnailPath) {
        thumbnailPath = isFolder ? "/folder.svg" : "/file.svg";
    }

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    const handleRemove = async () => {
        await core.update(new AppEventVariantShelf(new ShelfEventVariantRemoveResource(BigInt(model.order_id))))
    }

    return (
        <div
            className={clsx(
                "w-full overflow-hidden rounded-2xl flex relative group",
                "bg-muted/60 backdrop-blur-xl border border-white/10",
                "transition-all duration-300 ease-out",
                "hover:scale-[1.02] hover:shadow-2xl hover:shadow-muted/10 hover:border-white/30 hover:backdrop-blur-sm hover:bg-muted/80",
                isMobile ? "flex-row items-center gap-3 p-3 h-auto" : "flex-col h-full"
            )}>
            
            {/* Desktop: Remove button overlay */}
            {!isMobile && (
                <div className="absolute z-20 inset-0 flex items-center justify-center rounded-2xl opacity-0 group-hover:opacity-100 bg-black/60 backdrop-blur-none transition-all duration-300">
                    <Button 
                        size="sm"
                        className="rounded-full bg-black/80 shadow-lg border border-white/20 px-4 text-white" 
                        onClick={handleRemove}>
                        <X className="w-4 h-4"/>
                        <span className="ml-1 text-xs">Remove</span>
                    </Button>
                </div>
            )}

            {/* Mobile: Dropdown menu */}
            {isMobile && (
                <div className="absolute top-1/2 right-2 -translate-y-1/2 z-20">
                    <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                            <Button 
                                size="sm"
                                variant="ghost"
                                className="h-8 w-8 p-0 rounded-full hover:bg-muted/50">
                                <MoreVertical className="h-4 w-4"/>
                            </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                            <DropdownMenuItem
                                onClick={handleRemove}
                                variant="destructive"
                                className="text-destructive"
                            >
                                <X className="w-4 h-4"/>
                                <span>Remove</span>
                            </DropdownMenuItem>
                        </DropdownMenuContent>
                    </DropdownMenu>
                </div>
            )}

            {/* Thumbnail */}
            <div className={clsx(
                "flex items-center justify-center relative",
                isMobile ? "w-16 h-16 shrink-0" : "flex-1 p-3"
            )}>
                <div className={clsx(
                    "relative flex items-center justify-center rounded-2xl transition-all duration-300 bg-white/5 border border-white/10 group-hover:bg-white/10 group-hover:border-white/20 shadow-md",
                    isMobile ? "w-12 h-12" : "w-20 h-20"
                )}>
                    <div className={clsx("relative", isMobile ? "w-10 h-10" : "w-16 h-16")}>
                        <Image
                            className="w-full h-full object-contain drop-shadow-lg transition-transform duration-300 group-hover:scale-110"
                            layout="fill"
                            alt={`${model.type}`}
                            src={thumbnailPath}
                        />
                    </div>
                </div>
            </div>

            {/* File info */}
            <div className={clsx(
                "flex flex-col",
                isMobile ? "flex-1 min-w-0" : "gap-2.5 px-4 pb-4 bg-gradient-to-t from-black/20 to-transparent pt-3"
            )}>
                <p className={clsx(
                    "text-sm font-medium text-white/90 break-words leading-tight",
                    isMobile ? "line-clamp-1 text-left" : "line-clamp-2 text-center"
                )}>
                    {model.name}
                </p>
                <div className={clsx(
                    "flex items-center gap-2",
                    isMobile ? "mt-1" : "justify-center"
                )}>
                    <span className="text-xs px-2 py-0.5 rounded-full border font-medium bg-white/5 border-white/20 text-white/80">
                        {displaySize}
                    </span>
                    <span className="text-xs text-white/50">
                        {isFolder ? "Folder" : "File"}
                    </span>
                </div>
            </div>
        </div>
    );
}

function MediaView(props: {
    model: SelectedResourceViewModel,
}) {
    const {model} = props;

    const isMobile = useIsMobile()
    const isVideo = model.type.constructor == ResourceTypeVariantVideo
    const isImage = model.type.constructor == ResourceTypeVariantImage
    const defaultThumbnail = <Image
        className="w-full h-auto text-primaryText p-10"
        layout="fill"
        alt={`${model.name}`}
        src={'/file.svg'}
    />

    const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null)

    useEffect(() => {
        if ((isVideo || isImage) && model.thumbnail_path) {
            core.getDownloadUrl(model.thumbnail_path).then((it) => {
                if (it) {
                    setThumbnailUrl(it)
                }
            })
        }
    }, []);

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    const handleRemove = () => {
        core.update(new AppEventVariantShelf(new ShelfEventVariantRemoveResource(BigInt(model.order_id))))
    }

    return (
        <div
            className={clsx(
                "w-full overflow-hidden rounded-2xl relative group",
                "border border-white/10 backdrop-blur-sm",
                "transition-all duration-300 ease-out",
                "hover:scale-[1.02] hover:shadow-lg hover:shadow-muted/20 hover:border-white/30",
                isMobile ? "flex flex-row items-center gap-3 p-3 h-auto" : "h-full"
            )}>
            {/* Desktop: Thumbnail - full background */}
            {!isMobile && (
                <>
                    <div className="absolute inset-0 z-0">
                        {thumbnailUrl ? (
                            <Image className="w-full h-full object-cover" fill src={thumbnailUrl} alt={model.name}/>
                        ) : (
                            defaultThumbnail
                        )}
                    </div>
                    {/* Video play icon */}
                    {isVideo && (
                        <div className="absolute top-3 right-3 z-20 bg-black/60 backdrop-blur-md rounded-full p-2 border border-white/20 
                                       transition-all duration-300 group-hover:scale-110 group-hover:bg-white/20">
                            <Play className="w-4 h-4 text-white fill-white"/>
                        </div>
                    )}
                    {/* Gradient overlay */}
                    <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-transparent z-10" />
                    {/* Background overlay for hover effect */}
                    <div className="absolute inset-0 bg-black/40 backdrop-blur-sm z-15 opacity-0 group-hover:opacity-100 transition-all duration-300" />
                </>
            )}

            {/* Mobile: Thumbnail - small square */}
            {isMobile && (
                <div className="w-16 h-16 shrink-0 rounded-xl overflow-hidden relative bg-muted/20">
                    {thumbnailUrl ? (
                        <Image className="w-full h-full object-cover" fill src={thumbnailUrl} alt={model.name}/>
                    ) : (
                        <div className="w-full h-full flex items-center justify-center">
                            <ImageUpIcon className="w-6 h-6 opacity-40"/>
                        </div>
                    )}
                    {isVideo && (
                        <div className="absolute inset-0 flex items-center justify-center bg-black/40">
                            <Play className="w-4 h-4 text-white fill-white"/>
                        </div>
                    )}
                </div>
            )}

            {/* Desktop: Remove button - centered */}
            {!isMobile && (
                <div className="absolute inset-0 flex items-center justify-center z-30 opacity-0 group-hover:opacity-100 transition-all duration-300">
                    <Button
                        size="sm"
                        className="rounded-full bg-black/80 shadow-lg border border-white/20 px-4 text-white"
                        onClick={handleRemove}>
                        <X className="w-4 h-4"/>
                        <span className="ml-1 text-xs">Remove</span>
                    </Button>
                </div>
            )}

            {/* Mobile: Dropdown menu */}
            {isMobile && (
                <div className="absolute top-1/2 right-2 -translate-y-1/2 z-30">
                    <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                            <Button 
                                size="sm"
                                variant="ghost"
                                className="h-8 w-8 p-0 rounded-full bg-black/60 hover:bg-black/80">
                                <MoreVertical className="h-4 w-4 text-white"/>
                            </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                            <DropdownMenuItem
                                onClick={handleRemove}
                                variant="destructive"
                                className="text-destructive"
                            >
                                <X className="w-4 h-4"/>
                                <span>Remove</span>
                            </DropdownMenuItem>
                        </DropdownMenuContent>
                    </DropdownMenu>
                </div>
            )}

            {/* File info */}
            <div className={clsx(
                "flex flex-col z-20",
                isMobile 
                    ? "flex-1 min-w-0" 
                    : "absolute bottom-0 left-0 right-0 p-3 bg-gradient-to-t from-black/60 to-transparent backdrop-blur-sm"
            )}>
                <p className={clsx(
                    "text-white text-sm font-medium leading-tight",
                    isMobile ? "line-clamp-1 text-left" : "line-clamp-2"
                )}>
                    {model.name}
                </p>
                <div className={clsx(
                    "flex items-center gap-2",
                    isMobile ? "mt-1" : ""
                )}>
                    <span className="text-xs px-2 py-0.5 rounded-full border font-medium bg-white/5 border-white/20 text-white/80">
                        {displaySize}
                    </span>
                    <span className="text-xs text-white/60">
                        {isVideo ? "Video" : "Image"}
                    </span>
                </div>
            </div>
        </div>
    );
}

function TransferMethodSelector({ activeMethod, onActiveMethodChange }: { activeMethod: typeof activeMethods[0], onActiveMethodChange: (method: typeof activeMethods[0]) => void }) {
    const isMobile = useIsMobile();

    return (
        <SidebarMenu>
            <SidebarMenuItem>
                <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                        <SidebarMenuButton
                            size="lg"
                            className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
                        >
                            <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
                                <activeMethod.icon className="size-4" />
                            </div>
                            <div className="grid flex-1 text-left text-sm leading-tight">
                                <span className="truncate font-semibold">
                                    {activeMethod.name}
                                </span>
                            </div>
                            <ChevronsUpDown className="ml-auto" />
                        </SidebarMenuButton>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent
                        className="w-[--radix-dropdown-menu-trigger-width] min-w-56 rounded-lg"
                        align="start"
                        side={isMobile ? 'bottom' : 'right'}
                        sideOffset={4}
                    >
                        {activeMethods.map((method) => (
                            <DropdownMenuCheckboxItem
                                key={method.name}
                                checked={activeMethod === method}
                                onCheckedChange={() => onActiveMethodChange(method)}
                                className="gap-2 p-2"
                            >
                                <method.icon className="size-4" />
                                {method.name}
                            </DropdownMenuCheckboxItem>
                        ))}
                    </DropdownMenuContent>
                </DropdownMenu>
            </SidebarMenuItem>
        </SidebarMenu>
    );
}

function SidebarContentWrapper({ activeMethod }: { activeMethod: typeof activeMethods[0] }) {
    const { state } = useSidebar();
    
    if (state === 'collapsed') {
        return null;
    }
    
    return (
        <SidebarContent className="rounded-bl-xl px-1">
            <TransferForm activeMethod={activeMethod} />
        </SidebarContent>
    );
}

function TransferForm({ activeMethod }: { activeMethod: typeof activeMethods[0] }) {
    const content = activeMethod.type === TransferType.Public
        ? <PublicSend/>
        : <NearbySend/>

    return (
        <div className={"px-2 flex flex-col items-center justify-center pt-5 h-fit"}>
            {content}
        </div>
    )
}

function PublicSend() {
    const [password, setPassword] = useState('')
    const cloudSession = core.useTransferState()?.cloud_session
    const [isInProgressDefer, setIsInProgressDefer] = useState(false)
    const [isInProgress, setIsInProgress] = useState(false)
    const progress = (cloudSession?.progress ?? 0) * 100
    const cloudRef = useRef(cloudSession)
    cloudRef.current = cloudSession

    useEffect(() => {
        if (cloudSession?.is_in_progress) {
            setIsInProgress(true)
            setIsInProgressDefer(true)
        } else {
            setIsInProgress(false)
            setTimeout(() => {
                if (!cloudRef?.current?.is_in_progress) {
                    setIsInProgressDefer(false)
                }
            }, 2000)
        }
    }, [cloudSession?.is_in_progress])

    return <div className={"flex flex-col w-full h-full items-center gap-10 justify-center mt-1"}>
        <MotionEffect
            className={"flex flex-col w-full gap-3"}
            slide={{
                direction: 'down',
            }}
            fade
            zoom
            inView
            delay={0.2}>
            <p className="text-start w-full text-primaryText/70 text-sm">
                Create a sharable URL. Protect access by setting a password to keep your content safe.
            </p>

            <div className={"flex flex-col w-full gap-3"}>
                <Label htmlFor={"password"}>Password (optional)</Label>
                <Input id={"password"} disabled={isInProgress} value={password}
                       onChange={(it) => setPassword(it.target.value)}
                       type={"password"} maxLength={20} placeholder={"pwd@123"}/>
                {
                    cloudSession?.access_url &&
                    <>
                        <Label>Generated url</Label>
                        <UrlInputWithCopy url={cloudSession?.access_url ?? ''}/>
                    </>
                }
                {
                    isInProgressDefer
                    && <div className={"flex flex-col w-full gap-2"}>
                        <Progress value={progress} className="w-full space-y-2">
                            <div className="flex items-center justify-between gap-1">
                                    <span className="text-sm">
                                        {cloudSession?.display_download_speed}
                                    </span>
                            </div>
                            <ProgressTrack/>
                        </Progress>
                    </div>
                }
                {
                    isInProgress &&
                    <Button className="mt-2 w-fit h-[35px] bg-muted-foreground text-primary" onClick={() => {
                        if (cloudSession?.is_in_progress) {
                            core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(BigInt(cloudSession.session_id), new TransferTypeVariantSend())))
                        }
                    }}>Cancel</Button>
                }
                {
                    !cloudSession &&
                    <Button className="w-fit h-[35px] bg-bluePrimary text-primary" onClick={() => {
                        core.update(new AppEventVariantTransfer(new TransferEventVariantStartPublicTransfer(password, [])))
                    }}>Upload</Button>
                }
                {
                    cloudSession?.is_completed &&
                    <Button className="w-fit h-[35px] bg-greenSecondary/40 text-primary" onClick={() => {
                        core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(
                            BigInt(cloudSession?.session_id),
                            new TransferTypeVariantSend()
                        )))
                    }}>Continue</Button>
                }
            </div>
        </MotionEffect>
    </div>
}

function UrlInputWithCopy({url}: { url: string }) {
    const [isCopied, setIsCopied] = useState(false)

    const handleCopy = async () => {
        try {
            await navigator.clipboard.writeText(url)
            setIsCopied(true)
            setTimeout(() => setIsCopied(false), 2000) // Reset after 2 seconds
        } catch (err) {
            console.error('Failed to copy text: ', err)
        }
    }

    // Function to trim from the center
    const getTrimmedUrl = (url: string, maxLength: number = 40) => {
        if (url.length <= maxLength) return url

        const ellipsis = '...'
        const availableLength = maxLength - ellipsis.length
        const frontLength = Math.ceil(availableLength / 2)
        const backLength = Math.floor(availableLength / 2)

        return url.slice(0, frontLength) + ellipsis + url.slice(-backLength)
    }

    return (
        <TooltipProvider>
            <div className="relative">
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Input
                            value={getTrimmedUrl(url)}
                            disabled={true}
                            className="pr-12 cursor-default" // Add padding for the button and cursor
                        />
                    </TooltipTrigger>
                    <TooltipContent
                        side="top"
                        className="max-w-xs break-all"
                    >
                        {url}
                    </TooltipContent>
                </Tooltip>
                <button
                    onClick={handleCopy}
                    className="absolute right-2 top-1/2 transform -translate-y-1/2 p-1.5 rounded-md hover:bg-muted transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
                    title={isCopied ? "Copied!" : "Copy to clipboard"}
                >
                    {isCopied ? (
                        <Check className="h-4 w-4 text-green-500"/>
                    ) : (
                        <Copy className="h-4 w-4 text-muted-foreground hover:text-foreground"/>
                    )}
                </button>
            </div>
        </TooltipProvider>
    )
}

function NearbySend() {
    const nearbyState = window.core.useNearbyState()
    const nearbyPeers = nearbyState?.peers || []
    const [emails, setEmails] = React.useState<string[]>([])
    const cloudSession = core.useTransferState()?.cloud_session
    const [isInProgressDefer, setIsInProgressDefer] = useState(false)
    const [isInProgress, setIsInProgress] = useState(false)
    const progress = (cloudSession?.progress ?? 0) * 100
    const cloudRef = useRef(cloudSession)
    cloudRef.current = cloudSession

    useEffect(() => {
        if (cloudSession?.is_in_progress) {
            setIsInProgress(true)
            setIsInProgressDefer(true)
        } else {
            setIsInProgress(false)
            setTimeout(() => {
                if (!cloudRef?.current?.is_in_progress) {
                    setIsInProgressDefer(false)
                }
            }, 2000)
        }

    }, [cloudSession?.is_in_progress])

    return <>
        <MotionEffect
            className="flex flex-col w-full gap-3"
            slide={{direction: 'down'}}
            fade
            zoom
            inView
            delay={0.2}>

            <p className="text-start w-full text-primaryText/70 text-sm pb-1">
                Send files directly to a friend’s email
            </p>

            <div className="flex flex-col w-full gap-3">
                <MultiEmailInput
                    emails={emails}
                    onEmailsChange={setEmails}
                    placeholder="Enter email addresses..."
                    maxEmails={10}
                    disabled={isInProgress}
                />
                {
                    isInProgressDefer
                    && <div className={"flex flex-col w-full gap-2"}>
                        <Progress value={progress} className="w-full space-y-2">
                            <div className="flex items-center justify-between gap-1">
                                    <span className="text-sm">
                                        {cloudSession?.display_download_speed}
                                    </span>
                            </div>
                            <ProgressTrack/>
                        </Progress>
                    </div>
                }
                {
                    isInProgress &&
                    <Button className="mt-2 w-fit h-[35px] bg-muted-foreground text-primary" onClick={() => {
                        if (cloudSession?.is_in_progress) {
                            core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(BigInt(cloudSession.session_id), new TransferTypeVariantSend())))
                        }
                    }}>Cancel</Button>
                }
                {
                    !cloudSession &&
                    <Button
                        className="w-fit h-[35px] bg-bluePrimary text-primary"
                        disabled={emails.length === 0}
                        onClick={() => {
                            core.update(new AppEventVariantTransfer(new TransferEventVariantStartPublicTransfer(null, emails)))
                        }}
                    >
                        Send
                        to {emails.length > 0 ? `${emails.length} recipient${emails.length > 1 ? 's' : ''}` : 'Email'}
                    </Button>
                }
                {
                    cloudSession?.is_completed &&
                    <Button className="w-fit h-[35px] bg-greenSecondary/40 text-primary" onClick={() => {
                        core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(
                            BigInt(cloudSession?.session_id),
                            new TransferTypeVariantSend()
                        )))
                    }}>Continue</Button>
                }
            </div>

            <div className="flex flex-col w-full gap-3 mt-5">
                <p className="text-start w-full text-primaryText/70 text-sm pb-1">
                    Or share with nearby friends and devices
                </p>
                {nearbyPeers.map((peer) => (
                    <NearbyPeer key={peer.id} peer={peer}/>
                ))}
            </div>
        </MotionEffect>
    </>
}

function NearbyPeer(props: { peer: PeerViewModel }) {
    const peer = core.usePeerState(props.peer?.id) || props.peer
    const color = `rgb(${peer.avatar.dominant_color_r}, ${peer.avatar.dominant_color_g}, ${peer.avatar.dominant_color_b})`

    return <>
        <div
            className={"flex flex-row bg-muted hover:bg-muted-foreground/30 rounded-2xl items-center px-2 py-2 h-fit w-full border-1 border-primaryText/5 justify-between"}
            onClick={() => {
                core.update(new AppEventVariantTransfer(new TransferEventVariantStartTransfer(peer.id)))
            }}>
            <div className={"flex flex-row items-center gap-3"}>
                <div
                    className={"bg-bluePrimary rounded-xl aspect-square justify-center items-center text-primaryText flex h-[34px] w-[34px]"}>
                    <Avatar className={"p-1 rounded-xl"} style={{backgroundColor: color}}>
                        <AvatarImage src={peer.avatar.url}/>
                    </Avatar>
                </div>
                <div className={"flex flex-col gap-1 items-start"}>
                    <p className={"text-primaryText font-bold text-sm"}>{peer.display_name}</p>
                    {
                        peer.display_upload_speed
                            ? <p className={"text-primaryText/70 text-xs"}>{peer.display_upload_speed}</p>
                            : <></>
                    }
                </div>
            </div>
            {
                <div className={"w-[40px] h-[40px] flex justify-center items-center"}>
                    {peer.transfer_progress ? <CircleProgress progress={peer.transfer_progress} size={35}/> : <></>}
                </div>
            }
        </div>
    </>
}