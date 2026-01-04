'use client'
import * as React from "react";
import {
    AppEventVariantTransfer,
    FileReceiveResourceViewModel,
    ImageReceiveResourceViewModel, MessageReasonVariantFailedToFindPublicSession,
    MessageReasonVariantFailedToLoadSession,
    ReceiveSessionViewModel,
    ResourceTypeVariantFolder,
    SelectedResourceViewModel,
    TransferEventVariantFindSession,
    TransferEventVariantViewSession,
    TransferEventVariantRequestDownloadResource,
    TransferEventVariantRequestDownloadAllResources,
    TransferTypeVariantReceive,TransferEventVariantCancelResourceTransfer,
    VideoReceiveResourceViewModel
} from 'shared_types/types/shared_types'
import {
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
import {ReactElement, useCallback, useEffect, useRef, useState} from "react";
import Image from "next/image";
import { useIsMobile } from "@/hooks/use-mobile";
import { Input } from "@/components/ui/input";
import CircleProgress from "@/components/ui/progress";
import core from "@/wasm/wasm_core";
import { Avatar, AvatarImage } from "@/components/ui/avatar";
import DownloadButtonWithProgress from "./download-button-with-progress";
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
        <div className="rounded-xl border-1 overflow-hidden min-h-[450px] max-h-[70vh] sm:max-h-[80vh] h-[950px]">
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
                        <div className="flex items-center gap-2 px-2 w-full">
                            <SidebarTrigger className="-ml-1" />
                            <Separator orientation="vertical" className="mr-2 h-4" />
                            <HeaderInfo />
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

function HeaderInfo() {
    const selectedSessionId = core.useSelectedSession()?.id;
    const selectedSession = core.useSession(selectedSessionId ?? '');

    const onDownloadAll = useCallback(() => {
        if (!selectedSession) return;
        const peerId = selectedSession.sender_id;
        const sessionOrderId = BigInt(selectedSession.id);
        core.update(new AppEventVariantTransfer(
            new TransferEventVariantRequestDownloadAllResources(peerId, sessionOrderId)
        ));
    }, [selectedSession]);

    const onCancelClicked = useCallback(() => {
        if (!selectedSession) return
        const resourceId = selectedSession.download_all_resource_id
        core.update(new AppEventVariantTransfer(new TransferEventVariantCancelResourceTransfer(
            BigInt(selectedSession.id), new TransferTypeVariantReceive(), resourceId
        )))
    }, [selectedSession])

    if (!selectedSession || selectedSession.is_loading) {
        return null;
    }

    const progress = (selectedSession as ReceiveSessionViewModel).progress || 0
    const displaySpeed = (selectedSession as ReceiveSessionViewModel).display_download_speed || ''
    const isCompleted = (selectedSession as ReceiveSessionViewModel).is_completed || false
    const downloadAllEnabled = (selectedSession as ReceiveSessionViewModel).download_all_enabled || false
    const downloadAllProgress = (selectedSession as ReceiveSessionViewModel).download_all_progress ?? 0
    const downloadAllInProgress = (selectedSession as ReceiveSessionViewModel).download_all_in_progress || false
    const downloadAllCompleted = (selectedSession as ReceiveSessionViewModel).download_all_completed || false

    const totalResources = (selectedSession.image_resources?.length || 0) +
        (selectedSession.video_resources?.length || 0) +
        (selectedSession.file_resources?.length || 0);

    return (
        <div className="flex items-center gap-3 flex-1">
            <div className="flex items-center gap-2">
                <span className="text-sm text-muted-foreground">
                    {totalResources} {totalResources === 1 ? 'item' : 'items'}
                </span>
            </div>
            {!isCompleted && progress > 0 && (
                <>
                    <Separator orientation="vertical" className="h-4" />
                    <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-foreground">
                            {(progress * 100).toFixed(0)}%
                        </span>
                        {displaySpeed && (
                            <span className="text-sm text-muted-foreground">
                                {displaySpeed}
                            </span>
                        )}
                    </div>
                </>
            )}
            {downloadAllEnabled && (
                <div className="ml-auto">
                    <DownloadButtonWithProgress
                        progress={downloadAllProgress}
                        isReady={true}
                        isCompleted={downloadAllCompleted}
                        isInProgress={downloadAllInProgress}
                        onDownloadClick={onDownloadAll}
                        onCancelClick={onCancelClicked}
                        size={40}
                        strokeWidth={4}
                        buttonText="Download All"
                        buttonVariant="outline"
                        buttonSize="sm"
                    />
                </div>
            )}
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
    const isCloud = selectedSession?.is_cloud || false
    const coreReady = core.useCoreReady()
    const [url, setUrl] = useUrlState(['session'])
    const isLoading = selectedSession?.is_loading
    const loadMessage = core.useMessage(new MessageReasonVariantFailedToLoadSession(BigInt(selectedSessionId ?? '0')));
    const [enteredPassword, setEnteredPassword] = useState<string>(selectedSession?.password ?? '')
    const isMobile = useIsMobile();
    const {setOpenMobile} = useSidebar();

    useEffect(() => {
        if (selectedSession?.alias) {
            setUrl({
                session: selectedSession.alias ?? ''
            })
        }
    }, [selectedSession?.id])

    useEffect(() => {
        if (url.session && coreReady) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantFindSession(url.session)))
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
        if (!selectedSession?.password_required && selectedSession?.is_loading) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession(
                null,
                BigInt(selectedSession.id),
                new TransferTypeVariantReceive(),
            )))
        }
    }, [selectedSession?.id]);

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

    if (isLoading) {
        if (selectedSession.password_required && !selectedSession.password) {
            return <div className={"text-foreground w-full h-full flex flex-col justify-center items-center gap-2"}>
                <div className={"w-[50%] flex flex-col gap-4"}>
                    <p className={"text-muted-foreground flex flex-row items-center"}>
                        <Image alt={"lock"} width={10} height={10}
                               className={"w-7 text-white bg-muted p-1.5 rounded-lg mr-2 h-7"} src={"/lock.svg"}
                               color={'white'}/>
                        This session is password protected</p>
                    <input type="password" name="fake-password" style={{display: 'none'}}/>
                    <div className="flex flex-col gap-2">
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
                        {selectedSession.error_message && (
                            <p className="text-red-500 text-sm">{selectedSession.error_message}</p>
                        )}
                    </div>
                    <Button onClick={onSelected} className={"w-fit bg-foreground"}>Continue</Button>
                </div>
            </div>
        }

        return <div className={"w-full h-full flex flex-col justify-center items-center gap-4"}>
            <LoaderCircle className={"animate-spin"}/>

            {selectedSession.loading_status && (
                <p className="text-muted-foreground">{selectedSession.loading_status}</p>
            )}

            {selectedSession.error_message && (
                <div className="flex flex-col gap-2 items-center">
                    <p className="text-red-500">{selectedSession.error_message}</p>
                    <Button
                        onClick={() => {
                            core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession(
                                selectedSession.password,
                                BigInt(selectedSession.id),
                                new TransferTypeVariantReceive()
                            )))
                        }}
                        className="bg-bluePrimary text-white hover:bg-bluePrimary/90"
                    >
                        Retry
                    </Button>
                </div>
            )}

            {loadMessage.message && !selectedSession.error_message && (
                <p className="text-red-500">{loadMessage.message}</p>
            )}
        </div>
    }

    if (!selectedSession) {
        return <div className={"w-full h-full flex justify-center items-center gap-2"}>
            <p>No session selected</p>
        </div>
    }

    return (
        <div className="w-full h-full flex flex-col gap-4 pb-4">
            <Collapsible
                className={`w-full ${selectedSession?.image_resources.length ? 'visible' : 'hidden'}`}>
                <ReceiveCategory
                    title={`${selectedSession?.image_resources.length} Image${selectedSession?.image_resources.length !== 1 ? 's' : ''}`}/>
                <CollapsibleContent className={"space-y-2"}>
                    <div className="flex flex-col md:grid md:grid-cols-2 lg:grid-cols-3 gap-4 pb-8">
                        {
                            selectedSession?.image_resources.map((image: ImageReceiveResourceViewModel, index: number) => {
                                return <ItemEffect key={image.model.order_id} index={index}>
                                    <div className={isMobile ? "h-auto" : "h-[200px]"}>
                                        <MediaView id={image.model.order_id} isCloud={selectedSession.is_cloud} sessionId={selectedSession.id}/>
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
                    <div className="flex flex-col md:grid md:grid-cols-3 gap-4 pb-8">
                        {
                            selectedSession?.video_resources.map((video: VideoReceiveResourceViewModel, index: number) => {
                                return <ItemEffect key={video.model.order_id} index={index}>
                                    <div className={isMobile ? "h-auto" : "h-[200px]"}>
                                        <MediaView id={video.model.order_id} isCloud={isCloud} sessionId={selectedSession.id}/>
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
                                return <ItemEffect key={file.model.order_id} index={index}>
                                    <div className={"h-fit"}>
                                        <FileView key={file.model.order_id} id={file.model.order_id} isCloud={isCloud} sessionId={selectedSession.id}/>
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
    const selectedId = useRef<string | null>(null)
    const p2pSessions = core.useNearbySessionsList()

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
        core.update(new AppEventVariantTransfer(new TransferEventVariantFindSession(searchTerms || '')))
    }, [keywords, message, setUrl])

    useEffect(() => {
        const all = [...publicSessions, ...p2pSessions]
        if (all.length === 1) {
            const selected = all[0]
            if (selectedId.current === selected.id) {
                return
            }

            selectedId.current = selected.id
            core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession(null, BigInt(selected.id), new TransferTypeVariantReceive())))
        }
    }, [publicSessions?.length, p2pSessions?.length]);

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

const SessionItemsList = ({ sessions }: { sessions: (ReceiveSessionViewModel)[] }) => {
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
                                    core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession(
                                        item.password,
                                        BigInt(item.id),
                                        new TransferTypeVariantReceive()
                                    )))
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
        <SessionListWrapper title="P2P">
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

    const is_public = session.is_cloud;
    const is_scope_online = !is_public && (session as ReceiveSessionViewModel).is_scope_online;

    const name = session.sender_name
    const avatar_url = session.sender_avatar
    const is_required_password = session.password_required;

    const progress = is_public
        ? (session as ReceiveSessionViewModel).progress || 0
        : 0;
    const is_completed = is_public
        ? (session as ReceiveSessionViewModel).is_completed || false
        : false;
    const is_in_progress = is_public
        ? (session as ReceiveSessionViewModel).is_in_progress || false
        : false;

    return <>
        <button
            onClick={onPress}
            className={"w-full bg-muted/50 rounded-xl p-2.5 h-fit border border-white/10 hover:bg-muted hover:border-white/20 hover:cursor-pointer"}>
            <div className={"flex flex-row items-start gap-3"}>
                {/* Avatar Section */}
                <div className={"relative flex-shrink-0"}>
                    <div className={"bg-gradient-to-br from-bluePrimary to-bluePrimary/80 rounded-xl p-1 w-11 h-11"}>
                        <Avatar className={"w-full h-full"}>
                            <AvatarImage src={avatar_url} className={"rounded-lg"} />
                        </Avatar>
                    </div>
                    {is_public ? (
                        <div className={"absolute -bottom-1 -right-1 bg-bluePrimary rounded-full p-1 border-2 border-muted"}>
                            <Globe className={"w-3 h-3 text-white"} />
                        </div>
                    ) : (
                        <div className={"absolute -bottom-1 -right-1 bg-bluePrimary rounded-full p-1 border-2 border-muted"}>
                            <Wifi className={"w-3 h-3 text-white"} />
                        </div>
                    )}
                </div>

                {/* Content Section */}
                <div className={"flex justify-start flex-col gap-0.5 flex-1 min-w-0"}>
                    {/* Alias tag and password */}
                    <div className={"flex flex-row items-center gap-1.5"}>
                        <span className={`px-1 py-0.3 rounded-sm text-xs font-medium border w-fit ${
                            is_public
                                ? 'bg-blue-500/20 text-blue-400 border-blue-500/30'
                                : is_scope_online
                                    ? 'bg-green-500/20 text-green-400 border-green-500/30'
                                    : 'bg-gray-500/20 text-gray-400 border-gray-500/30'
                        }`}>
                            {session.alias}
                        </span>
                        {is_required_password && (
                            <div className={"bg-muted-foreground/20 rounded-md p-0.5 border border-white/10"}>
                                <Image
                                    alt={"lock"}
                                    width={10}
                                    height={10}
                                    className={"w-2.5 h-2.5"}
                                    src={"/lock.svg"}
                                />
                            </div>
                        )}
                    </div>

                    {/* Name */}
                    <p className={"text-start text-sm text-primaryText font-semibold"}>{name}</p>

                    {/* Description */}
                    {session.sender_description && (
                        <p className={"text-start text-[11px] text-primaryText/60 line-clamp-1"}>{session.sender_description}</p>
                    )}
                </div>

                {/* Right Section - Progress */}
                <div className={"flex flex-col items-end gap-1 flex-shrink-0"}>
                    <CircleProgress
                        isCompleted={is_completed}
                        isInProgress={is_in_progress}
                        progress={progress}
                        size={28}
                        strokeWidth={3}
                    />
                </div>
            </div>
        </button>
    </>
}

function FileView(props: {
    id: string,
    isCloud: boolean,
    sessionId: string
}) {
    const { id, isCloud, sessionId } = props;
    const file = core.useReceiveResource(id, isCloud);
    const session = core.useSession(sessionId);
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
        if (!model || !session) return

        if (!isCloud) {
            const peerId = session.sender_id;
            const sessionOrderId = BigInt(sessionId);
            const resourceOrderId = BigInt(id);

            core.update(new AppEventVariantTransfer(
                new TransferEventVariantRequestDownloadResource(
                    peerId,
                    sessionOrderId,
                    resourceOrderId
                )
            ));
        } else {
            core.downloadFile(model.path, model.name)
        }
    }, [model?.path, model?.name, isCloud, session, sessionId, id])

    const onCancelClick = useCallback(() => {
        if (!model || !session) return

        core.update(new AppEventVariantTransfer(
            new TransferEventVariantCancelResourceTransfer(
                BigInt(sessionId),
                new TransferTypeVariantReceive(),
                BigInt(id)
            )
        ));
    }, [model, session, sessionId, id])

    if (!file || !model) return null;

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <div className="w-full flex items-center gap-3 p-3 rounded-lg border border-border bg-card hover:bg-accent/50 transition-colors">
            {/* Thumbnail */}
            <div className="w-10 h-10 shrink-0 flex items-center justify-center rounded-md bg-muted">
                <img
                    className="w-6 h-6 object-contain opacity-70"
                    alt={model.name}
                    src={thumbnailSource || fallbackThumbnail}
                    onError={() => setThumbnailSource(fallbackThumbnail)}
                />
            </div>

            {/* File Info */}
            <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate text-foreground">
                    {model.name}
                </p>
                <div className="flex items-center gap-2 mt-0.5">
                    <p className="text-xs text-muted-foreground">
                        {displaySize}
                    </p>
                    <span className="text-xs text-muted-foreground/60">•</span>
                    <p className="text-xs text-muted-foreground">
                        {isFolder ? "Folder" : "File"}
                    </p>
                </div>
            </div>

            {/* Download Button / Progress */}
            <div className="shrink-0">
                <DownloadButtonWithProgress
                    progress={file.completion}
                    isReady={file.is_ready}
                    isCompleted={file.is_completed}
                    isInProgress={!file.is_completed && file.completion > 0}
                    onDownloadClick={onDownloadClick}
                    onCancelClick={onCancelClick}
                    size={40}
                    strokeWidth={4}
                />
            </div>
        </div>
    );
}

