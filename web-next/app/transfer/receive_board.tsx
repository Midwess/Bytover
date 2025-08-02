'use client'
import * as React from "react";
import {
    AppEventVariantTransfer,
    FileReceiveResourceViewModel,
    ImageReceiveResourceViewModel,
    LocalResourcePathVariantAbsolutePath, MessageReasonVariantFailedToFindPublicSession, ReceiveCloudSessionViewModel,
    ReceiveSessionViewModel,
    ResourceTypeVariantFolder,
    SelectedResourceViewModel, TransferEventVariantFindPublicSession, TransferEventVariantViewPublicSession,
    VideoReceiveResourceViewModel
} from 'shared_types/types/shared_types'
import {
    ChevronsUpDown, Download,
    Globe, LoaderCircle, LoaderCircleIcon, Lock, Play
} from 'lucide-react'
import {receiveList} from "@/app/mock_data";
import {Button} from '@/components/ui/button'
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleTrigger,
} from '@/components/animate-ui/radix/collapsible';
import {ReactElement, useEffect, useState} from "react";
import {MotionEffect} from '@/components/animate-ui/effects/motion-effect';
import Image from "next/image";
import {useIsMobile} from "@/hooks/use-mobile";
import clsx from "clsx";
import {Input} from "@/components/ui/input";
import {MotionHighlight} from "@/components/animate-ui/effects/motion-highlight";
import CircleProgress from "@/components/ui/progress";
import core from "@/wasm/wasm_core";

export default function ReceiveBoard() {
    return <>
        <div
            className="h-[950px] max-h-[85vh] w-full rounded-xl bg-blackBase flex flex-col border-primaryText/20 items-center justify-center border-1">
            <div className={"grid grid-cols-12 w-full h-full gap-2"}>
                <div className={"col-span-4 lg:col-span-3 h-full"}>
                    <Board />
                </div>
                <div className={`col-span-8 lg:col-span-9 h-full p-4 flex flex-col overflow-y-scroll pb-20`}>
                    <ContentBoard/>
                </div>
            </div>
        </div>
    </>
}

