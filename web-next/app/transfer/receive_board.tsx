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
    TransferEventVariantViewSession,
    TransferEventVariantRequestSessionDetail,
    TransferTypeVariantReceive,
    VideoReceiveResourceViewModel
} from 'shared_types/types/shared_types'
import {
    ArrowDown,
    Book,
    ChevronsUpDown,
    Globe, ImageUpIcon, LoaderCircle, Play, Wifi
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleTrigger,
} from '@/components/animate-ui/radix/collapsible';
import { ReactElement, useCallback, useEffect, useState } from "react";
import Image from "next/image";
import { useIsMobile } from "@/hooks/use-mobile";
import clsx from "clsx";
import { Input } from "@/components/ui/input";
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
import { Label } from "@/components/ui/label";

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
    const isCloud = selectedSession instanceof ReceiveCloudSessionViewModel
    const coreReady = core.useCoreReady()
    const [url, setUrl] = useUrlState(['session'])
    const isLoading = selectedSession instanceof ReceiveCloudSessionViewModel
        ? (selectedSession as ReceiveCloudSessionViewModel)?.is_loading
        : false
    const loadMessage = core.useMessage(new MessageReasonVariantFailedToLoadSession(BigInt(selectedSessionId ?? '0')));
    const [enteredPassword, setEnteredPassword] = useState<string>((selectedSession as ReceiveCloudSessionViewModel)?.password ?? '')
    const [nearbyPassword, setNearbyPassword] = useState<string>('')
    const isMobile = useIsMobile();
    const { setOpenMobile } = useSidebar();

    useEffect(() => {
        if (selectedSession && selectedSession instanceof ReceiveCloudSessionViewModel) {
            if (selectedSession.alias) {
                setUrl({
                    session: selectedSession.alias ?? ''
                })
            }
        }
    }, [selectedSession?.id])

    useEffect(() => {
        if (url.session && coreReady) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantFindPublicSession(url.session)))
        }
    }, [coreReady])

    const onSelected = () => {
        if (!selectedSession) {
            return
        }

        core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession(
            enteredPassword ? enteredPassword : null, BigInt(selectedSession!.id), new TransferTypeVariantReceive()
        )))
    }

    useEffect(() => {
        if (selectedSession && selectedSession instanceof ReceiveCloudSessionViewModel) {
            const cloud = selectedSession as ReceiveCloudSessionViewModel
            if (!cloud.is_required_password && isLoading) {
                core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession(
                    null,
                    BigInt(cloud.id),
                    new TransferTypeVariantReceive(),
                )))
            }
        }
    }, [selectedSession?.id]);

    useEffect(() => {
        if (selectedSession && selectedSession instanceof ReceiveSessionViewModel) {
            const nearby = selectedSession as ReceiveSessionViewModel
            if (!nearby.password_required && !nearby.is_authenticated) {
                core.update(new AppEventVariantTransfer(
                    new TransferEventVariantRequestSessionDetail(
                        nearby.peer_id,
                        BigInt(nearby.id),
                        null
                    )
                ))
            }
        }
    }, [selectedSession?.id]);

    const requestNearbySessionDetail = () => {
        if (!(selectedSession instanceof ReceiveSessionViewModel)) return
        const nearby = selectedSession as ReceiveSessionViewModel
        core.update(new AppEventVariantTransfer(
            new TransferEventVariantRequestSessionDetail(
                nearby.peer_id,
                BigInt(nearby.id),
                nearbyPassword || null
            )
        ))
    }

    if (!selectedSession) {
        return <div className={"w-full h-full flex flex-col justify-center items-center gap-4"}>
            <p className="text-lg font-medium text-muted-foreground/50">No session selected</p>
            <Button
                onClick={() => setOpenMobile(true)}
                className="block sm:hidden bg-bluePrimary text-white hover:bg-bluePrimary/90"
            >
                Select Session
            </Button>
        </div>
    }

    if (selectedSession instanceof ReceiveCloudSessionViewModel) {
        const cloud = selectedSession as ReceiveCloudSessionViewModel
        if (cloud.is_required_password && !cloud.password && isLoading) {
            return <div className={"text-foreground w-full h-full flex flex-col justify-center items-center gap-2"}>
                <div className={"w-[50%] flex flex-col gap-4"}>
                    <p className={"text-muted-foreground flex flex-row items-center"}>
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

    if (selectedSession instanceof ReceiveSessionViewModel) {
        const nearby = selectedSession as ReceiveSessionViewModel
        if (nearby.password_required && !nearby.is_authenticated) {
            return <div className={"text-foreground w-full h-full flex flex-col justify-center items-center gap-2"}>
                <div className={"w-[50%] flex flex-col gap-4"}>
                    <p className={"text-muted-foreground flex flex-row items-center"}>
                        <Image alt={"lock"} width={10} height={10}
                            className={"w-7 text-white bg-muted p-1.5 rounded-lg mr-2 h-7"} src={"/lock.svg"}
                            color={'white'} />
                        This session is password protected</p>
                    <input type="password" name="fake-password" style={{ display: 'none' }} />
                    <Input
                        className="h-10"
                        placeholder="Enter password"
                        value={nearbyPassword}
                        onChange={(e) => setNearbyPassword(e.target.value)}
                        type="password"
                        onKeyDown={(e) => {
                            if (e.key === 'Enter') {
                                requestNearbySessionDetail()
                            }
                        }}
                    />
                    <Button onClick={requestNearbySessionDetail} className={"w-fit bg-foreground"}>Continue</Button>
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
                    <div className="flex flex-col md:grid md:grid-cols-2 lg:grid-cols-4 gap-4 pb-8">
                        {
                            selectedSession?.image_resources.map((image: ImageReceiveResourceViewModel, index: number) => {
                                return <ItemEffect key={image.model.order_id} index={index}>
                                    <div className={isMobile ? "h-auto" : "h-[200px]"}>
                                        <MediaView id={image.model.order_id} isCloud={isCloud} />
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
                                return <ItemEffect key={video.model.order_id} index={index}>
                                    <div className={isMobile ? "h-auto" : "h-[200px]"}>
                                        <MediaView id={video.model.order_id} isCloud={isCloud} />
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
    const { children } = props
    return <>{children}</>
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

function FindSessionSection() {
    const [url, setUrl] = useUrlState(['session'])
    const message = core.useMessage(new MessageReasonVariantFailedToFindPublicSession())
    const [keywords, setKeywords] = useState<string>(url.session || '')
    const publicSessions = core.useCloudSessionsList()

    useEffect(() => {
        if (url.session) {
            setKeywords(url.session)
        }
    }, [])

    const handleFind = useCallback((overrideKeywords?: string) => {
        const searchTerms = overrideKeywords !== undefined ? overrideKeywords : keywords;

        if (!searchTerms) {
            setUrl({ session: null })
        }

        message?.resolveMessage()
        console.log('finding public session with keywords: ', searchTerms || '')
        core.update(new AppEventVariantTransfer(new TransferEventVariantFindPublicSession(searchTerms || '')))
    }, [keywords, message, setUrl])

    useEffect(() => {
        if (publicSessions?.length === 1 && keywords) {
            setUrl({ session: publicSessions[0].alias })
            core.updateSelectedSession(publicSessions[0])
        }
    }, [publicSessions, keywords, setUrl])

    return (
        <div className={"flex flex-col justify-start text-primaryText gap-3"}>
            <div className={"flex flex-col gap-1 pt-2"}>
                <Label htmlFor={"session-name"} className={"pl-1 text-muted-foreground text-sm"}>Find session</Label>
                <div className={"relative"}>
                    <Input id="session-name" value={keywords || ''} className={"rounded-md pr-8 min-h-10 h-fit"}
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
                            handleFind('')
                        }}
                    >
                        ×
                    </Button>
                </div>
            </div>
            {message.message && <p className={"text-foreground text-sm"}>{message.message}</p>}
            <Button className={"w-fit h-8 text-foreground bg-bluePrimary"} onClick={() => handleFind()}>Find</Button>
        </div>
    )
}

const SessionListWrapper = ({ title, children }: { title: string, children: React.ReactNode }) => {
    return (
        <Collapsible className={"flex flex-col w-full gap-3"} defaultOpen={true}>
            <CollapsibleTrigger asChild className={"flex flex-row items-start"}>
                <Button variant="secondary"
                    className="w-full justify-between items-center text-start flex flex-row cursor-pointer rounded-lg">
                    {title}
                    <ChevronsUpDown className="h-4 w-4" />
                    <span className="sr-only">Toggle</span>
                </Button>
            </CollapsibleTrigger>
            <CollapsibleContent className={"flex flex-col gap-3"}>
                {children}
            </CollapsibleContent>
        </Collapsible>
    )
}

const SessionItemsList = ({ sessions }: { sessions: (ReceiveCloudSessionViewModel | ReceiveSessionViewModel)[] }) => {
    const { isMobile, setOpenMobile } = useSidebar();

    return (
        <>
            {sessions.length === 0 && <p className={"text-muted-foreground text-sm pl-2"}>Empty</p>}
            <div className={"flex flex-col gap-2"}>
                {
                    sessions.map((item) => {
                        return <div key={item.id}>
                            <TransferSession
                                onPress={() => {
                                    core.updateSelectedSession(item)
                                    if (isMobile) {
                                        setOpenMobile(false)
                                    }
                                }}
                                id={item.id}
                                key={item.id}
                            />
                        </div>
                    })
                }
            </div>
        </>
    )
}

const PublicSessionItems = () => {
    const publicSessions = core.useCloudSessionsList()
    return <SessionItemsList sessions={publicSessions} />
}

const PublicSessionList = () => {
    return (
        <SessionListWrapper title="Public">
            <PublicSessionItems />
        </SessionListWrapper>
    )
}

const NearbySessionItems = () => {
    const nearbySessions = core.useNearbySessionsList()
    return <SessionItemsList sessions={nearbySessions} />
}

const NearbySessionList = () => {
    return (
        <SessionListWrapper title="Nearby">
            <NearbySessionItems />
        </SessionListWrapper>
    )
}

function Board() {
    return (
        <div className="flex flex-col gap-6 h-full overflow-y-auto px-2 pb-4">
            <FindSessionSection />
            <NearbySessionList />
            <PublicSessionList />
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
            className={"w-full p-1 flex flex-row bg-muted rounded-2xl items-center px-2 py-2 h-fit max-h-[80px] border-1 border-primaryText/5 justify-between hover:bg-muted-foreground/50 hover:cursor-pointer"}>
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
            <CircleProgress isCompleted={is_completed} isInProgress={!!progress && progress < 1} progress={progress} size={30} strokeWidth={3}
                onClick={() => {
                    if (!is_public) {
                        core.update(new AppEventVariantTransfer(new TransferEventVariantCancelTransfer(BigInt(id), new TransferTypeVariantReceive())))
                    }
                }} />
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
            className="gap-4 flex flex-row w-full overflow-clip justify-between items-center h-fit rounded-2xl relative group 
                       bg-muted/60
                       backdrop-blur-xl border border-white/10 p-1.5
                       transition-all duration-300 ease-out
                       hover:shadow-xl hover:shadow-white/10 hover:border-white/30 hover:backdrop-blur-sm
                       hover:bg-muted/80">
            <div className={"flex flex-row gap-4 items-center flex-1 min-w-0"}>
                {/* Icon Container */}
                <div className="relative w-14 h-14 flex items-center justify-center rounded-xl transition-all duration-300 flex-shrink-0 bg-white/5 border border-white/10 group-hover:bg-white/10 group-hover:border-white/20 shadow-md">
                    <div className="relative w-10 h-10">
                        <img
                            className="w-full h-full object-contain transition-transform duration-300 group-hover:scale-110"
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
                        className="rounded-xl p-2 bg-white/10 hover:bg-white/20 border border-white/20
                                   transition-all duration-300 hover:scale-110 shadow-lg flex-shrink-0"
                        onClick={onDownloadClick}
                    >
                        <ArrowDown className="w-5 h-5 text-white" />
                    </button>
                    : <div className="flex-shrink-0">
                        <CircleProgress isCompleted={file.is_completed} isInProgress={!file.is_completed} progress={file.completion} size={40} strokeWidth={4} />
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
                "w-full rounded-2xl relative group overflow-clip",
                "border border-white/10 backdrop-blur-sm",
                "transition-all duration-300 ease-out",
                "hover:scale-[1.02] hover:shadow-lg hover:shadow-muted/20 hover:border-muted-foreground m-1",
                isMobile ? "flex flex-row items-center gap-3 p-1.5 h-auto bg-muted/60 backdrop-blur-xl" : "h-full"
            )}>
            {/* Desktop: Thumbnail - full background */}
            {!isMobile && (
                <>
                    <div className="absolute inset-0 z-0">
                        {thumbnailSource ? (
                            <img
                                className="object-cover w-full h-full rounded-2xl"
                                alt={model.name}
                                src={thumbnailSource}
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
                </>
            )}

            {/* Mobile: Thumbnail - small square */}
            {isMobile && (
                <div className="w-14 h-14 shrink-0 rounded-xl overflow-hidden relative bg-muted/20">
                    {thumbnailSource ? (
                        <img className="w-full h-full object-cover" src={thumbnailSource} alt={model.name} />
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
                                    className="rounded-xl p-2 bg-white/10 hover:bg-white/20 border border-white/20
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
                        <button
                            className="rounded-xl p-2.5 bg-white/10 hover:bg-white/20 border border-white/20
                                       transition-all duration-300 hover:scale-110 shadow-lg"
                            onClick={onDownloadClick}
                        >
                            <ArrowDown className="w-5 h-5 text-white" />
                        </button>
                    ) : (
                        <CircleProgress progress={media.completion} size={32} />
                    )}
                </div>
            )}
        </div>
    );
}
