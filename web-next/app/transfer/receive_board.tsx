'use client'
import * as React from "react";
import {
    AppEventVariantTransfer,
    FileReceiveResourceViewModel,
    ImageReceiveResourceViewModel, MessageReasonVariantFailedToFindPublicSession,
    MessageReasonVariantFailedToLoadSession,
    ReceiveCloudSessionViewModel,
    ReceiveSessionViewModel,
    ResourceTypeVariantFolder,
    SelectedResourceViewModel, TransferEventVariantCancelTransfer,
    TransferEventVariantFindPublicSession,
    TransferEventVariantViewPublicSession,
    TransferTypeVariantReceive,
    VideoReceiveResourceViewModel
} from 'shared_types/types/shared_types'
import {
    ArrowDown,
    Book,
    ChevronsUpDown, Download,
    Globe, ImageUpIcon, LoaderCircle, MoreVertical, Play, Wifi
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/animate-ui/radix/dropdown-menu';
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleTrigger,
} from '@/components/animate-ui/radix/collapsible';
import { ReactElement, useCallback, useEffect, useState } from "react";
import { MotionEffect } from '@/components/animate-ui/effects/motion-effect';
import Image from "next/image";
import { useIsMobile } from "@/hooks/use-mobile";
import clsx from "clsx";
import { Input } from "@/components/ui/input";
import { MotionHighlight } from "@/components/animate-ui/effects/motion-highlight";
import CircleProgress from "@/components/ui/progress";
import core from "@/wasm/wasm_core";
import { Avatar, AvatarImage } from "@/components/ui/avatar";
import { useUrlState } from "@/hooks/use-url";
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

export default function ReceiveBoard() {
    return (
        <div className="rounded-xl border-2 overflow-hidden min-h-[450px] max-h-[70vh] sm:max-h-[80vh] h-[950px]">
            <SidebarProvider className="h-[100%]">
                <Sidebar collapsible="icon" className="h-full bg-card overflow-hidden border-2 border-muted rounded-xl mb-1">
                    <SidebarHeader className="rounded-tl-xl">
                        <SessionSelector />
                    </SidebarHeader>
                    <SidebarContentWrapper />
                    <SidebarRail />
                </Sidebar>
                <SidebarInset className="flex flex-col h-[100%] min-h-0">
                    <header className="flex h-10 md:h-16 shrink-0 items-center gap-2 transition-[width,height] ease-linear group-has-[[data-collapsible=icon]]/sidebar-wrapper:h-12">
                        <div className="flex items-center gap-2 px-4">
                            <SidebarTrigger className="-ml-1" />
                            <Separator orientation="vertical" className="mr-2 h-4" />
                        </div>
                    </header>
                    <div className="flex flex-1 flex-col min-h-0 px-2 pt-0 overflow-y-auto">
                        <ContentBoard />
                    </div>
                </SidebarInset>
            </SidebarProvider>
        </div>
    );
}

function SessionSelector() {
    return (
        <SidebarMenu>
            <SidebarMenuItem>
                <SidebarMenuButton
                    size="lg"
                    className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
                >
                    <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
                        <Book className="size-4" />
                    </div>
                    <div className="grid flex-1 text-left text-sm leading-tight">
                        <span className="truncate font-semibold">
                            Sessions
                        </span>
                    </div>
                </SidebarMenuButton>
            </SidebarMenuItem>
        </SidebarMenu>
    );
}

function SidebarContentWrapper() {
    const { state } = useSidebar();

    if (state === 'collapsed') {
        return null;
    }

    return (
        <SidebarContent className="rounded-bl-xl px-1">
            <Board />
        </SidebarContent>
    );
}

