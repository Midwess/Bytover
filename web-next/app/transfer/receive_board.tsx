'use client'
import * as React from "react";
import {
    FileReceiveResourceViewModel,
    ImageReceiveResourceViewModel, LocalResourcePathVariantAbsolutePath, ResourceTypeVariantFolder,
    SelectedResourceViewModel,
    VideoReceiveResourceViewModel
} from 'shared_types/types/shared_types'
import {
    ChevronsUpDown, Download,
    Globe, Play
} from 'lucide-react'
import {receiveList} from "@/app/mock_data";
import {PinList, PinListItem} from "@/components/animate-ui/components/pin-list";
import {Button} from '@/components/ui/button'
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleTrigger,
} from '@/components/animate-ui/radix/collapsible';
import {ReactElement, useState} from "react";
import {MotionEffect} from '@/components/animate-ui/effects/motion-effect';
import Image from "next/image";
import {useIsMobile} from "@/hooks/use-mobile";
import clsx from "clsx";
import {Input} from "@/components/ui/input";

export default function ReceiveBoard() {
    const [selectedSession] = useState(receiveList[0])

    return <>
        <div
            className="h-[950px] max-h-[85vh] w-full rounded-xl bg-blackBase flex flex-col border-primaryText/20 items-center justify-center border-1">
            <div className={"grid grid-cols-12 w-full h-full gap-2"}>
                <div className={"col-span-3 h-full"}>
                    <Board/>
                </div>
                <div className={"col-span-9 h-full p-4 flex flex-col overflow-y-scroll pb-20"}>
                    <Collapsible
                        className={`w-full ${selectedSession.image_resources.length ? 'visible' : 'hidden'}`}>
                        <ReceiveCategory
                            title={`${selectedSession.image_resources.length} Image${selectedSession.image_resources.length !== 1 ? 's' : ''}`}/>
                        <CollapsibleContent className={"space-y-2"}>
                            <div className="grid grid-cols-1 sm:grid-cols-3 gap-x-4 gap-y-4 pb-8">
                                {
                                    selectedSession.image_resources.map((image: ImageReceiveResourceViewModel, index: number) => {
                                        return <ItemEffect key={index} index={index}>
                                            <div className={"h-[200px]"}>
                                                <MediaView key={index} media={image}/>
                                            </div>
                                        </ItemEffect>
                                    })
                                }
                            </div>
                        </CollapsibleContent>
                    </Collapsible>
                    <Collapsible
                        className={`w-full ${selectedSession.video_resources.length ? 'visible' : 'hidden'}`}>
                        <ReceiveCategory
                            title={`${selectedSession.video_resources.length} Video${selectedSession.video_resources.length !== 1 ? 's' : ''}`}/>
                        <CollapsibleContent className={"space-y-2"}>
                            <div className="grid grid-cols-1 sm:grid-cols-3 gap-x-4 gap-y-4 pb-8">
                                {
                                    selectedSession.video_resources.map((video: VideoReceiveResourceViewModel, index: number) => {
                                        return <ItemEffect key={index} index={index}>
                                            <div className={"h-[200px]"}>
                                                <MediaView key={index} media={video}/>
                                            </div>
                                        </ItemEffect>
                                    })
                                }
                            </div>
                        </CollapsibleContent>
                    </Collapsible>
                    <Collapsible
                        className={`w-full ${selectedSession.file_resources.length ? 'visible' : 'hidden'}`}>
                        <ReceiveCategory
                            title={`${selectedSession.file_resources.length} File${selectedSession.file_resources.length !== 1 ? 's' : ''}`}/>
                        <CollapsibleContent className={"space-y-2"}>
                            <div
                                className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-6 gap-x-4 gap-y-4 pb-8">
                                {
                                    selectedSession.file_resources.map((file: FileReceiveResourceViewModel, index: number) => {
                                        return <ItemEffect key={index} index={index}>
                                            <div className={"h-[180px]"}>
                                                <FileView key={index} file={file}/>
                                            </div>
                                        </ItemEffect>
                                    })
                                }
                            </div>
                        </CollapsibleContent>
                    </Collapsible>
                </div>
            </div>
        </div>
    </>
}

function ItemEffect(props: { children: ReactElement, index: number }) {
    const {children, index} = props
    return <MotionEffect
        key={index}
        slide={{
            direction: 'down',
        }}
        fade
        zoom
        inView
        delay={0.5 + index * 0.1}>
        {children}
    </MotionEffect>
}

