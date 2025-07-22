'use client'

import {
    DropdownMenuTrigger,
    DropdownMenu, DropdownMenuContent, DropdownMenuCheckboxItem
} from "@/components/animate-ui/radix/dropdown-menu";
import {
    AlertCircleIcon,
    Globe, ImageUpIcon,
    Users, XIcon
} from 'lucide-react'
import {Button} from "@/components/ui/button";
import {ChevronsUpDown} from "lucide-react";
import * as React from "react";
import {Input} from "@/components/ui/input";
import {Label} from "@/components/ui/label";
import {MotionEffect} from "@/components/animate-ui/effects/motion-effect";
import {PeerViewModel} from "../../../shared_types/generated/typescript/types/shared_types";
import CircleProgress from "@/components/ui/progress";
import {Avatar, AvatarImage} from "@/components/ui/avatar";
import {useFileUpload} from "@/hooks/use-file-upload";

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
        { files, isDragging, errors },
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

    const previewUrl = files[0]?.preview || null

    return (
        <div className={"flex flex-col w-full h-full rounded-2xl items-center p-5"}>
            <div className="relative w-full flex flex-col">
                <div
                    role="button"
                    onClick={openFileDialog}
                    onDragEnter={handleDragEnter}
                    onDragLeave={handleDragLeave}
                    onDragOver={handleDragOver}
                    onDrop={handleDrop}
                    data-dragging={isDragging || undefined}
                    className="bg-muted border-input w-full hover:bg-muted-foreground/40 data-[dragging=true]:bg-muted-foreground/40 has-[input:focus]:border-ring has-[input:focus]:ring-ring/50 relative flex min-h-52 flex-col items-center justify-center overflow-hidden rounded-xl border border-dashed p-4 transition-colors has-disabled:pointer-events-none has-disabled:opacity-50 has-[img]:border-none has-[input:focus]:ring-[3px]"
                >
                    <input
                        {...getInputProps()}
                        className="sr-only"
                        aria-label="Upload file"
                    />
                    {previewUrl ? (
                        <div className="absolute inset-0">
                            <img
                                src={previewUrl}
                                alt={files[0]?.file?.name || "Uploaded image"}
                                className="size-full object-cover"
                            />
                        </div>
                    ) : (
                        <div className="flex flex-col items-center justify-center px-4 py-3 text-center">
                            <div
                                className="bg-background mb-2 flex size-11 shrink-0 items-center justify-center rounded-full border"
                                aria-hidden="true"
                            >
                                <ImageUpIcon className="size-4 opacity-60" />
                            </div>
                            <p className="mb-1.5 text-sm font-medium">
                                Drop your image here or click to browse
                            </p>
                        </div>
                    )}
                </div>
                {previewUrl && (
                    <div className="absolute top-4 right-4">
                        <button
                            type="button"
                            className="focus-visible:border-ring focus-visible:ring-ring/50 z-50 flex size-8 cursor-pointer items-center justify-center rounded-full bg-black/60 text-white transition-[color,box-shadow] outline-none hover:bg-black/80 focus-visible:ring-[3px]"
                            onClick={() => removeFile(files[0]?.id)}
                            aria-label="Remove image"
                        >
                            <XIcon className="size-4" aria-hidden="true" />
                        </button>
                    </div>
                )}
            </div>

            {errors.length > 0 && (
                <div
                    className="text-destructive flex items-center gap-1 text-xs"
                    role="alert"
                >
                    <AlertCircleIcon className="size-3 shrink-0" />
                    <span>{errors[0]}</span>
                </div>
            )}
        </div>
    )
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
            delay={0.2 + 0 * 0.1}>
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
            delay={0.2 + 1 * 0.1}>
            <Label htmlFor={"password"}>Password (optional)</Label>
            <Input id={"password"} type={"password"} maxLength={20} placeholder={"pwd@123"}/>
            <Button className="w-fit h-[35px] bg-bluePrimary text-primary">Upload</Button>
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
            slide={{ direction: 'down' }}
            fade
            zoom
            inView
            delay={0.2}>

            <p className="text-start w-full text-primaryText/70 text-sm pb-1">
                Send files directly to a friend’s email
            </p>

            <div className="flex flex-col w-full gap-3">
                <Input id="email" type="email" maxLength={20} placeholder="someone@company" />
                <Button className="w-fit h-[35px] bg-bluePrimary text-primary">Send</Button>
            </div>

            <div className="flex flex-col w-full gap-3 mt-5">
                <p className="text-start w-full text-primaryText/70 text-sm pb-1">
                    Or share with nearby friends and devices
                </p>
                {nearbyPeers.map(peer => (
                    <NearbyPeer key={peer.id} peer={peer} />
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