function ContentBoard() {
    const selectedSessionId = core.useSelectedSession()?.id
    const selectedSession = core.useSession(selectedSessionId ?? '');
    const cloudSessions = core.useCloudSessionsList()
    const isCloud = selectedSession instanceof ReceiveCloudSessionViewModel
    const coreReady = core.useCoreReady()
    const [url, setUrl] = useUrlState(['session'])
    const isLoading = selectedSession instanceof ReceiveCloudSessionViewModel
        ? (selectedSession as ReceiveCloudSessionViewModel)?.is_loading
        : false
    const loadMessage = core.useMessage(new MessageReasonVariantFailedToLoadSession(BigInt(selectedSessionId ?? '0')));
    const [enteredPassword, setEnteredPassword] = useState<string>((selectedSession as ReceiveCloudSessionViewModel)?.password ?? '')
    const isMobile = useIsMobile();

    useEffect(() => {
        if (selectedSession && selectedSession instanceof ReceiveCloudSessionViewModel) {
            if (selectedSession.alias) {
                setUrl({
                    session: selectedSession.alias ?? ''
                })
            }
        }
    }, [selectedSession])

    useEffect(() => {
        if (url.session && coreReady) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantFindPublicSession(url.session)))
        }
    }, [coreReady])

    useEffect(() => {
        if (!url?.session || !cloudSessions?.length) return

        const session = cloudSessions?.find((it) => {
            return it.alias === url!.session!
        })

        if (session) {
            core.updateSelectedSession(session)
        }
    }, [cloudSessions?.length])

    const onSelected = () => {
        if (!selectedSession) {
            return
        }

        core.update(new AppEventVariantTransfer(new TransferEventVariantViewPublicSession(
            enteredPassword ? enteredPassword : null, BigInt(selectedSession!.id), new TransferTypeVariantReceive()
        )))
    }

    useEffect(() => {
        if (selectedSession && selectedSession instanceof ReceiveCloudSessionViewModel) {
            const cloud = selectedSession as ReceiveCloudSessionViewModel
            if (!cloud.is_required_password && isLoading) {
                core.update(new AppEventVariantTransfer(new TransferEventVariantViewPublicSession(
                    null,
                    BigInt(cloud.id),
                    new TransferTypeVariantReceive(),
                )))
            }
        }
    }, [selectedSession?.id]);

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
                        <Image alt={"lock"} width={10} height={10}
                            className={"w-7 text-white bg-muted p-1.5 rounded-lg mr-2 h-7"} src={"/lock.svg"}
                            color={'white'} />
                        This session is password protected</p>
                    <input type="password" name="fake-password" style={{ display: 'none' }} />
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
                    <Button onClick={onSelected} className={"w-fit bg-foreground"}>Continue</Button>
                </div>
            </div>
        }
    }

    if (!selectedSession) {
        return <div className={"w-full h-full flex justify-center items-center gap-2"}>
            <p>No session selected</p>
        </div>
    }

    if (isLoading) {
        return <div className={"w-full h-full flex justify-center items-center gap-2"}>
            {
                loadMessage.message
                    ? <p>{loadMessage.message}</p>
                    : <>
                        <LoaderCircle className={"animate-spin"} />
                        <p>Loading...</p>
                    </>
            }
        </div>
    }

    return (
        <div className="w-full h-full flex flex-col gap-4 pb-4">
            <Collapsible
                className={`w-full ${selectedSession?.image_resources.length ? 'visible' : 'hidden'}`}>
                <ReceiveCategory
                    title={`${selectedSession?.image_resources.length} Image${selectedSession?.image_resources.length !== 1 ? 's' : ''}`} />
                <CollapsibleContent className={"space-y-2"}>
                    <div className="flex flex-col md:grid md:grid-cols-3 gap-4 pb-8">
                        {
                            selectedSession?.image_resources.map((image: ImageReceiveResourceViewModel, index: number) => {
                                return <ItemEffect key={index} index={index}>
                                    <div className={isMobile ? "h-auto" : "h-[200px]"}>
                                        <MediaView key={index} id={image.model.order_id} isCloud={isCloud} />
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
                    title={`${selectedSession?.video_resources.length} Video${selectedSession?.video_resources.length !== 1 ? 's' : ''}`} />
                <CollapsibleContent className={"space-y-2"}>
                    <div className="flex flex-col md:grid md:grid-cols-3 gap-4 pb-8">
                        {
                            selectedSession?.video_resources.map((video: VideoReceiveResourceViewModel, index: number) => {
                                return <ItemEffect key={index} index={index}>
                                    <div className={isMobile ? "h-auto" : "h-[200px]"}>
                                        <MediaView key={index} id={video.model.order_id} isCloud={isCloud} />
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
                    title={`${selectedSession?.file_resources.length} File${selectedSession?.file_resources.length !== 1 ? 's' : ''}`} />
                <CollapsibleContent className={"h-full"}>
                    <div
                        className="flex flex-col gap-4 h-fit min-h-[400px]">
                        {
                            selectedSession?.file_resources.map((file: FileReceiveResourceViewModel, index: number) => {
                                return <ItemEffect key={file.model.order_id} index={index}>
                                    <div className={"h-fit"}>
                                        <FileView key={file.model.order_id} id={file.model.order_id} isCloud={isCloud} />
                                    </div>
                                </ItemEffect>
                            })
                        }
                    </div>
                </CollapsibleContent>
            </Collapsible>
        </div>
    );
}

function ItemEffect(props: { children: ReactElement, index: number }) {
    const { children, index } = props
    return <MotionEffect
        key={index}
        slide={{
            direction: 'down',
        }}
        fade
        zoom
        delay={0.2 + index * 0.1}>
        {children}
    </MotionEffect>
}

function ReceiveCategory(props: {
    title: string
}) {
    const { title } = props
    return <CollapsibleTrigger asChild>
        <Button variant="secondary" className="w-full cursor-pointer mb-4 rounded-xl h-10 border border-primaryText/5">
            <div className={"flex flex-row w-full items-center justify-between"}>
                <p className={"font-bold h2 text-md"}>{title}</p>
                <ChevronsUpDown className="h-4 w-4" />
                <span className="sr-only">Toggle</span>
            </div>
        </Button>
    </CollapsibleTrigger>
}

function Board() {
    const publicSessions = core.useCloudSessionsList()
    const nearbySessions = core.useNearbySessionsList()

    const [url, setUrl] = useUrlState(['session'])

    const message = core.useMessage(new MessageReasonVariantFailedToFindPublicSession())
    const [keywords, setKeywords] = useState<string>()

    useEffect(() => {
        if (url.session) {
            setKeywords(url.session)
        }
    }, []);

    const handleFind = useCallback(() => {
        message?.resolveMessage()
        console.log(keywords)
        setUrl({ session: keywords?.trim() || null })

        core.update(new AppEventVariantTransfer(new TransferEventVariantFindPublicSession(keywords || '')))
    }, [keywords])

    return (
        <div className="flex flex-col gap-4 h-full overflow-y-auto px-2 pb-4">
            <div className={"flex flex-col justify-start text-primaryText gap-4"}>
                <p className={"opacity-80 text-sm"}>Find session</p>
                <div className={"relative"}>
                    <Input value={keywords || ''} className={"rounded-md font-poppins pr-8 min-h-10 h-fit"}
                        placeholder={"Session name or url"}
                        onChange={(it) => setKeywords(it.target.value.replace(/\s/g, ''))}
                        onKeyDown={(e) => {
                            if (e.key === 'Enter') {
                                handleFind()
                            }
                        }} />
                    <Button
                        variant="ghost"
                        size="sm"
                        className={"absolute right-1 top-1/2 transform -translate-y-1/2 text-xl cursor-pointer h-8 w-8 p-0"}
                        onClick={() => {
                            setKeywords('')
                            handleFind()
                        }}
                    >
                        ×
                    </Button>
                </div>
                {message.message && <p className={"text-foreground text-sm"}>{message.message}</p>}
                <Button className={"w-fit h-8 text-foreground bg-bluePrimary"} onClick={handleFind}>Find</Button>
            </div>
            <Collapsible className={"flex flex-col w-full gap-3"} defaultOpen={true}>
                <CollapsibleTrigger asChild className={"flex flex-row items-start"}>
                    <Button variant="secondary"
                        className="w-full justify-between items-center text-start flex flex-row cursor-pointer rounded-lg">
                        Nearby
                        <ChevronsUpDown className="h-4 w-4" />
                        <span className="sr-only">Toggle</span>
                    </Button>
                </CollapsibleTrigger>
                <CollapsibleContent className={"flex flex-col gap-3"}>
                    {nearbySessions.length === 0 && <p className={"text-muted-foreground text-sm pl-2"}>Empty</p>}
                    <MotionHighlight hover
                        className={"pointer-events-none flex flex-col gap-2 rounded-2xl bg-primaryText/10"}>
                        {
                            nearbySessions.map((item, index) => {
                                return <ItemEffect key={item.id} index={index}>
                                    <TransferSession
                                        onPress={() => {
                                            core.updateSelectedSession(item)
                                        }}
                                        id={item.id}
                                        key={item.id}
                                    />
                                </ItemEffect>
                            })
                        }
                    </MotionHighlight>
                </CollapsibleContent>
            </Collapsible>
            <Collapsible className={"flex flex-col w-full gap-3"} defaultOpen={true}>
                <CollapsibleTrigger asChild className={"flex flex-row items-start"}>
                    <Button variant="secondary"
                        className="w-full justify-between items-center text-start flex flex-row cursor-pointer rounded-lg">
                        Public
                        <ChevronsUpDown className="h-4 w-4" />
                        <span className="sr-only">Toggle</span>
                    </Button>
                </CollapsibleTrigger>
                <CollapsibleContent className={"flex flex-col gap-3"}>
                    {publicSessions.length === 0 && <p className={"text-muted-foreground text-sm pl-2"}>Empty</p>}
                    <MotionHighlight
                        hover
                        className={"pointer-events-none flex flex-col gap-2 rounded-2xl bg-primaryText/10"}>
                        {
                            publicSessions.map((item, index) => {
                                return <ItemEffect key={item.id} index={index}><TransferSession
                                    onPress={() => {
                                        core.updateSelectedSession(item)
                                    }}
                                    id={item.id}
                                    key={item.id}
                                />
                                </ItemEffect>
                            })
                        }
                    </MotionHighlight>
                </CollapsibleContent>
            </Collapsible>
        </div>
    );
}

function TransferSession(props: {
    id: string,
    onPress: () => void
}) {
    const {
        id,
        onPress = () => {
        }
    } = props;

    const session = core.useSession(id);

    if (!session) {
        return null;
    }

    const is_public = session instanceof ReceiveCloudSessionViewModel;

    const name = is_public
        ? (session as ReceiveCloudSessionViewModel).sender_name
        : (session as ReceiveSessionViewModel).peer_name;
    const display_datetime = session.display_datetime;
    const avatar_url = is_public
        ? (session as ReceiveCloudSessionViewModel).avatar_url
        : (session as ReceiveSessionViewModel).peer_avatar?.url;
    const is_required_password = is_public
        ? (session as ReceiveCloudSessionViewModel).is_required_password
        : false;

    const progress = is_public
        ? 0
        : (session as ReceiveSessionViewModel).progress || 0;
    const is_completed = is_public
        ? false
        : (session as ReceiveSessionViewModel).is_completed || false;

    return <>
        <button
            onClick={onPress}
            className={"w-full flex flex-row bg-muted rounded-2xl items-center px-2 py-2 h-fit max-h-[80px] border-1 border-primaryText/5 justify-between"}>
            <div className={"flex flex-row items-center gap-5"}>
                <div
                    className={"bg-bluePrimary rounded-xl aspect-square justify-center items-center text-primaryText flex h-[34px] w-[34px] relative"}>
                    <Avatar className={"p-1"}>
                        <AvatarImage src={avatar_url} />
                    </Avatar>
                    {is_public
                        ? <Globe
                            className={"bg-bluePrimary w-5 h-5 p-0.5 text-white rounded-full absolute bottom-[-20%] right-[-24%]"} />
                        : <Wifi
                            className={"bg-bluePrimary w-5 h-5 p-0.5 text-white rounded-full absolute bottom-[-20%] right-[-24%]"} />
                    }
                </div>
                <div className={"flex flex-col gap-1 items-start"}>
                    <p className={"text-primaryText text-sm text-start"}>{name}</p>
                    <p className={"text-primaryText/70 text-xs"}>{display_datetime}</p>
                </div>
            </div>
            {!!progress && !is_completed &&
                <CircleProgress center={is_public ? <Download /> : undefined} progress={progress} size={30}
                    onClick={() => {
                        if (!is_public) {
                            core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(BigInt(id), new TransferTypeVariantReceive())))
                        }
                    }} />}
            {is_required_password &&
                <Image alt={"lock"} width={10} height={10} className={"w-4 text-white mr-2 bg-muted h-4"}
                    src={"/lock.svg"} color={'white'} />}
        </button>
    </>
}

function FileView(props: {
    id: string,
    isCloud: boolean
}) {
    const { id, isCloud } = props;
    const file = core.useReceiveResource(id, isCloud);
    const model = file?.model;

    const isFolder = model?.type instanceof ResourceTypeVariantFolder;
    const fallbackThumbnail = isFolder ? "/folder.svg" : "/file.svg";

    const [thumbnailSource, setThumbnailSource] = useState<string | undefined>();

    useEffect(() => {
        if (!model?.thumbnail_path) {
            setThumbnailSource(undefined)
            return
        }

        if (model.thumbnail_path && !thumbnailSource) {
            core.getDownloadUrl(model.thumbnail_path).then(setThumbnailSource)
        }
    }, [model, model?.thumbnail_path, thumbnailSource]);

    const onDownloadClick = useCallback(() => {
        if (!model) return
        core.downloadFile(model.path, model.name)
    }, [model?.path, model?.name])

    if (!file || !model) return null;

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <div
            className="gap-4 flex flex-row w-full justify-between items-center h-fit overflow-hidden rounded-2xl relative group 
                       bg-muted/60
                       backdrop-blur-xl border border-white/10 p-3
                       transition-all duration-300 ease-out
                       hover:shadow-xl hover:shadow-white/10 hover:border-white/30 hover:backdrop-blur-sm
                       hover:bg-muted/80">
            <div className={"flex flex-row gap-4 items-center flex-1 min-w-0"}>
                {/* Icon Container */}
                <div className="relative w-14 h-14 flex items-center justify-center rounded-xl transition-all duration-300 flex-shrink-0 bg-white/5 border border-white/10 group-hover:bg-white/10 group-hover:border-white/20 shadow-md">
                    <div className="relative w-10 h-10">
                        <Image
                            className="w-full h-full object-contain transition-transform duration-300 group-hover:scale-110"
                            layout="fill"
                            alt="Thumbnail"
                            src={thumbnailSource || fallbackThumbnail}
                            onError={() => setThumbnailSource(fallbackThumbnail)}
                        />
                    </div>
                </div>

                {/* File Info */}
                <div className="flex flex-col text-white items-start gap-1.5 flex-1 min-w-0">
                    <p className="text-sm font-medium text-white/90 break-words w-full line-clamp-2">{model.name}</p>
                    <div className="flex items-center gap-2">
                        <span className="text-xs px-2 py-0.5 rounded-full border font-medium bg-white/5 border-white/20 text-white/80">
                            {displaySize}
                        </span>
                        <span className="text-xs text-white/50">
                            {isFolder ? "Folder" : "File"}
                        </span>
                    </div>
                </div>
            </div>

            {/* Download Button / Progress */}
            {
                file.is_completed
                    ? <button
                        className="rounded-xl p-2.5 bg-white/10 hover:bg-white/20 border border-white/20
                                   transition-all duration-300 hover:scale-110 shadow-lg flex-shrink-0"
                        onClick={onDownloadClick}
                    >
                        <ArrowDown className="w-5 h-5 text-white" />
                    </button>
                    : <div className="flex-shrink-0">
                        <CircleProgress progress={file.completion} size={40} />
                    </div>
            }
        </div>
    );
}

function MediaView(props: {
    id: string,
    isCloud: boolean
}) {
    const { id, isCloud } = props;
    const media = core.useReceiveResource(id, isCloud);

    const model: SelectedResourceViewModel | undefined = media?.model;
    const isVideo = media instanceof VideoReceiveResourceViewModel;
    const isMobile = useIsMobile();
    const [thumbnailSource, setThumbnailSource] = useState<string | undefined>();

    useEffect(() => {
        if (model?.thumbnail_path) {
            core.getDownloadUrl(model.thumbnail_path).then(setThumbnailSource)
        }
    }, [model?.thumbnail_path]);

    const onDownloadClick = useCallback(() => {
        if (!model) return
        core.downloadFile(model.path, model.name)
    }, [model?.path, model?.name])

    if (!media || !model) return null;

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <div
            className={clsx(
                "w-full overflow-hidden rounded-2xl relative group",
                "border border-white/10 backdrop-blur-sm",
                "transition-all duration-300 ease-out",
                "hover:scale-[1.02] hover:shadow-lg hover:shadow-muted/20 hover:border-white/30",
                isMobile ? "flex flex-row items-center gap-3 p-3 h-auto bg-muted/60 backdrop-blur-xl" : "h-full"
            )}>
            {/* Desktop: Thumbnail - full background */}
            {!isMobile && (
                <>
                    <div className="absolute inset-0 z-0">
                        {thumbnailSource ? (
                            <Image
                                className="object-cover w-full h-full"
                                alt={model.name}
                                src={thumbnailSource}
                                fill
                                sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
                            />
                        ) : (
                            <div className="w-full h-full bg-muted/40 flex items-center justify-center">
                                <Image
                                    className="w-16 h-16 opacity-50"
                                    width={64}
                                    height={64}
                                    alt="placeholder"
                                    src="/file.svg"
                                />
                            </div>
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
                    <div className="absolute inset-0 bg-black/40 backdrop-blur-sm z-15 opacity-0 group-hover:opacity-100 transition-all duration-300" />
                </>
            )}

            {/* Mobile: Thumbnail - small square */}
            {isMobile && (
                <div className="w-16 h-16 shrink-0 rounded-xl overflow-hidden relative bg-muted/20">
                    {thumbnailSource ? (
                        <Image className="w-full h-full object-cover" fill src={thumbnailSource} alt={model.name} />
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

            {/* Desktop: Download Button / Progress - centered */}
            {!isMobile && (
                <div className="absolute bottom-0 left-0 right-0 p-3 z-20 bg-gradient-to-t from-black/60 to-transparent backdrop-blur-sm">
                    <div className="flex items-center justify-between gap-3">
                        <div className="flex flex-col gap-1.5 flex-1 min-w-0">
                            <p className="text-white text-sm font-medium line-clamp-2 leading-tight">
                                {model.name}
                            </p>
                            <div className="flex items-center gap-2">
                                <span className="text-xs px-2 py-0.5 rounded-full border font-medium bg-white/5 border-white/20 text-white/80">
                                    {displaySize}
                                </span>
                                <span className="text-xs text-white/60">
                                    {isVideo ? "Video" : "Image"}
                                </span>
                            </div>
                        </div>

                        {/* Download Button / Progress */}
                        <div className="flex-shrink-0">
                            {media.is_completed
                                ? <button
                                    className="rounded-xl p-2.5 bg-white/10 hover:bg-white/20 border border-white/20
                                               transition-all duration-300 hover:scale-110 shadow-lg"
                                    onClick={onDownloadClick}>
                                    <ArrowDown className="w-5 h-5 text-white" />
                                </button>
                                : <CircleProgress progress={media.completion} size={36} />
                            }
                        </div>
                    </div>
                </div>
            )}

            {/* Mobile: File info */}
            {isMobile && (
                <div className="flex flex-col flex-1 min-w-0">
                    <p className="text-white text-sm font-medium leading-tight line-clamp-1 text-left">
                        {model.name}
                    </p>
                    <div className="flex items-center gap-2 mt-1">
                        <span className="text-xs px-2 py-0.5 rounded-full border font-medium bg-white/5 border-white/20 text-white/80">
                            {displaySize}
                        </span>
                        <span className="text-xs text-white/60">
                            {isVideo ? "Video" : "Image"}
                        </span>
                    </div>
                </div>
            )}

            {/* Mobile: Actions */}
            {isMobile && (
                <div className="flex-shrink-0">
                    {media.is_completed ? (
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
                                    onClick={onDownloadClick}
                                >
                                    <ArrowDown className="w-4 h-4 mr-2" />
                                    <span>Download</span>
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    ) : (
                        <CircleProgress progress={media.completion} size={32} />
                    )}
                </div>
            )}
        </div>
    );
}