function ReceiveCategory(props: {
    title: string
}) {
    const {title} = props
    return <CollapsibleTrigger asChild>
        <Button variant="secondary" className="w-full cursor-pointer mb-4 rounded-xl h-10 border border-primaryText/5">
            <div className={"flex flex-row w-full items-center justify-between"}>
                <p className={"font-poppins font-bold text-md"}>{title}</p>
                <ChevronsUpDown className="h-4 w-4"/>
                <span className="sr-only">Toggle</span>
            </div>
        </Button>
    </CollapsibleTrigger>
}

function Board() {
    const list = receiveList.map((item) => {
        return {
            id: item.id,
            name: item.peer_name,
            info: item.display_datetime,
            icon: <Globe className="text-primaryText size-4"/>,
            pinned: false
        } as unknown as PinListItem
    });

    return <>
        <div className={"flex flex-col border-1 w-full h-full bg-sidebar rounded-xl p-4 gap-8"}>
            <p className={"text-lg font-poppins font-bold pl-2"}>Receive sessions</p>
            <div className={"flex flex-col justify-start font-poppins text-primaryText gap-2"}>
                <p className={"opacity-80 text-sm pl-2"}>Search session</p>
                <Input className={"rounded-xl font-poppins"} placeholder={"Enter an id or an url, eg: 123123"}/>
            </div>
            <PinList className={"space-y-2"} items={list} labels={{pinned: "Starred", unpinned: "List"}}/>
        </div>
    </>
}

function FileView(props: { file: FileReceiveResourceViewModel }) {
    const {file} = props;
    const isMobile = useIsMobile();
    const model = file.model;

    let thumbnailPath = (model.thumbnail_path as LocalResourcePathVariantAbsolutePath)?.value;
    if (!thumbnailPath) {
        thumbnailPath = model.type instanceof ResourceTypeVariantFolder
            ? "/folder.svg"
            : "/file.svg";
    }

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    console.log(model)

    return (
        <div
            className="w-full h-full overflow-hidden rounded-2xl relative group bg-muted p-6 border-1 border-primaryText/5 overflow-clip">
            <div
                className={clsx(
                    "absolute z-20 inset-0 flex items-center justify-center",
                    isMobile ? "opacity-100" : "opacity-0 group-hover:opacity-100 w-full h-full bg-blackBase/40 transition-opacity duration-300"
                )}>
                <Button className={"rounded-xl"}>
                    <Download/>
                </Button>
            </div>

            <div className="relative aspect-square w-full scale-80">
                <Image
                    className="w-full h-auto text-primaryText"
                    layout="fill"
                    alt={`${model.type}`}
                    src={thumbnailPath}
                />
            </div>

            {/* Metadata */}
            <div className="flex flex-col text-white items-center mt-1">
                <p className="text-md font-poppins">{model.name}</p>
                <p className="text-sm text-white/80 font-poppins">{displaySize}</p>
            </div>
        </div>
    );
}

function MediaView(props: {
    media: ImageReceiveResourceViewModel | VideoReceiveResourceViewModel,
}) {
    const {media} = props;

    const model: SelectedResourceViewModel = media.model;
    const isVideo = media instanceof VideoReceiveResourceViewModel;
    const isMobile = useIsMobile();

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <div className="w-full h-full overflow-hidden rounded-2xl relative group">
            <div
                className={clsx(
                    "z-3 w-full h-[90%] absolute bg-gradient-to-t from-blackBase/70 bottom-0",
                    isMobile
                        ? "opacity-100"
                        : "opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                )}
            ></div>

            {
                isVideo && <div className={"absolute z-10 flex w-full h-full justify-center items-center"}>
                    <Button>
                        <Play/>
                    </Button>
                </div>
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
                    <p className="font-poppins text-primaryText text-md">
                        {model.name}
                    </p>
                    <p className="font-poppins text-sm text-primaryText/80">
                        {displaySize}
                    </p>
                </div>
                <Button className={"rounded-xl"}>
                    <Download/>
                </Button>
            </div>

            {
                isVideo
                    ? <video
                        className="w-auto h-full rounded-2xl object-cover"
                        muted>
                        <source src={model.display_path}/>
                        Your browser does not support the video tag.
                    </video>
                    : <Image
                        layout="fill"
                        className="rounded-2xl"
                        alt={model.name}
                        src={model.display_path}
                    />
            }
        </div>
    );
}