'use client'

import {
    DropdownMenuTrigger,
    DropdownMenu, DropdownMenuContent, DropdownMenuCheckboxItem
} from "@/components/animate-ui/radix/dropdown-menu";
import {
    AlertCircleIcon, Delete, Download, File,
    Globe, ImageUpIcon, Play, Trash,
    Users, X, XIcon
} from 'lucide-react'
import {Button} from "@/components/ui/button";
import {ChevronsUpDown} from "lucide-react";
import * as React from "react";
import {Input} from "@/components/ui/input";
import {Label} from "@/components/ui/label";
import {MotionEffect} from "@/components/animate-ui/effects/motion-effect";
import {
    AppEventVariantTransfer, FileReceiveResourceViewModel,
    ImageReceiveResourceViewModel, LocalResourcePathVariantAbsolutePath,
    PeerViewModel,
    ResourceTypeVariantFile, ResourceTypeVariantFolder,
    ResourceTypeVariantImage,
    ResourceTypeVariantVideo,
    SelectedResourceViewModel,
    TransferEventVariantAddResources, TransferEventVariantRemoveResource,
    VideoReceiveResourceViewModel,
    TransferEventVariantStartPublicTransfer,
} from 'shared_types/types/shared_types'
import CircleProgress from "@/components/ui/progress";
import {Avatar, AvatarImage} from "@/components/ui/avatar";
import {useFileUpload} from "@/hooks/use-file-upload";
import {useEffect, useState} from "react";
import core from "@/wasm/wasm_core";
import {useIsMobile} from "@/hooks/use-mobile";
import clsx from "clsx";
import Image from "next/image";