function MediaView(props: {
    id: string,
    isCloud: boolean,
    sessionId: string
}) {
    const { id, isCloud, sessionId } = props;
    const media = core.useReceiveResource(id, isCloud);
    const session = core.useSession(sessionId);

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
        if (!model || !session) return

        if (!isCloud) {
            const peerId = session.sender_id;
            const sessionOrderId = BigInt(sessionId);
            const resourceOrderId = BigInt(id);

            core.update(new AppEventVariantTransfer(
                new TransferEventVariantRequestDownloadResource(
                    peerId,
                    sessionOrderId,
                    resourceOrderId
                )
            ));
        } else {
            core.downloadFile(model.path, model.name)
        }
    }, [model?.path, model?.name, isCloud, session, sessionId, id])

    const onCancelClick = useCallback(() => {
        if (!model || !session) return

        core.update(new AppEventVariantTransfer(
            new TransferEventVariantCancelResourceTransfer(
                BigInt(sessionId),
                new TransferTypeVariantReceive(),
                BigInt(id)
            )
        ));
    }, [model, session, sessionId, id])

    if (!media || !model) return null;

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    if (isMobile) {
        return (
            <div className="w-full flex items-center gap-3 p-3 rounded-lg border border-border bg-card hover:bg-accent/50 transition-colors">
                {/* Thumbnail */}
                <div className="w-10 h-10 shrink-0 rounded-md overflow-hidden bg-muted relative">
                    {thumbnailSource ? (
                        <img className="w-full h-full object-cover" src={thumbnailSource} alt={model.name} />
                    ) : (
                        <div className="w-full h-full flex items-center justify-center">
                            <ImageUpIcon className="w-5 h-5 opacity-40" />
                        </div>
                    )}
                    {isVideo && (
                        <div className="absolute inset-0 flex items-center justify-center bg-black/30">
                            <Play className="w-3 h-3 text-white fill-white" />
                        </div>
                    )}
                </div>

                {/* File info */}
                <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate text-foreground">
                        {model.name}
                    </p>
                    <div className="flex items-center gap-2 mt-0.5">
                        <p className="text-xs text-muted-foreground">
                            {displaySize}
                        </p>
                        <span className="text-xs text-muted-foreground/60">•</span>
                        <p className="text-xs text-muted-foreground">
                            {isVideo ? "Video" : "Image"}
                        </p>
                    </div>
                </div>

                {/* Actions */}
                <div className="shrink-0">
                    <DownloadButtonWithProgress
                        progress={media.completion}
                        isReady={media.is_ready}
                        isCompleted={media.is_completed}
                        isInProgress={media.completion > 0 && !media.is_completed}
                        onDownloadClick={onDownloadClick}
                        onCancelClick={onCancelClick}
                        size={32}
                        strokeWidth={3}
                    />
                </div>
            </div>
        );
    }

    return (
        <div className="w-full h-full flex flex-col rounded-lg border border-border bg-card overflow-hidden group hover:border-foreground/20 transition-colors">
            {/* Thumbnail */}
            <div className="relative bg-muted/30 h-[calc(100%-76px)]">
                {thumbnailSource ? (
                    <img
                        className="w-full h-full object-cover"
                        alt={model.name}
                        src={thumbnailSource}
                    />
                ) : (
                    <div className="w-full h-full flex items-center justify-center">
                        <ImageUpIcon className="w-12 h-12 opacity-20" />
                    </div>
                )}

                {/* Video play icon */}
                {isVideo && (
                    <div className="absolute top-2 right-2 bg-black/60 rounded-full p-1.5">
                        <Play className="w-3 h-3 text-white fill-white" />
                    </div>
                )}
            </div>

            {/* File info */}
            <div className="p-3 border-t border-border flex items-center gap-3 h-[76px] flex-shrink-0">
                <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate text-foreground mb-1">
                        {model.name}
                    </p>
                    <div className="flex items-center gap-2">
                        <p className="text-xs text-muted-foreground">
                            {displaySize}
                        </p>
                        <span className="text-xs text-muted-foreground/60">•</span>
                        <p className="text-xs text-muted-foreground">
                            {isVideo ? "Video" : "Image"}
                        </p>
                    </div>
                </div>

                {/* Download Button / Progress */}
                <div className="shrink-0">
                    <DownloadButtonWithProgress
                        progress={media.completion}
                        isReady={media.is_ready}
                        isCompleted={media.is_completed}
                        isInProgress={media.completion > 0 && !media.is_completed}
                        onDownloadClick={onDownloadClick}
                        onCancelClick={onCancelClick}
                        size={36}
                        strokeWidth={3}
                    />
                </div>
            </div>
        </div>
    );
}
