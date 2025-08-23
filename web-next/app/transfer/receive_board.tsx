'use client'
import * as React from "react";
import {
    AppEventVariantTransfer,
    FileReceiveResourceViewModel,
    ImageReceiveResourceViewModel,
    LocalResourcePathVariantAbsolutePath,
    ReceiveCloudSessionViewModel,
    ResourceTypeVariantFolder,
    SelectedResourceViewModel,
    TransferEventVariantFindPublicSession,
    TransferEventVariantViewPublicSession,
    TransferTypeVariantReceive,
    VideoReceiveResourceViewModel
} from 'shared_types/types/shared_types'
import {
    ArrowDown,
    ChevronsUpDown, Download,
    Globe, LoaderCircle, Play
} from 'lucide-react'
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
import {Avatar, AvatarImage} from "@/components/ui/avatar";
import {useUrlState} from "@/hooks/use-url";

export default function ReceiveBoard() {
    return <>
        <div
            className="h-[950px] max-h-[85vh] w-full rounded-xl bg-blackBase flex flex-col border-primaryText/20 items-center justify-center border-1">
            <div className={"grid grid-cols-11 w-full h-full gap-2"}>
                <div className={"col-span-3 lg:col-span-3 h-full"}>
                    <Board/>
                </div>
                <div className={`col-span-8 lg:col-span-8 h-full p-4 flex flex-col overflow-y-scroll pb-20`}>
                    <ContentBoard/>
                </div>
            </div>
        </div>
    </>
}

