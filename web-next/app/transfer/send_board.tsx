'use client'

import {
    DropdownMenuTrigger,
    DropdownMenu
} from "@/components/animate-ui/radix/dropdown-menu";
import {
    Globe,
    Users
} from 'lucide-react'
import {Button} from "@/components/ui/button";
import {ChevronsUpDown} from "lucide-react";
import * as React from "react";

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
    const [activeMethod] = React.useState(activeMethods[0])

    return <>
        <div className={"flex flex-col border-1 w-full h-full bg-sidebar rounded-xl p-3"}>
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button
                        variant="ghost"
                        className="h-12">
                        <div
                            className="flex aspect-square size-8 items-center justify-center rounded-lg bg-bluePrimary text-primaryText">
                            <activeMethod.icon className="size-4"/>
                        </div>
                        <div className="grid flex-1 text-left text-sm leading-tight pl-2">
                                <span className="truncate font-poppins font-semibold">
                                    {activeMethod.name}
                                </span>
                        </div>
                        <ChevronsUpDown className="ml-auto"/>
                    </Button>
                </DropdownMenuTrigger>
            </DropdownMenu>
        </div>
    </>
}