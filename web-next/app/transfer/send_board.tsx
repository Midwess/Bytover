'use client'

import {
    DropdownMenuTrigger,
    DropdownMenu, DropdownMenuContent, DropdownMenuCheckboxItem
} from "@/components/animate-ui/radix/dropdown-menu";
import {
    Globe,
    Users
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

export default function SendBoard() {
    return <>
        <div
            className="h-[950px] max-h-[85vh] w-full rounded-xl bg-blackBase flex flex-col border-primaryText/20 items-center justify-center border-1">
            <div className={"grid grid-cols-12 w-full h-full gap-2"}>
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
    return <div className={"flex flex-col w-full h-full bg-blackBase rounded-2xl"}>
    </div>
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
            <div className={"px-2 flex flex-col items-center justify-center pt-8 px-1"}>
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
            <Button className={"w-fit h-[35px]"}>Upload</Button>
        </MotionEffect>
    </div>
}

function NearbySend() {
    const nearbyState = window.core.useNearbyState()
    const nearbyPeers = nearbyState?.peers || []

    return <>
        <div className={"w-full h-full flex-col p-2"}>
            {nearbyPeers.map(peer => (<NearbyPeer key={peer.id} peer={peer}/>))}
        </div>
    </>
}

function NearbyPeer({peer}: { peer: PeerViewModel }) {
    const color = `rgb(${peer.avatar.dominant_color_r}, ${peer.avatar.dominant_color_g}, ${peer.avatar.dominant_color_b})`
    console.log(color)
    return <>
        <Button
            className={"h-fit w-full hover:bg-muted-foreground/30 flex flex-row bg-muted rounded-3xl items-center px-2 py-2 max-h-[60px] border-1 border-muted shadow-xl justify-between"}>
            <div className={"flex flex-row items-center gap-3"}>
                <div
                    className={"rounded-full aspect-square justify-center items-center flex h-auto w-auto"}>
                    <Avatar className="border-3 p-0.5" style={{backgroundColor: color}}>
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

