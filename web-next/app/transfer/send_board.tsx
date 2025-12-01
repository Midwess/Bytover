'use client'

import {
    DropdownMenuTrigger,
    DropdownMenu, DropdownMenuContent, DropdownMenuCheckboxItem, DropdownMenuItem
} from "@/components/animate-ui/radix/dropdown-menu";
import {
    Globe, ImageUpIcon, Play,
    Users, X, Copy, Check, FolderIcon, MoreVertical, Plus,
} from 'lucide-react'
import { Button } from "@/components/ui/button";
import { ChevronsUpDown } from "lucide-react";
import * as React from "react";
import { Input } from "@/components/ui/input";
import { MultiEmailInput } from "@/components/ui/multi-email-input";
import { Label } from "@/components/ui/label";
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from "@/components/animate-ui/radix/tooltip";
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
import { Avatar, AvatarImage } from "@/components/ui/avatar";
import { useFileUpload } from "@/hooks/use-file-upload";
import { useEffect, useRef, useState } from "react";
import core from "@/wasm/wasm_core";
import { useIsMobile } from "@/hooks/use-mobile";
import clsx from "clsx";
import Image from "next/image";
import { Progress, ProgressTrack } from "@/components/animate-ui/base/progress";
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
import { Switch } from '@/components/ui/switch';

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
        <div className="rounded-xl border-2 overflow-hidden max-h-[70vh] sm:max-h-[80vh] min-h-[450px] h-[950px]">
            <SidebarProvider className="h-[100%]">
                <Sidebar collapsible="icon" className="h-full bg-card overflow-hidden border-2 border-muted rounded-xl mb-1">
                    <SidebarHeader className="rounded-tl-xl">
                        <TransferMethodSelector activeMethod={activeMethod} onActiveMethodChange={setActiveMethod} />
                    </SidebarHeader>
                    <SidebarContentWrapper activeMethod={activeMethod} />
                    <SidebarRail />
                </Sidebar>
                <SidebarInset className="flex flex-col h-[100%] min-h-0">
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
        { files, folders, isDragging, supportsDirectories },
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
                // Mobile: Dropdown Button (only show when resources exist)
                selectedResources.length > 0 && (
                    <div className="relative w-full flex-shrink-0 h-[50px]">
                        <input {...getInputProps()} className="sr-only" aria-label="Upload files" />
                        <input {...getDirectoryInputProps()} className="sr-only" aria-label="Upload folder" />
                        <div className="absolute top-2 right-2">
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <Button
                                        size="sm"
                                        className="h-8 w-8 rounded-full bg-bluePrimary text-primaryText hover:bg-bluePrimary/90 p-0"
                                    >
                                        <Plus className="h-4 w-4" />
                                    </Button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end">
                                    <DropdownMenuItem
                                        onClick={openFileDialog}
                                    >
                                        <ImageUpIcon className="w-4 h-4 mr-2" />
                                        <span>Select file</span>
                                    </DropdownMenuItem>
                                    {supportsDirectories && (
                                        <DropdownMenuItem
                                            onClick={openDirectoryDialog}
                                        >
                                            <FolderIcon className="w-4 h-4 mr-2" />
                                            <span>Select folder</span>
                                        </DropdownMenuItem>
                                    )}
                                </DropdownMenuContent>
                            </DropdownMenu>
                        </div>
                    </div>
                )
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
                        <ImageUpIcon className="size-4 opacity-60 mb-2" aria-hidden="true" />
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
                            <FolderIcon className="size-4 opacity-60 mb-2" aria-hidden="true" />
                            <p className="text-sm font-medium">Drop folders or click</p>
                        </div>
                    )}
                </div>
            )}

            <div className="h-fit max-h-[95%] overflow-y-auto overflow-x-hidden w-full">
                <div className="sticky top-0 left-0 right-0 h-8 bg-gradient-to-b from-background to-transparent z-10 pointer-events-none" />

                {/* Resource grid - single column on mobile, grid on desktop */}
                {selectedResources.length === 0 ? (
                    <div className="flex flex-col items-center justify-center min-h-[200px] text-muted-foreground/50 gap-4">
                        <p className="text-lg font-medium">No selected resources</p>
                        {isMobile ? (
                            <>
                                <input {...getInputProps()} className="sr-only" aria-label="Upload files" />
                                <input {...getDirectoryInputProps()} className="sr-only" aria-label="Upload folder" />
                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button
                                            size="sm"
                                            className="h-10 w-10 rounded-full bg-bluePrimary text-primaryText hover:bg-bluePrimary/90 p-0"
                                        >
                                            <Plus className="h-5 w-5" />
                                        </Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent align="center">
                                        <DropdownMenuItem
                                            onClick={openFileDialog}
                                        >
                                            <ImageUpIcon className="w-4 h-4 mr-2" />
                                            <span>Select file</span>
                                        </DropdownMenuItem>
                                        {supportsDirectories && (
                                            <DropdownMenuItem
                                                onClick={openDirectoryDialog}
                                            >
                                                <FolderIcon className="w-4 h-4 mr-2" />
                                                <span>Select folder</span>
                                            </DropdownMenuItem>
                                        )}
                                    </DropdownMenuContent>
                                </DropdownMenu>
                            </>
                        ) : null}
                    </div>
                ) : (
                    <div className="flex flex-col md:grid md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 gap-2 md:gap-4 p-2 md:px-1">
                        {selectedResources.map((resource) => (
                            <div className="md:h-[230px] flex items-start flex-row" key={resource.order_id}>
                                <ResourceView model={resource} />
                            </div>
                        ))}
                    </div>
                )}
                {selectedResources.length > 0 && <div className="h-[50px] sm:h-[80px]"></div>}
            </div>
        </div>
    )
}