function ContentBoard() {
    const selectedSession = core.useSelectedSession()
    const isLoading = !selectedSession?.file_resources.length && !selectedSession?.image_resources.length && !selectedSession?.video_resources.length
    const [enteredPassword, setEnteredPassword] = useState<string>('')
    const onSelected = () => {
        if (!selectedSession) {
            return
        }

        core.update(new AppEventVariantTransfer(new TransferEventVariantViewPublicSession(
            enteredPassword ? null : enteredPassword, selectedSession!.id
        )))
    }

    useEffect(() => {
        if (selectedSession instanceof ReceiveCloudSessionViewModel) {
            let cloud = selectedSession as ReceiveCloudSessionViewModel
            if (!cloud.is_required_password && isLoading) {
                core.update(new AppEventVariantTransfer(new TransferEventVariantViewPublicSession(
                    null,
                    cloud.id
                )))
            }
        }
    }, [selectedSession, isLoading]);

    if (!selectedSession) {
        return <div className={"w-full h-full flex justify-center items-center gap-2"}>
            <p>No session selected</p>
        </div>
    }

    if (selectedSession instanceof ReceiveCloudSessionViewModel) {
        const cloud = selectedSession as ReceiveCloudSessionViewModel
        if (cloud.is_required_password) {
            return <div className={"text-foreground w-full h-full flex flex-col justify-center items-center gap-2"}>
                <div className={"w-[50%] flex flex-col gap-4"}>
                    <p className={"font-poppins text-muted-foreground flex flex-row gap-1"}><Lock
                        className={"w-4"}/> This session is password protected</p>
                    <Input className={""} placeholder={"Enter password"} value={enteredPassword} onChange={(it) => {
                        setEnteredPassword(it.target.value)
                    }} type={"password"}/>
                    <Button onClick={onSelected} className={"w-fit"}>Continue</Button>
                </div>
            </div>
        }
    }

    if (isLoading) {
        return <div className={"w-full h-full flex justify-center items-center gap-2"}>
            <LoaderCircle className={"animate-spin"}/>
            <p>Loading...</p>
        </div>
    }

    return <div>
        <Collapsible
            className={`w-full ${selectedSession?.image_resources.length ? 'visible' : 'hidden'}`}>
            <ReceiveCategory
                title={`${selectedSession?.image_resources.length} Image${selectedSession?.image_resources.length !== 1 ? 's' : ''}`}/>
            <CollapsibleContent className={"space-y-2"}>
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-x-4 gap-y-4 pb-8">
                    {
                        selectedSession?.image_resources.map((image: ImageReceiveResourceViewModel, index: number) => {
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
            className={`w-full ${selectedSession?.video_resources.length ? 'visible' : 'hidden'}`}>
            <ReceiveCategory
                title={`${selectedSession?.video_resources.length} Video${selectedSession?.video_resources.length !== 1 ? 's' : ''}`}/>
            <CollapsibleContent className={"space-y-2"}>
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-x-4 gap-y-4 pb-8">
                    {
                        selectedSession?.video_resources.map((video: VideoReceiveResourceViewModel, index: number) => {
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
            className={`w-full ${selectedSession?.file_resources.length ? 'visible' : 'hidden'}`}>
            <ReceiveCategory
                title={`${selectedSession?.file_resources.length} File${selectedSession?.file_resources.length !== 1 ? 's' : ''}`}/>
            <CollapsibleContent className={"space-y-2"}>
                <div
                    className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 xl:grid-cols-7 gap-x-4 gap-y-4 pb-8">
                    {
                        selectedSession?.file_resources.map((file: FileReceiveResourceViewModel, index: number) => {
                            return <ItemEffect key={index} index={index}>
                                <div className={"h-fit"}>
                                    <FileView key={index} file={file}/>
                                </div>
                            </ItemEffect>
                        })
                    }
                </div>
            </CollapsibleContent>
        </Collapsible>
    </div>
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
                <p className={"font-bold h2 text-md"}>{title}</p>
                <ChevronsUpDown className="h-4 w-4"/>
                <span className="sr-only">Toggle</span>
            </div>
        </Button>
    </CollapsibleTrigger>
}

function Board() {
    const transferState = core.useTransferState()
    const sessions = transferState?.received_cloud_sessions || []
    const message = core.useMessage('MessageReasonVariantFailedToFindPublicSession')
    const [keywords, setKeywords] = useState<string>('')

    return <>
        <div className={"flex flex-col border-1 w-full h-full bg-sidebar rounded-xl p-4 gap-8"}>
            <h2 className={"text-lg font-bold pl-2"}>Receive sessions</h2>
            <div className={"flex flex-col justify-start text-primaryText gap-4"}>
                <p className={"opacity-80 text-sm"}>Search session</p>
                <Input className={"rounded-md font-poppins"} placeholder={"Enter an id or an url, eg: 123123"} onChange={(it) => setKeywords(it.target.value)}/>
                {message.message && <p className={"text-foreground text-sm"}>{message.message?.field0}</p>}
                <Button className={"w-fit"} onClick={() => {
                    message?.resolveMessage()
                    core.update(new AppEventVariantTransfer(new TransferEventVariantFindPublicSession(keywords)))
                }}>Find</Button>
            </div>
            <div className={"flex flex-col gap-3"}>
                {!!sessions.length && <p className={"font-poppins text-muted-foreground"}>Public</p>}
                <MotionHighlight hover className={"pointer-events-none flex flex-col gap-2 rounded-2xl bg-primaryText/10"}>
                    {
                        sessions.map((item, index) => {
                            return <TransferSession
                                onPress={() => {
                                    core.updateSelectedSession(item)
                                }}
                                name={item.sender_name}
                                display_datetime={item.display_datetime}
                                key={index}
                                is_public={true}
                                avatar_url={item.avatar_url}
                                is_required_password={item.is_required_password}
                            />
                        })
                    }
                </MotionHighlight>
            </div>
        </div>
    </>
}

function TransferSession(props: any) {
    const {
        name,
        display_datetime,
        progress,
        avatar_url,
        is_public,
        is_required_password,
        onPress = () => {}
    } = props;

    return <>
        <button
            onClick={onPress}
            className={"w-full flex flex-row bg-muted rounded-2xl items-center px-2 py-2 max-h-[60px] border-1 border-primaryText/5 justify-between"}>
            <div className={"flex flex-row items-center gap-3"}>
                <div
                    className={"bg-bluePrimary rounded-xl aspect-square justify-center items-center text-primaryText flex h-[34px] w-[34px]"}>
                    {is_public && <Globe className={"text-primaryText w-full h-full m-2"}/>}
                </div>
                <div className={"flex flex-col gap-0"}>
                    <p className={"text-primaryText text-sm"}>{name}</p>
                    <p className={"text-primaryText/70 text-xs"}>{display_datetime}</p>
                </div>
            </div>
            {progress && <CircleProgress progress={progress} size={30}/>}
            {is_required_password && <Lock className={"w-4 bg-muted text-muted"} color={'gray'}/>}
        </button>
    </>
}

function FileView(props: {
    file: FileReceiveResourceViewModel
}) {
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

    return (
        <div
            className="w-full h-fit overflow-hidden rounded-2xl relative group bg-muted p-6 border-1 border-primaryText/5">
            <div
                className={clsx(
                    "absolute z-20 inset-0 flex items-center justify-center",
                    isMobile ? "opacity-100" : "opacity-0 group-hover:opacity-100 w-full h-full bg-blackBase/40 transition-opacity duration-300"
                )}>
                <Button className={"rounded-xl"}>
                    <Download/>
                </Button>
            </div>

            <div className="relative aspect-square w-full scale-60">
                <Image
                    className="w-full h-auto text-primaryText"
                    layout="fill"
                    alt={`${model.type}`}
                    src={thumbnailPath}
                />
            </div>

            {/* Metadata */}
            <div className="flex flex-col text-white items-center mt-1">
                <p className="text-sm text-center font-poppins break-words w-full">{model.name}</p>
                <p className="text-sm text-center text-white/80 font-poppins">{displaySize}</p>
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
                    <p className="text-primaryText text-md">
                        {model.name}
                    </p>
                    <p className="text-sm text-primaryText/80">
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
