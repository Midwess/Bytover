'use client'

import {
    DropdownMenuTrigger,
    DropdownMenu, DropdownMenuContent, DropdownMenuCheckboxItem
} from "@/components/animate-ui/radix/dropdown-menu";
import {
    AlertCircleIcon,
    Globe, ImageUpIcon, Play,
    Users, X, Copy, Check,
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
    TransferEventVariantAddResources, TransferEventVariantRemoveResource,
    TransferEventVariantStartPublicTransfer,
    TransferEventVariantCancelTransfer, TransferTypeVariantSend,
    TransferEventVariantStartTransfer
} from 'shared_types/types/shared_types'
import CircleProgress from "@/components/ui/progress";
import {Avatar, AvatarImage} from "@/components/ui/avatar";
import {useFileUpload} from "@/hooks/use-file-upload";
import {useEffect, useReducer, useRef, useState} from "react";
import core from "@/wasm/wasm_core";
import {useIsMobile} from "@/hooks/use-mobile";
import clsx from "clsx";
import Image from "next/image";
import {Progress, ProgressTrack} from "@/components/animate-ui/base/progress";

export default function SendBoard() {
    return <>
        <div
            className="h-[950px] max-h-[85vh] w-full rounded-xl bg-blackBase flex flex-col border-primaryText/20 items-center justify-center border-1 overflow-scroll">
            <div className={"grid grid-cols-11 w-full h-full gap-2"}>
                <div className={"col-span-3 h-full"}>
                    <Board/>
                </div>
                <div className={"col-span-8 h-full"}>
                    <FileSelections/>
                </div>
            </div>
        </div>
    </>
}

function FileSelections() {
    const [
        {files, isDragging, errors},
        {
            handleDragEnter,
            handleDragLeave,
            handleDragOver,
            handleDrop,
            openFileDialog,
            getInputProps,
            clearFiles
        },
    ] = useFileUpload({
        accept: "*",
        multiple: true,
    })

    const transfer_state = core.useTransferState()
    const selectedResources = transfer_state?.selected_resources || []

    useEffect(() => {
        if (files.length) {
            core.addFiles(files.map(file => file.file))
                .then((selections) => {
                    core.update(new AppEventVariantTransfer(new TransferEventVariantAddResources(
                        selections
                    )))
                })
            clearFiles()
        }
    }, [files]);

    return (
        <div className={"flex flex-col w-full h-full rounded-2xl items-center p-5 gap-8"}>
            <div className="relative w-full flex flex-col">
                <div
                    role="button"
                    onClick={openFileDialog}
                    onDragEnter={handleDragEnter}
                    onDragLeave={handleDragLeave}
                    onDragOver={handleDragOver}
                    onDrop={handleDrop}
                    data-dragging={isDragging || undefined}
                    className="border-input w-full hover:bg-muted-foreground/10 data-[dragging=true]:bg-muted-foreground/10 has-[input:focus]:border-ring has-[input:focus]:ring-ring/50 relative flex min-h-52 flex-col items-center justify-center overflow-hidden rounded-xl border border-dashed p-4 transition-colors has-disabled:pointer-events-none has-disabled:opacity-50 has-[img]:border-none has-[input:focus]:ring-[3px]"
                >
                    <input
                        {...getInputProps()}
                        className="sr-only"
                        aria-label="Upload file"
                    />
                    <div className="flex flex-col items-center justify-center px-4 py-3 text-center">
                        <div
                            className="bg-background mb-2 flex size-11 shrink-0 items-center justify-center rounded-full border"
                            aria-hidden="true"
                        >
                            <ImageUpIcon className="size-4 opacity-60"/>
                        </div>
                        <p className="mb-1.5 text-sm font-medium">
                            Drop multiple files here or click to browse
                        </p>
                    </div>
                </div>
            </div>

            {errors.length > 0 && (
                <div
                    className="text-destructive flex items-center gap-1 text-xs"
                    role="alert"
                >
                    <AlertCircleIcon className="size-3 shrink-0"/>
                    <span>{errors[0]}</span>
                </div>
            )}
            <div
                className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-5 gap-x-4 gap-y-4 pb-8 w-full">
                {
                    selectedResources.map((resource) => (
                        <div className={"h-[220px] flex items-start flex-row"} key={resource.order_id}>
                            <ResourceView model={resource}/>
                        </div>
                    ))
                }
            </div>
        </div>
    )
}