function ResourceView(props: {
    model: SelectedResourceViewModel
}) {
    const { model } = props;

    const isFile = model.type.constructor == ResourceTypeVariantFile ||
        model.type.constructor == ResourceTypeVariantFolder

    if (isFile) {
        return <FileView model={model} />
    } else {
        return <MediaView model={model} />
    }
}

function FileView(props: {
    model: SelectedResourceViewModel
}) {
    const { model } = props;
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
                isMobile ? "flex-row items-center gap-3 p-1.5 h-auto" : "flex-col h-full"
            )}>

            {/* Desktop: Remove button overlay */}
            {!isMobile && (
                <div className="absolute z-20 inset-0 flex items-center justify-center rounded-2xl opacity-0 group-hover:opacity-100 bg-black/60 backdrop-blur-none transition-all duration-300">
                    <Button
                        size="sm"
                        className="rounded-full bg-black/80 shadow-lg border border-white/20 px-4 text-white"
                        onClick={handleRemove}>
                        <X className="w-4 h-4" />
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
                                <MoreVertical className="h-4 w-4" />
                            </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                            <DropdownMenuItem
                                onClick={handleRemove}
                                variant="destructive"
                                className="text-destructive"
                            >
                                <X className="w-4 h-4" />
                                <span>Remove</span>
                            </DropdownMenuItem>
                        </DropdownMenuContent>
                    </DropdownMenu>
                </div>
            )}

            {/* Thumbnail */}
            <div className={clsx(
                "flex items-center justify-center relative",
                isMobile ? "w-14 h-14 shrink-0" : "flex-1 p-2.5"
            )}>
                <div className={clsx(
                    "relative flex items-center justify-center rounded-2xl transition-all duration-300 bg-white/5 border border-white/10 group-hover:bg-white/10 group-hover:border-white/20 shadow-md",
                    isMobile ? "w-12 h-12" : "w-24 h-24"
                )}>
                    <div className={clsx("relative", isMobile ? "w-10 h-10" : "w-20 h-20")}>
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
                isMobile ? "flex-1 min-w-0" : "gap-2.5 px-2 pb-3 bg-gradient-to-t from-black/20 to-transparent pt-2.5"
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
    const { model } = props;

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
                "border border-white/10",
                "transition-all duration-300 ease-out",
                "hover:scale-[1.02] hover:shadow-lg hover:shadow-muted/20 hover:border-white/30",
                isMobile ? "flex flex-row items-center gap-3 p-1.5 h-auto bg-muted/60 backdrop-blur-xl" : "h-full"
            )}>
            {/* Desktop: Thumbnail - full background */}
            {!isMobile && (
                <>
                    <div className="absolute inset-0 z-0">
                        {thumbnailUrl ? (
                            <Image className="w-full h-full object-cover" fill src={thumbnailUrl} alt={model.name} />
                        ) : (
                            defaultThumbnail
                        )}
                    </div>
                    {/* Video play icon */}
                    {isVideo && (
                        <div className="absolute top-3 right-3 z-20 bg-black/60 backdrop-blur-md rounded-full p-2 border border-white/20 
                                       transition-all duration-300 group-hover:scale-110 group-hover:bg-white/20">
                            <Play className="w-4 h-4 text-white fill-white" />
                        </div>
                    )}
                    {/* Gradient overlay */}
                    <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-transparent z-10" />
                    {/* Background overlay for hover effect */}
                    <div className="absolute inset-0 bg-black/40 backdrop-blur-sm z-15 opacity-0 group-hover:opacity-100 transition-all duration-300 rounded-2xl" />
                </>
            )}

            {/* Mobile: Thumbnail - small square */}
            {isMobile && (
                <div className="w-14 h-14 shrink-0 rounded-xl overflow-hidden relative bg-muted/20">
                    {thumbnailUrl ? (
                        <Image className="w-full h-full object-cover" fill src={thumbnailUrl} alt={model.name} />
                    ) : (
                        <div className="w-full h-full flex items-center justify-center">
                            <ImageUpIcon className="w-6 h-6 opacity-40" />
                        </div>
                    )}
                    {isVideo && (
                        <div className="absolute inset-0 flex items-center justify-center bg-black/40">
                            <Play className="w-4 h-4 text-white fill-white" />
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
                        <X className="w-4 h-4" />
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
                                className="h-8 w-8 p-0 rounded-full hover:bg-muted/50">
                                <MoreVertical className="h-4 w-4" />
                            </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                            <DropdownMenuItem
                                onClick={handleRemove}
                                variant="destructive"
                                className="text-destructive"
                            >
                                <X className="w-4 h-4" />
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
                    : "absolute bottom-0 left-0 right-0 p-3 bg-gradient-to-t from-black/60 to-transparent backdrop-blur-sm rounded-b-2xl"
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
        ? <PublicSend />
        : <NearbySend />

    return (
        <div className={"px-2 flex flex-col items-center justify-center pt-5 h-fit"}>
            {content}
        </div>
    )
}

function PublicSend() {
    const [password, setPassword] = useState('')
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

    return <div className={"flex flex-col w-full h-full items-center gap-10 justify-center mt-1"}>
        <div className={"flex flex-col w-full gap-3"}>
            <p className="text-start w-full text-primaryText/70 text-sm">
                Create a sharable URL or send files directly to email addresses. Optionally protect with a password.
            </p>

            <div className={"flex flex-col w-full gap-3"}>
                <Label htmlFor={"emails"}>Send to emails (optional)</Label>
                <MultiEmailInput
                    emails={emails}
                    onEmailsChange={setEmails}
                    placeholder="Enter email addresses..."
                    maxEmails={10}
                    disabled={isInProgress}
                />
                <Label htmlFor={"password"}>Password (optional)</Label>
                <Input id={"password"} disabled={isInProgress} value={password}
                    onChange={(it) => setPassword(it.target.value)}
                    type={"password"} maxLength={20} placeholder={"pwd@123"} />
                {
                    cloudSession?.access_url &&
                    <>
                        <Label>Generated url</Label>
                        <UrlInputWithCopy url={cloudSession?.access_url ?? ''} />
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
                            <ProgressTrack />
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
                        core.update(new AppEventVariantTransfer(new TransferEventVariantStartPublicTransfer(password || null, emails)))
                    }}>
                        {emails.length > 0
                            ? `Send to ${emails.length} recipient${emails.length > 1 ? 's' : ''}`
                            : 'Upload'}
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
        </div>
    </div>
}

function UrlInputWithCopy({ url }: { url: string }) {
    const [isCopied, setIsCopied] = useState(false)
    const inputRef = useRef<HTMLInputElement>(null)

    const handleCopy = async () => {
        try {
            await navigator.clipboard.writeText(url)
            setIsCopied(true)
            setTimeout(() => setIsCopied(false), 2000) // Reset after 2 seconds
        } catch (err) {
            console.error('Failed to copy text: ', err)
        }
    }

    // Scroll to the end of the input to show the last part of the URL
    useEffect(() => {
        if (inputRef.current) {
            inputRef.current.scrollLeft = inputRef.current.scrollWidth
        }
    }, [url])

    return (
        <TooltipProvider>
            <div className="relative">
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Input
                            ref={inputRef}
                            value={url}
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
                        <Check className="h-4 w-4 text-green-500" />
                    ) : (
                        <Copy className="h-4 w-4 text-muted-foreground hover:text-foreground" />
                    )}
                </button>
            </div>
        </TooltipProvider>
    )
}

