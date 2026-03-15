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
import Link from "next/link";
import {Input} from "@/components/ui/input";
import {MultiEmailInput} from "@/components/ui/multi-email-input";
import {Label} from "@/components/ui/label";
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from "@/components/animate-ui/radix/tooltip";
import {
    AppEventVariantTransfer,
    LocalResourcePathVariantAbsolutePath,
    ResourceTypeVariantFile,
    ResourceTypeVariantImage,
    ResourceTypeVariantVideo,
    SelectedResourceViewModel,
    TransferEventVariantStartPublicTransfer,
    TransferEventVariantCancelTransfer, TransferTypeVariantSend,
    ResourceTypeVariantFolder,
    ShelfEventVariantAddResources,
    AppEventVariantShelf,
    ShelfEventVariantRemoveResource
} from 'shared_types/types/shared_types'
import {useFileUpload} from "@/hooks/use-file-upload";
import {useEffect, useRef, useState} from "react";
import core from "@/wasm/wasm_core";
import {useIsMobile} from "@/hooks/use-mobile";
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
import {Separator} from '@/components/ui/separator';
import {Card} from "@/components/ui/card.tsx";
import {formatFileSize} from "@/utils/format-file-size";

enum TransferType {
    Public,
    People
}

const activeMethods = [
    {
        name: 'Cloud',
        icon: Globe,
        type: TransferType.Public
    },
    {
        name: 'P2P',
        icon: Users,
        type: TransferType.People
    },
]