function ContentBoard() {
    const selectedSession = core.useSelectedSession()
    const transferState = core.useTransferState()
    const coreReady = core.useCoreReady()
    const [url, setUrl] = useUrlState(['session'])
    const isLoading = !selectedSession?.file_resources.length && !selectedSession?.image_resources.length && !selectedSession?.video_resources.length
    const [enteredPassword, setEnteredPassword] = useState<string>((selectedSession as ReceiveCloudSessionViewModel)?.password ?? '')

    useEffect(() => {
        if (selectedSession instanceof ReceiveCloudSessionViewModel) {
            if (selectedSession.alias) {
                setUrl({
                    session: selectedSession.alias ?? ''
                })
            }
        }
    }, [selectedSession]);

    useEffect(() => {
        if (url.session && coreReady) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantFindPublicSession(url.session)))
        }
    }, [coreReady]);

    useEffect(() => {
        if (!url?.session || !transferState?.received_cloud_sessions?.length) return

        const session = transferState?.received_cloud_sessions?.find((it) => {
            return it.alias === url!.session!
        })

        if (session) {
            core.updateSelectedSession(session)
        }

    }, [transferState?.received_cloud_sessions?.length]);

    const onSelected = () => {
        if (!selectedSession) {
            return
        }

        core.update(new AppEventVariantTransfer(new TransferEventVariantViewPublicSession(
            enteredPassword ? enteredPassword : null, selectedSession!.id, new TransferTypeVariantReceive()
        )))
    }

    useEffect(() => {
        if (selectedSession instanceof ReceiveCloudSessionViewModel) {
            const cloud = selectedSession as ReceiveCloudSessionViewModel
            if (!cloud.is_required_password && isLoading) {
                core.update(new AppEventVariantTransfer(new TransferEventVariantViewPublicSession(
                    null,
                    cloud.id,
                    new TransferTypeVariantReceive(),
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
        if (cloud.is_required_password && !cloud.password && isLoading) {
            return <div className={"text-foreground w-full h-full flex flex-col justify-center items-center gap-2"}>
                <div className={"w-[50%] flex flex-col gap-4"}>
                    <p className={"font-poppins text-muted-foreground flex flex-row items-center"}>
                        <Image alt={"lock"} width={10} height={10} className={"w-7 text-white bg-muted p-1.5 rounded-lg mr-2 h-7"} src={"/lock.svg"} color={'white'}/>
                        This session is password protected</p>
                    <Input
                        className="h-10"
                        placeholder="Enter password"
                        value={enteredPassword}
                        onChange={(e) => setEnteredPassword(e.target.value)}
                        type="password"
                        onKeyDown={(e) => {
                            if (e.key === 'Enter') {
                                onSelected()
                            }
                        }}
                    />
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
            className={`w-full h-fit ${selectedSession?.file_resources.length ? 'visible' : 'hidden'}`}>
            <ReceiveCategory
                title={`${selectedSession?.file_resources.length} File${selectedSession?.file_resources.length !== 1 ? 's' : ''}`}/>
            <CollapsibleContent className={"h-full"}>
                <div
                    className="flex flex-col gap-4 h-fit min-h-[400px]">
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
    const [url, setUrl] = useUrlState(['session'])

    const message = core.useMessage('MessageReasonVariantFailedToFindPublicSession')
    const [keywords, setKeywords] = useState<string>()

    useEffect(() => {
        if (url.session) {
            setKeywords(url.session)
        }
    }, []);

    const handleFind = () => {
        message?.resolveMessage()
        if (!keywords || keywords.trim() === '') {
            setUrl({
                session: undefined
            })

            return
        }

        setUrl({
            session: keywords
        })

        core.update(new AppEventVariantTransfer(new TransferEventVariantFindPublicSession(keywords)))
    }

    const sessions = transferState?.received_cloud_sessions.filter((it) => {
        if (keywords) {
            return it?.sender_name?.toLowerCase()?.includes(keywords.toLowerCase()) ?? false
        }

        return true
    }) || []

    return <>
        <div className={"flex flex-col border-1 w-full h-full bg-sidebar rounded-xl p-4 gap-8"}>
            <div className={"flex flex-col justify-start text-primaryText gap-4"}>
                <p className={"opacity-80 text-sm"}>Find session</p>
                <div className={"relative"}>
                    <Input value={keywords || ''} className={"rounded-md font-poppins pr-8 min-h-10 h-fit"} placeholder={"Session name or url"}
                           onChange={(it) => setKeywords(it.target.value.replace(/\s/g, ''))}
                           onKeyDown={(e) => {
                               if (e.key === 'Enter') {
                                   handleFind()
                               }
                           }}/>
                    {keywords && (
                        <Button
                            variant="ghost"
                            size="sm"
                            className={"absolute right-1 top-1/2 transform -translate-y-1/2 h-6 w-6 p-0"}
                            onClick={() => {
                                setKeywords('')
                                setUrl({
                                    session: undefined
                                })
                            }}
                        >
                            ×
                        </Button>
                    )}
                </div>
                {message.message && <p className={"text-foreground text-sm"}>{message.message?.field0}</p>}
                <Button className={"w-fit h-8"} onClick={handleFind}>Find</Button>
            </div>
            <div className={"flex flex-col gap-3"}>
                {!!sessions.length && <p className={"font-poppins text-muted-foreground"}>Public</p>}
                <MotionHighlight hover
                                 className={"pointer-events-none flex flex-col gap-2 rounded-2xl bg-primaryText/10"}>
                    {
                        sessions.map((item, index) => {
                            return <TransferSession
                                onPress={() => {
                                    core.updateSelectedSession(item)
                                }}
                                name={item.sender_name}
                                progress={0}
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

function TransferSession(props: {
    name: string,
    display_datetime: string,
    progress: number,
    avatar_url: string,
    is_public: boolean,
    is_required_password: boolean,
    onPress: () => void
}) {
    const {
        name,
        display_datetime,
        progress,
        avatar_url,
        is_public,
        is_required_password,
        onPress = () => {
        }
    } = props;

    return <>
        <button
            onClick={onPress}
            className={"w-full flex flex-row bg-muted rounded-2xl items-center px-2 py-2 h-fit max-h-[80px] border-1 border-primaryText/5 justify-between"}>
            <div className={"flex flex-row items-center gap-5"}>
                <div
                    className={"bg-bluePrimary rounded-xl aspect-square justify-center items-center text-primaryText flex h-[34px] w-[34px] relative"}>
                    <Avatar className={"p-1"}>
                        <AvatarImage src={avatar_url}/>
                    </Avatar>
                    {is_public && <Globe className={"bg-bluePrimary w-5 h-5 p-0.5 text-white rounded-full absolute bottom-[-20%] right-[-24%]"}/>}
                </div>
                <div className={"flex flex-col gap-1 items-start"}>
                    <p className={"text-primaryText text-sm text-start"}>{name}</p>
                    <p className={"text-primaryText/70 text-xs"}>{display_datetime}</p>
                </div>
            </div>
            {!!progress && <CircleProgress progress={progress} size={30}/>}
            {is_required_password && <Image alt={"lock"} width={10} height={10} className={"w-4 text-white mr-2 bg-muted h-4"} src={"/lock.svg"} color={'white'}/>}
        </button>
    </>
}

function FileView(props: {
    file: FileReceiveResourceViewModel
}) {
    const {file} = props;
    const model = file.model;

    const thumbnailPath = (model.thumbnail_path as LocalResourcePathVariantAbsolutePath)?.value;
    const fallbackThumbnail = model.type instanceof ResourceTypeVariantFolder
        ? "/folder.svg"
        : "/file.svg";

    const [imgSrc, setImgSrc] = useState(thumbnailPath)
    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <div
            className="gap-3 flex flex-row w-full justify-between items-center h-fit overflow-hidden rounded-2xl relative group bg-black-base p-2 border-1 border-primaryText/5 bg-muted/50 hover:bg-muted-foreground/30">
            <div className={"flex flex-row gap-3"}>
            <div className="relative aspect-square w-12 h-12">
                <Image
                    className="w-full h-auto bg-muted rounded-xl p-1.5"
                    layout="fill"
                    objectFit="cover"
                    alt="Thumbnail"
                    src={imgSrc}
                    onError={() => setImgSrc(fallbackThumbnail)}
                />
            </div>

            {/* Metadata */}
            <div className="flex flex-col text-white items-start mt-1">
                <p className="text-sm text-center font-poppins break-words w-full">{model.name}</p>
                <p className="text-sm text-center text-white/80 font-poppins">{displaySize}</p>
            </div>
            </div>
            {
                file.is_completed
                    ? <a className={"rounded-xl p-2 bg-bluePrimary/80"} href={(file.model.path as LocalResourcePathVariantAbsolutePath).value}>
                        <ArrowDown color={'var(--foreground)'}/>
                    </a>
                    : <>
                        <CircleProgress progress={file.completion} size={30}/>
                    </>
            }
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

    let thumbnailPath = (model.thumbnail_path as LocalResourcePathVariantAbsolutePath)?.value;
    if (!thumbnailPath) {
        thumbnailPath = isVideo ? '/file-video.svg' : '/file-image.svg';
    }

    return (
        <div className="w-full h-full bg-muted-foreground overflow-hidden rounded-2xl relative group">
            <div
                className={clsx(
                    "z-3 w-full h-[100%] absolute bg-gradient-to-t from-blackBase bottom-0",
                    isMobile
                        ? "opacity-100"
                        : "opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                )}
            ></div>

            {
                isVideo && <div className={"absolute z-2 flex w-full h-full justify-center items-center"}>
                    <Button className={"bg-muted-foreground hover:bg-muted"}>
                        <Play color={"white"} fill={"white"}/>
                    </Button>
                </div>
            }

            <div
                className={clsx(
                    "flex w-full flex-row z-4 bottom-0 absolute items-center px-3 justify-between py-2",
                    isMobile
                        ? "opacity-100"
                        : "opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                )}
            >
                <div className="flex flex-col items-start gap-1 w-[80%]">
                    <p className="text-primaryText text-sm break-words w-full">
                        {model.name}
                    </p>
                    <p className="text-sm text-primaryText/80">
                        {displaySize}
                    </p>
                </div>
                    <div className={"flex-1 w-fit flex"}>
                    {media.is_completed
                        ? <a className={"rounded-xl bg-bluePrimary/80 p-2"} href={(media.model.path as LocalResourcePathVariantAbsolutePath).value}>
                            <ArrowDown color={'white'}/>
                          </a>
                        : <>
                            <CircleProgress progress={media.completion} size={30}/>
                        </>
                }</div>

            </div>

            <Image
                className={`object-cover w-full h-full rounded-2xl fill-white bg-muted/40 ${model.display_path ? '' : 'p-10'}`}
                alt={model.name}
                src={thumbnailPath}
                fill
                sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
            />
        </div>
    );
}