function NearbySend() {
    const nearbyState = window.core.useNearbyState()
    const nearbyPeers = nearbyState?.peers || []

    return <>
        <div className="flex flex-col w-full gap-3">
            <p className="text-start w-full text-primaryText/70 text-sm pb-1">
                Share with nearby friends and devices
            </p>
            {/* Current User Info */}
            <MyPeerInfo />
            <div className="flex flex-col w-full gap-3">
                {nearbyPeers.length > 0 && <span className="text-start w-full text-muted-foreground text-sm font-medium pb-1">Peers:</span>}
                {nearbyPeers.map((peer) => (
                    <NearbyPeer key={peer.id} peer={peer} />
                ))}
            </div>
        </div>
    </>
}

function MyPeerInfo() {
    const myPeer = core.useMyPeer()
    const isLocationEnabled = core.useLocationEnabled()
    const isLocationLoading = core.useLocationLoading()

    // Request location on mount
    useEffect(() => {
        core.enableLocation()
    }, [])

    if (!myPeer) {
        return (
            <div className="w-full mb-6">
                <div className="relative overflow-hidden rounded-2xl backdrop-blur-sm">
                    <div className="flex items-center justify-center gap-3 py-2">
                        <div className="h-4 w-4 animate-spin rounded-full border-2 border-white/20 border-t-white"></div>
                        <span className="text-sm font-medium text-muted-foreground animate-pulse">Initializing...</span>
                    </div>
                </div>
            </div>
        )
    }

    const color = `rgb(${myPeer.avatar.dominant_color_r}, ${myPeer.avatar.dominant_color_g}, ${myPeer.avatar.dominant_color_b})`

    const handleLocationToggle = async (checked: boolean) => {
        if (checked) {
            // Reset and request fresh permission
            await core.enableLocation()
        } else {
            core.disableLocation()
        }
    }

    return (
        <div className="flex flex-col w-full gap-3">
            <div className="flex flex-row rounded-2xl items-center w-full">
                <div className="flex flex-row items-center gap-5 justify-between flex-1 rounded-xl">
                    <div className="flex flex-col gap-[0.5] items-start">
                        <p className="text-start w-full text-primaryText/70 text-xs">
                            You&apos;re online as
                        </p>
                        <p className="text-primaryText font-bold text-sm">{myPeer.display_name}</p>
                    </div>
                    <div className="relative aspect-square justify-center items-center text-primaryText flex h-[40px] w-[40px] border-greenSecondary p-3 border-2 rounded-2xl">
                        <Avatar className="p-1 rounded-xl" style={{ backgroundColor: color }}>
                            <AvatarImage src={myPeer.avatar.url} />
                        </Avatar>
                        {/* Online status indicator */}
                        <div className="absolute -bottom-0.5 -right-0.5 w-3 h-3 bg-greenSecondary rounded-full border-2 border-background" />
                    </div>
                </div>
            </div>

            {/* Location Toggle */}
            <div className="flex items-center justify-between py-2 border-t border-white/10">
                <div className="flex flex-col gap-0.5">
                    <span className="text-sm font-medium text-primaryText">Allow Location</span>
                    <span className="text-xs text-muted-foreground">
                        {isLocationLoading ? "Getting location..." : "Helps find nearby devices"}
                    </span>
                </div>
                <div className="flex items-center gap-2">
                    {isLocationLoading && (
                        <div className="h-3 w-3 animate-spin rounded-full border-2 border-white/20 border-t-white"></div>
                    )}
                    <Switch
                        checked={isLocationEnabled}
                        onCheckedChange={handleLocationToggle}
                        disabled={isLocationLoading}
                    />
                </div>
            </div>
        </div>
    )
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
                    <Avatar className={"p-1 rounded-xl"} style={{ backgroundColor: color }}>
                        <AvatarImage src={peer.avatar.url} />
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
                    <CircleProgress isCompleted={!Number(peer.transfer_progress)} isInProgress={!!peer.transfer_progress && peer.transfer_progress < 1} progress={peer.transfer_progress} size={35} strokeWidth={4} />
                </div>
            }
        </div>
    </>
}