export default function SendBoard() {
    const [activeMethod, setActiveMethod] = React.useState(activeMethods[0])

    return (
        <div className="rounded-xl border-1 overflow-hidden max-h-[70vh] sm:max-h-[80vh] min-h-[450px] h-[950px]">
            <SidebarProvider className="h-[100%]">
                <Sidebar collapsible="icon"
                         className="h-full bg-card overflow-hidden border-1 border-muted rounded-xl mb-1">
                    <SidebarHeader className="rounded-tl-xl">
                        <TransferMethodSelector activeMethod={activeMethod} onActiveMethodChange={setActiveMethod}/>
                    </SidebarHeader>
                    <SidebarContentWrapper activeMethod={activeMethod}/>
                    <SidebarRail/>
                </Sidebar>
                <SidebarInset className="flex flex-col h-[100%] min-h-0">
                    <header
                        className="flex h-10 md:h-16 shrink-0 items-center gap-2 transition-[width,height] ease-linear group-has-[[data-collapsible=icon]]/sidebar-wrapper:h-12">
                        <div className="flex items-center gap-2 px-4">
                            <SidebarTrigger className="-ml-1"/>
                            <Separator orientation="vertical" className="mr-2 h-4"/>
                        </div>
                    </header>
                    <div className="flex flex-1 flex-col min-h-0 px-2 pt-0">
                        <FileSelections/>
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
    const defaultShelfId = core.useDefaultShelfId()
    const isResourceRemoveAllowed = core.useShelfRemoveResourceAllow()

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

    const isMobile = useIsMobile();

    return (
        <div className="flex flex-col w-full h-full">
            {/* Resource Selection Area */}
            {isMobile ? (
                // Mobile: Dropdown Button (only show when resources exist)
                selectedResources.length > 0 && (
                    <div className="relative w-full flex-shrink-0 h-[50px]">
                        <input {...getInputProps()} className="sr-only" aria-label="Upload files"/>
                        <input {...getDirectoryInputProps()} className="sr-only" aria-label="Upload folder"/>
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
                        <input {...getInputProps()} className="sr-only" aria-label="Upload files"/>
                        <ImageUpIcon className="size-4 opacity-60 mb-2" aria-hidden="true"/>
                        <p className="text-sm font-medium">Drop files or browse</p>
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
                            className="w-1/3 flex flex-col items-center justify-center border border-dashed rounded-xl transition-colors cursor-pointer hover:bg-muted-foreground/10 data-[dragging=true]:bg-muted-foreground/10 h-full"
                        >
                            <input
                                {...getDirectoryInputProps()}
                                className="sr-only"
                                aria-label="Upload folder"
                            />
                            <FolderIcon className="size-4 opacity-60 mb-2" aria-hidden="true"/>
                            <p className="text-sm font-medium">Drop folders or browse</p>
                        </div>
                    )}
                </div>
            )}

            <div className="h-fit max-h-[95%] overflow-y-auto overflow-x-hidden w-full">
                <div
                    className="sticky top-0 left-0 right-0 h-8 bg-gradient-to-b from-background to-transparent z-10 pointer-events-none"/>

                {/* Resource grid - single column on mobile, grid on desktop */}
                {selectedResources.length === 0 ? (
                    <div
                        className="flex flex-col items-center justify-center min-h-[200px] text-muted-foreground/50 gap-4">
                        <p className="text-lg font-medium">No selected resources</p>
                        {isMobile ? (
                            <>
                                <input {...getInputProps()} className="sr-only" aria-label="Upload files"/>
                                <input {...getDirectoryInputProps()} className="sr-only" aria-label="Upload folder"/>
                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button
                                            size="sm"
                                            className="h-10 w-10 rounded-full bg-bluePrimary text-primaryText hover:bg-bluePrimary/90 p-0"
                                        >
                                            <Plus className="h-5 w-5"/>
                                        </Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent align="center">
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
                            </>
                        ) : null}
                    </div>
                ) : (
                    <div className="flex flex-col md:grid md:grid-cols-3 lg:grid-cols-4 gap-2 md:gap-4 p-2 md:px-1">
                        {selectedResources.map((resource) => (
                            <div className="md:h-[230px] flex items-start flex-row" key={resource.order_id}>
                                <ResourceView model={resource} shelfId={defaultShelfId}
                                              isRemoveAllowed={isResourceRemoveAllowed}/>
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
    model: SelectedResourceViewModel,
    shelfId: string | undefined,
    isRemoveAllowed: boolean
}) {
    const {model, shelfId, isRemoveAllowed} = props;

    const isFile = model.type.constructor == ResourceTypeVariantFile ||
        model.type.constructor == ResourceTypeVariantFolder

    if (isFile) {
        return <FileView model={model} shelfId={shelfId} isRemoveAllowed={isRemoveAllowed}/>
    } else {
        return <MediaView model={model} shelfId={shelfId} isRemoveAllowed={isRemoveAllowed}/>
    }
}

function FileView(props: {
    model: SelectedResourceViewModel,
    shelfId: string | undefined,
    isRemoveAllowed: boolean
}) {
    const {model, shelfId, isRemoveAllowed} = props;
    const isMobile = useIsMobile();

    let thumbnailPath = (model.thumbnail_path as LocalResourcePathVariantAbsolutePath)?.value;
    const isFolder = model.type instanceof ResourceTypeVariantFolder;
    if (!thumbnailPath) {
        thumbnailPath = isFolder ? "/folder.svg" : "/file.svg";
    }

    const displaySize = formatFileSize(model);

    const handleRemove = async () => {
        if (!isRemoveAllowed || !shelfId) return;
        await core.update(new AppEventVariantShelf(new ShelfEventVariantRemoveResource(BigInt(shelfId), BigInt(model.order_id))))
    }

    if (isMobile) {
        return (
            <div
                className="w-full flex items-center gap-3 p-3 rounded-lg border border-border bg-card hover:bg-accent/50 transition-colors group">
                {/* Thumbnail */}
                <div className="w-10 h-10 shrink-0 flex items-center justify-center rounded-md bg-muted">
                    <Image
                        className="w-6 h-6 object-contain opacity-70"
                        width={24}
                        height={24}
                        alt={model.name}
                        src={thumbnailPath}
                    />
                </div>

                {/* File info */}
                <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate text-foreground">
                        {model.name}
                    </p>
                    <p className="text-xs text-muted-foreground mt-0.5">
                        {displaySize}
                    </p>
                </div>

                {/* Actions */}
                {isRemoveAllowed && (
                    <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                            <Button
                                size="sm"
                                variant="ghost"
                                className="h-8 w-8 p-0 shrink-0">
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
                )}
            </div>
        );
    }

    return (
        <div
            className="w-full h-full flex flex-col rounded-lg border border-border bg-card overflow-hidden group hover:border-foreground/20 transition-colors">
            {/* Thumbnail */}
            <div className="flex-1 flex items-center justify-center p-6 bg-muted/30 relative">
                <Image
                    className="w-16 h-16 object-contain opacity-70"
                    width={64}
                    height={64}
                    alt={model.name}
                    src={thumbnailPath}
                />

                {/* Remove button - shows on hover */}
                {isRemoveAllowed && (
                    <Button
                        size="sm"
                        variant="ghost"
                        className="absolute top-2 right-2 h-7 w-7 p-0 opacity-0 group-hover:opacity-100 transition-opacity hover:bg-destructive hover:text-destructive-foreground"
                        onClick={handleRemove}>
                        <X className="h-3.5 w-3.5"/>
                    </Button>
                )}
            </div>

            {/* File info */}
            <div className="p-3 border-t border-border">
                <p className="text-sm font-medium truncate text-foreground mb-1">
                    {model.name}
                </p>
                <p className="text-xs text-muted-foreground">
                    {displaySize}
                </p>
            </div>
        </div>
    );
}

function MediaView(props: {
    model: SelectedResourceViewModel,
    shelfId: string | undefined,
    isRemoveAllowed: boolean
}) {
    const {model, shelfId, isRemoveAllowed} = props;

    const isMobile = useIsMobile()
    const isVideo = model.type.constructor == ResourceTypeVariantVideo
    const isImage = model.type.constructor == ResourceTypeVariantImage

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

    const displaySize = formatFileSize(model);

    const handleRemove = () => {
        if (!isRemoveAllowed || !shelfId) return;
        core.update(new AppEventVariantShelf(new ShelfEventVariantRemoveResource(BigInt(shelfId), BigInt(model.order_id))))
    }

    if (isMobile) {
        return (
            <div
                className="w-full flex items-center gap-3 p-3 rounded-lg border border-border bg-card hover:bg-accent/50 transition-colors group">
                {/* Thumbnail */}
                <div className="w-10 h-10 shrink-0 rounded-md overflow-hidden bg-muted relative">
                    {thumbnailUrl ? (
                        <Image className="w-full h-full object-cover" fill src={thumbnailUrl} alt={model.name}/>
                    ) : (
                        <div className="w-full h-full flex items-center justify-center">
                            <ImageUpIcon className="w-5 h-5 opacity-40"/>
                        </div>
                    )}
                    {isVideo && (
                        <div className="absolute inset-0 flex items-center justify-center bg-black/30">
                            <Play className="w-3 h-3 text-white fill-white"/>
                        </div>
                    )}
                </div>

                {/* File info */}
                <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate text-foreground">
                        {model.name}
                    </p>
                    <p className="text-xs text-muted-foreground mt-0.5">
                        {displaySize}
                    </p>
                </div>

                {/* Actions */}
                {isRemoveAllowed && (
                    <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                            <Button
                                size="sm"
                                variant="ghost"
                                className="h-8 w-8 p-0 shrink-0">
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
                )}
            </div>
        );
    }

    return (
        <div
            className="w-full h-full flex flex-col rounded-lg border border-border bg-card overflow-hidden group hover:border-foreground/20 transition-colors">
            {/* Thumbnail */}
            <div className="flex-1 relative bg-muted/30">
                {thumbnailUrl ? (
                    <Image className="w-full h-full object-cover" fill src={thumbnailUrl} alt={model.name}/>
                ) : (
                    <div className="w-full h-full flex items-center justify-center">
                        <ImageUpIcon className="w-12 h-12 opacity-20"/>
                    </div>
                )}

                {/* Video play icon */}
                {isVideo && (
                    <div className="absolute top-2 left-2 bg-black/60 rounded-full p-1.5">
                        <Play className="w-3 h-3 text-white fill-white"/>
                    </div>
                )}

                {/* Remove button - shows on hover */}
                {isRemoveAllowed && (
                    <Button
                        size="sm"
                        variant="ghost"
                        className="absolute top-2 right-2 h-7 w-7 p-0 opacity-0 group-hover:opacity-100 transition-opacity bg-black/60 hover:bg-destructive rounded-md text-white"
                        onClick={handleRemove}>
                        <X className="h-3.5 w-3.5"/>
                    </Button>
                )}
            </div>

            {/* File info */}
            <div className="p-3 border-t border-border">
                <p className="text-sm font-medium truncate text-foreground mb-1">
                    {model.name}
                </p>
                <p className="text-xs text-muted-foreground">
                    {displaySize}
                </p>
            </div>
        </div>
    );
}

function TransferMethodSelector({activeMethod, onActiveMethodChange}: {
    activeMethod: typeof activeMethods[0],
    onActiveMethodChange: (method: typeof activeMethods[0]) => void
}) {
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
                            <div
                                className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
                                <activeMethod.icon className="size-4"/>
                            </div>
                            <div className="grid flex-1 text-left text-sm leading-tight">
                                <span className="truncate font-semibold">
                                    {activeMethod.name}
                                </span>
                            </div>
                            <ChevronsUpDown className="ml-auto"/>
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
                                <method.icon className="size-4"/>
                                {method.name}
                            </DropdownMenuCheckboxItem>
                        ))}
                    </DropdownMenuContent>
                </DropdownMenu>
            </SidebarMenuItem>
        </SidebarMenu>
    );
}

function SidebarContentWrapper({activeMethod}: { activeMethod: typeof activeMethods[0] }) {
    const {state} = useSidebar();

    if (state === 'collapsed') {
        return null;
    }

    return (
        <SidebarContent className="rounded-bl-xl px-1">
            <TransferForm activeMethod={activeMethod}/>
        </SidebarContent>
    );
}

function TransferForm({activeMethod}: { activeMethod: typeof activeMethods[0] }) {
    const content = activeMethod.type === TransferType.Public
        ? <PublicSend/>
        : <P2PSend/>

    return (
        <div className={"px-2 flex flex-col items-center justify-center pt-5 h-fit h-full items-start justify-start"}>
            {content}
        </div>
    )
}

function PublicSend() {
    const [password, setPassword] = useState('')
    const [emails, setEmails] = React.useState<string[]>([])
    const defaultShelfId = core.useDefaultShelfId()
    const cloudSession = core.useCloudSession(defaultShelfId)
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

    return <div className={"flex flex-col w-full h-full items-center gap-10 justify-between mt-1"}>
        <div className={"flex flex-col w-full gap-3 justify-between h-fit"}>
            <p className="text-start w-full text-primaryText/70 text-sm">
                Create a shareable link or send files by email. Stored for 7 days, with optional password protection.
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
                            core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(BigInt(cloudSession.session_id), new TransferTypeVariantSend(BigInt(defaultShelfId || 0)))))
                        }
                    }}>Cancel</Button>
                }
                {
                    !cloudSession && defaultShelfId &&
                    <Button className="w-fit h-[35px] bg-bluePrimary text-primary" onClick={() => {
                        core.update(new AppEventVariantTransfer(new TransferEventVariantStartPublicTransfer(BigInt(defaultShelfId || 0), password || null, emails)))
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
                            new TransferTypeVariantSend(BigInt(defaultShelfId || 0))
                        )))
                    }}>Continue</Button>
                }
            </div>
        </div>
        <div className="w-full h-fit mb-4">
            <div className="bg-card flex flex-col gap-2 px-2 items-center text-center">
                <p className="text-sm text-primaryText/80">
                    2× faster. No upload. Instant URL generation.
                </p>
                <Link
                    href="#desktop"
                    className="text-blue-300 text-sm font-medium rounded-lg hover:opacity-90 transition-opacity"
                >
                    Download Desktop App
                </Link>
            </div>
        </div>
    </div>
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
        <TooltipProvider>
            <div className="relative animate-in fade-in slide-in-from-bottom-2 duration-500">
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Input
                            ref={inputRef}
                            value={url}
                            disabled={true}
                            className="pr-12 cursor-default bg-slate-900 text-white border-3 border-indigo-400 rounded-lg shadow-[0_0_10px_rgba(59,130,246,0.5)]"
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
                    className="absolute right-2 top-1/2 transform -translate-y-1/2 p-1.5 rounded-md hover:bg-white/10 transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
                    title={isCopied ? "Copied!" : "Copy to clipboard"}
                >
                    {isCopied ? (
                        <Check className="h-4 w-4 text-green-500"/>
                    ) : (
                        <Copy className="h-4 w-4 text-white/80 hover:text-white"/>
                    )}
                </button>
            </div>
        </TooltipProvider>
    )
}

function P2PSend() {
    return (
        <div className="flex flex-col w-full h-full gap-6 justify-center items-center">
            <div className="flex flex-col gap-4 text-center">
                <div className="flex justify-center">
                    <div className="p-4 rounded-full bg-bluePrimary/10">
                        <Users className="w-8 h-8 text-bluePrimary"/>
                    </div>
                </div>
                <h3 className="text-lg font-semibold text-primaryText">
                    P2P Transfer
                </h3>
                <p className="text-sm text-primaryText/70 max-w-[280px]">
                    Transfer files directly without uploading to any cloud. Fast, private, and secure.
                </p>
            </div>
            <div className="w-full rounded-xl shadow-lg">
                <Card className="p-3 bg-card rounded-[10px] flex flex-col gap-4 items-center text-center">
                    <p className="text-sm text-primaryText/80">
                        P2P transfer is currently available on Desktop only.
                    </p>
                    <Link
                        href="#desktop"
                        className="py-2 w-full items-center bg-bluePrimary text-sm font-medium text-white rounded-lg hover:opacity-90 transition-opacity text-center"
                    >
                        Download Desktop App
                    </Link>
                </Card>
            </div>
        </div>
    )
}