export default function SendBoard() {
    return <>
        <div
            className="h-[950px] max-h-[85vh] w-full rounded-xl bg-blackBase flex flex-col border-primaryText/20 items-center justify-center border-1">
            <div className={"grid grid-cols-12 w-full h-full gap-2"}>
                <div className={"col-span-3 h-full"}>
                    <Board/>
                </div>
                <div className={"col-span-9 h-full"}>
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
            removeFile,
            getInputProps,
        },
    ] = useFileUpload({
        accept: "*",
    })

    const transfer_state = core.useTransferState()
    const selectedResources = transfer_state?.selected_resources || []
    console.log('selectedResources', selectedResources)

    useEffect(() => {
        if (files.length) {
            core.addFiles(files.map(file => file.file))
                .then((selections) => {
                    core.update(new AppEventVariantTransfer(new TransferEventVariantAddResources(
                        selections
                    )))
                })
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
                            Drop your image here or click to browse
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
                    selectedResources.map((resource, index) => (
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

    let isMobile = useIsMobile()
    let isVideo = model.type.constructor == ResourceTypeVariantVideo
    let isImage = model.type.constructor == ResourceTypeVariantImage
    let defaultThumbnail = <Image
        className="w-full h-auto text-primaryText p-10"
        layout="fill"
        alt={`${model.name}`}
        src={'/file.svg'}
    />

    let [thumbnail, setThumbnail] = useState(defaultThumbnail)

    useEffect(() => {
        if (isVideo || isImage) {
            core.nativeProcessor?.load_thumbnail_bytes(model.order_id).then((it) => {
                if (it) {
                    const blob = new Blob([it], {type: 'image/png'});
                    setThumbnail(<Image className={"w-full h-full"} src={URL.createObjectURL(blob)} alt={model.name}
                                        layout={"fill"}/>)
                }
            })
        }
    }, []);

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <div className="w-full h-full overflow-hidden rounded-2xl relative group">
            <div
                className={clsx(
                    "z-3 w-full h-[90%] absolute items-center flex justify-center bg-gradient-to-t from-blackBase/70 bottom-0",
                    isMobile
                        ? "opacity-100"
                        : "opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                )}
            >
                {
                    <Button className={"z-20 rounded-xl bg-white"} onClick={() => {
                        core.update(new AppEventVariantTransfer(new TransferEventVariantRemoveResource(model.order_id)))
                    }}>
                        <X/>
                    </Button>
                }
            </div>

            {
                isVideo
                    ? <div>
                        <Play className={"w-5 h-5 bg-muted p-1 z-20 rounded-sm absolute top-2 right-2"}/>
                    </div>
                    : <></>
            }

            <div
                className={clsx(
                    "flex w-full flex-row z-4 bottom-0 absolute items-center px-3 justify-between",
                    isMobile
                        ? "opacity-100"
                        : "opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                )}
            >
                <div className="flex flex-col items-start gap-1">
                    <p className="text-primaryText text-sm">
                        {model.name}
                    </p>
                    <p className="text-sm text-primaryText/80">
                        {displaySize}
                    </p>
                </div>
            </div>

            {
                thumbnail
            }
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
            <div className={"px-2 flex flex-col items-center justify-center pt-8"}>
                {
                    content
                }
            </div>
        </div>
    </>
}

function PublicSend() {
    const [password, setPassword] = useState('')
    return <div className={"flex flex-col w-full h-full items-center gap-8 justify-center px-2"}>
        <MotionEffect
            className={"flex flex-col w-full gap-3"}
            key={0}
            slide={{
                direction: 'down',
            }}
            fade
            zoom
            inView
            delay={0.2}>
            <p className="text-start w-full text-primaryText/70 text-sm pb-1 text-center">
                Create a sharable URL. Protect access by setting a password to keep your content safe.
            </p>
        </MotionEffect>

        <MotionEffect
            className={"flex flex-col w-full gap-3"}
            key={1}
            slide={{
                direction: 'down',
            }}
            fade
            zoom
            inView
            delay={0.2 + 0.1}>
            <Label htmlFor={"password"}>Password (optional)</Label>
            <Input id={"password"} value={password} onChange={(it) => setPassword(it.target.value)} type={"password"} maxLength={20} placeholder={"pwd@123"}/>
            <Button className="w-fit h-[35px] bg-bluePrimary text-primary" onClick={() => {
                core.update(new AppEventVariantTransfer(new TransferEventVariantStartPublicTransfer(password)))
            }}>Upload</Button>
        </MotionEffect>
    </div>
}

function NearbySend() {
    const nearbyState = window.core.useNearbyState()
    const nearbyPeers = nearbyState?.peers || []

    return <>
        <MotionEffect
            className="flex flex-col w-full gap-3"
            key={0}
            slide={{direction: 'down'}}
            fade
            zoom
            inView
            delay={0.2}>

            <p className="text-start w-full text-primaryText/70 text-sm pb-1">
                Send files directly to a friend’s email
            </p>

            <div className="flex flex-col w-full gap-3">
                <Input id="email" type="email" maxLength={20} placeholder="someone@company"/>
                <Button className="w-fit h-[35px] bg-bluePrimary text-primary">Send</Button>
            </div>

            <div className="flex flex-col w-full gap-3 mt-5">
                <p className="text-start w-full text-primaryText/70 text-sm pb-1">
                    Or share with nearby friends and devices
                </p>
                {nearbyPeers.map(peer => (
                    <NearbyPeer key={peer.id} peer={peer}/>
                ))}
            </div>
        </MotionEffect>
    </>
}

function NearbyPeer({
                        peer
                    }: {
    peer: PeerViewModel
}) {
    const color = `rgb(${peer.avatar.dominant_color_r}, ${peer.avatar.dominant_color_g}, ${peer.avatar.dominant_color_b})`
    return <>
        <Button
            className={"flex flex-row bg-muted hover:bg-muted-foreground/30 rounded-2xl items-center px-2 py-2 h-fit w-full border-1 border-primaryText/5 justify-between"}>
            <div className={"flex flex-row items-center gap-3"}>
                <div
                    className={"bg-bluePrimary rounded-xl aspect-square justify-center items-center text-primaryText flex h-[34px] w-[34px]"}>
                    <Avatar className={"p-1 rounded-xl"} style={{backgroundColor: color}}>
                        <AvatarImage src={peer.avatar.url}/>
                    </Avatar>
                </div>
                <div className={"flex flex-col gap-0"}>
                    <p className={"text-primaryText font-bold text-sm"}>{peer.display_name}</p>
                </div>
            </div>
            {
                peer.transfer_progress ? <CircleProgress progress={peer.transfer_progress} size={30}/> : <></>
            }
        </Button>
    </>
}