function ResourceView(props: {
    model: SelectedResourceViewModel
}) {
    const {model} = props;

    const isFile = model.type.constructor == ResourceTypeVariantFile

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
    if (!thumbnailPath) {
        thumbnailPath = "/file.svg";
    }

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <div
            className="px-2 w-full h-full overflow-hidden gap-3 justify-center rounded-2xl flex flex-col relative group bg-muted p-4 border-1 border-primaryText/5">
            <div
                className={clsx(
                    "absolute z-20 inset-0 flex items-center justify-center",
                    isMobile ? "opacity-100" : "opacity-0 group-hover:opacity-100 w-full h-full bg-blackBase/40 transition-opacity duration-300"
                )}>
                <Button className={"rounded-xl"} onClick={() => {
                    core.update(new AppEventVariantTransfer(new TransferEventVariantRemoveResource(model.order_id)))
                }}>
                    <X/>
                </Button>
            </div>

            <div className="relative aspect-square w-auto h-[40%]">
                <Image
                    className="w-full h-auto text-primaryText"
                    layout="fill"
                    alt={`${model.type}`}
                    src={thumbnailPath}
                />
            </div>

            {/* Metadata */}
            <div className="flex h-fit flex-col text-white items-center mt-3">
                <p className="text-sm text-center font-poppins break-words w-full line-clamp-3-ellipsis">{model.name}</p>
                <p className="text-sm text-center text-white/80 font-poppins">{displaySize}</p>
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

    const [thumbnail, setThumbnail] = useState(defaultThumbnail)

    useEffect(() => {
        if ((isVideo || isImage) && model.thumbnail_path) {
            core.loadThumbnailSource(model.thumbnail_path).then((it) => {
                if (it) {
                    setThumbnail(<Image className={"w-full h-full object-cover"} fill src={it} alt={model.name}/>)
                }
            })
        }
    }, []);

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <div className="w-full h-full bg-muted-foreground/20 border border-muted/10 overflow-hidden rounded-2xl relative group">
            {/* Thumbnail - lowest z-index */}
            <div className="absolute inset-0 z-0">
                {thumbnail}
            </div>

            {/* Video play icon */}
            {isVideo && (
                <div className="absolute top-2 right-2 z-20">
                    <Play className="w-5 h-5 bg-muted p-1 rounded-sm"/>
                </div>
            )}

            {/* Background overlay for hover effect */}
            <div
                className={clsx(
                    "absolute inset-0 bg-gradient-to-t from-blackBase/70 via-transparent to-transparent z-10",
                    isMobile
                        ? "opacity-100"
                        : "opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                )}
            />

            {/* Remove button - centered with high z-index */}
            <div
                className={clsx(
                    "absolute inset-0 flex items-center justify-center z-30",
                    isMobile
                        ? "opacity-100"
                        : "opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                )}
            >
                <Button 
                    className="rounded-xl bg-white/90 hover:bg-white text-black shadow-md" 
                    onClick={() => {
                        core.update(new AppEventVariantTransfer(new TransferEventVariantRemoveResource(model.order_id)))
                    }}
                >
                    <X className="w-4 h-4"/>
                </Button>
            </div>

            {/* File info - positioned at bottom */}
            <div
                className={clsx(
                    "absolute bottom-0 left-0 right-0 p-2 z-20",
                    isMobile
                        ? "opacity-100"
                        : "opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                )}
            >
                <div className="flex flex-col items-start gap-1">
                    <p className="text-primaryText text-sm font-medium">
                        {model.name}
                    </p>
                    <p className="text-sm text-primaryText/80">
                        {displaySize}
                    </p>
                </div>
            </div>
        </div>
    );
}

enum TransferType {
    Public,
    People
}

const activeMethods = [
    {
        name: 'Public',
        icon: Globe,
        type: TransferType.Public
    },
    {
        name: 'People',
        icon: Users,
        type: TransferType.People
    }
]

function Board() {
    const [activeMethod, setActiveMethod] = React.useState(activeMethods[0])

    const content = activeMethod.type === TransferType.Public
        ? <PublicSend/>
        : <NearbySend/>

    return <>
        <div className={"flex flex-col border-1 w-full h-full bg-sidebar rounded-xl p-2"}>
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button
                        variant="ghost"
                        className="h-12">
                        <div
                            className="flex aspect-square size-8 items-center justify-center rounded-lg bg-bluePrimary text-primaryText">
                            <activeMethod.icon className="size-4"/>
                        </div>
                        <div className="grid flex-1 text-left text-sm leading-tight">
                                <span className="truncate font-semibold">
                                    {activeMethod.name}
                                </span>
                        </div>
                        <ChevronsUpDown className="ml-auto"/>
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent className={"font-medium w-[200px]"}>
                    <DropdownMenuCheckboxItem className={"w-[200px] flex flex-row h2"}
                                              checked={(activeMethod === activeMethods[0])} onCheckedChange={() => {
                        setActiveMethod(activeMethods[0])
                    }}>
                        <Globe/>
                        Public
                    </DropdownMenuCheckboxItem>
                    <DropdownMenuCheckboxItem className={"w-[200px] h2"} checked={(activeMethod === activeMethods[1])}
                                              onCheckedChange={() => {
                                                  setActiveMethod(activeMethods[1])
                                              }}>
                        <Users/>
                        People
                    </DropdownMenuCheckboxItem>
                </DropdownMenuContent>
            </DropdownMenu>
            <div className={"px-2 flex flex-col items-center justify-center pt-5"}>
                {
                    content
                }
            </div>
        </div>
    </>
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
        }
        else {
            setIsInProgress(false)
            setTimeout(() => {
                if (!cloudRef?.current?.is_in_progress) {
                    setIsInProgressDefer(false)
                }
            }, 2000)
        }

    }, [cloudSession?.is_in_progress])

    return <div className={"flex flex-col w-full h-full items-center gap-10 justify-center px-2 mt-4"}>
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
                <Input id={"password"} disabled={isInProgress} value={password} onChange={(it) => setPassword(it.target.value)}
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
                    isInProgress && <Button className="mt-2 w-fit h-[35px] bg-muted-foreground text-primary" onClick={() => {
                        if (cloudSession?.is_in_progress) {
                            core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(cloudSession.session_id, new TransferTypeVariantSend())))
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
                            cloudSession?.session_id,
                            new TransferTypeVariantSend()
                        )))
                    }}>Continue</Button>
                }
            </div>
        </MotionEffect>
    </div>
}

function UrlInputWithCopy({url}: {url: string}) {
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
        }
        else {
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
                    isInProgress && <Button className="mt-2 w-fit h-[35px] bg-muted-foreground text-primary" onClick={() => {
                        if (cloudSession?.is_in_progress) {
                            core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(cloudSession.session_id, new TransferTypeVariantSend())))
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
                        Send to {emails.length > 0 ? `${emails.length} recipient${emails.length > 1 ? 's' : ''}` : 'Email'}
                    </Button>
                }
                {
                    cloudSession?.is_completed &&
                    <Button className="w-fit h-[35px] bg-greenSecondary/40 text-primary" onClick={() => {
                        core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(
                            cloudSession?.session_id,
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

function NearbyPeer(props: {peer: PeerViewModel}) {
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