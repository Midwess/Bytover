import ReactDOM from "react-dom/client"
import React, {useEffect} from "react"
import core from "@/core.ts"
import {Card} from "@/components/ui/card.tsx";
import {Avatar, AvatarImage} from "@/components/ui/avatar.tsx";
import {Button} from "@/components/ui/button.tsx";
import CircleProgress from "@/components/ui/progress.tsx";
import {Label} from "@/components/ui/label.tsx";
import {ArrowRightCircle, Inbox, Trash2, FolderIcon, FileIcon, Play, Loader, Loader2} from "lucide-react";
import {convertFileSrc, invoke} from "@tauri-apps/api/core";

import {
    ResourceTypeVariantFolder,
} from "shared_types/types/shared_types";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window/>
    </React.StrictMode>,
);

function Window() {
    useEffect(() => {
        core.launch()
    }, [])

    return (
        <main className="flex flex-col w-screen h-screen dark rounded-4xl overflow-clip no-drag p-0 relative bg-card/10">
            <div className={"w-full justify-end flex flex-row absolute z-10 pr-3 py-2 border-b border-gray-200/5 backdrop-blur-[20px] bg h-fit bg-gradient-to-b from-card/30"}>
                <div className={"w-[60%] flex flex-col gap-1.5 z-100 pl-3"}>
                    <Intro/>
                    <SessionTitle/>
                </div>
            </div>
            <div className={"h-full w-[40%] flex flex-col gap-2 absolute z-20"}>
                <Card
                    shadowSize={0.8}
                    className={"h-full bg-muted/50 backdrop-blur-xl flex flex-col border rounded-3xl gap-1.5 p-2 m-1 overflow-y-auto"}>
                    <Label className={"flex flex-row items-center gap-2 px-1 py-1 text-muted-foreground"}>
                        <Inbox size={21} className={"bg-muted-foreground/10 border rounded-md pl-[3px] pb-[2px] pr-[3px]"}/>
                        Inbox
                    </Label>
                    <SessionList/>
                </Card>
            </div>
            <div className={"flex flex-row h-full w-[60%] absolute z-0 self-end"}>
               <div
                   className={"flex-7/12 gap-1 pb-2 rounded-t-4xl h-full w-full flex flex-col shadow-background/20 pt-2 overflow-hidden text-foreground"}>
                  <div className="flex-1 min-h-0">
                       <ResourceList/>
                    </div>
                </div>
            </div>
        </main>
    )
}

function Intro() {
    return <>
        <div className={"flex flex-row items-center p-1 justify-between w-full"}>
            <div className={"flex flex-row gap-1 items-center"}>
                <div
                    className={"bg-bluePrimary rounded-full aspect-square justify-center items-center text-primaryText flex z-10"}>
                    <Avatar
                        className={"p-1 rounded-2xl h-10 w-10 bg-yellow-600/90 ring-2 ring-yellow-500/30"}>
                        <AvatarImage
                            src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                    </Avatar>
                </div>
                <div className={"flex flex-col items-start p-1 z-10"}>
                    <p className={"text-primaryText text-foreground font-thin"}>{"Ciao,"}</p>
                    <p className={"text-foreground"}>{"Tien Dang"}</p>
                </div>
            </div>
            <div className={"flex flex-row items-center gap-2"}>
                <Button onClick={() => {
                    invoke("open_shelf")
                }} className={"bg-bluePrimary text-foreground"}>Open shelf</Button>
            </div>
        </div>
    </>
}

function SessionTitle() {
    const sessionId = core.useSelectedSession()?.id
    const session = core.useSession(sessionId || BigInt(0))

    if (!session) return <></>

    return <>
        <Card
            shadowSize={0.5}
            className={"bg-muted-foreground/15 border-1 shadow-md h-10 w-full flex flex-row gap-2 items-center px-2 rounded-2xl overflow-clip justify-between text-foreground"}>
            <div className={"flex flex-row gap-2 items-center"}>
                <div className={"flex flex-col items-start p-1 z-10"}>
                    <p className={"text-primaryText"}>{
                        (session?.file_resources?.length || 0) + (session?.image_resources?.length || 0) + (session?.video_resources?.length || 0)} resources
                    </p>
                </div>
            </div>
            {
                session?.is_in_progress &&
                <div className={"flex flex-row items-center gap-3"}>
                    <p className={"text-muted-foreground"}>{session.display_download_speed}</p>
                    <CircleProgress progress={session.progress} size={22} strokeWidth={3}/>
                </div>
            }
            {
                session?.is_completed &&
                <div className={"flex flex-row items-center h-fit"}>
                    <Button onClick={() => {
                        invoke("open_session", {
                            sessionId: sessionId?.toString()
                        })
                    }} variant={"ghost"} className={"w-fit px-1"}>
                        <ArrowRightCircle className={"-rotate-45"}/>
                    </Button>
                    <Button onClick={() => {
                        invoke("delete_receive_session", {
                            sessionId: sessionId?.toString(),
                        })
                    }} variant={"ghost"} className={"text-muted-foreground px-2"}>
                        <Trash2/>
                    </Button>
                </div>
            }
        </Card>
    </>
}

function SessionList() {
    const selectedSessionId = core.useSelectedSession()?.id
    const sessions = core.useNearbySessionsList()

    return <div className={"gap-1.5 flex flex-col h-full"}>
        {
            sessions.length === 0 && <div className={"flex flex-col items-center justify-center h-[90%] text-muted-foreground"}>
                <p>Empty</p>
            </div>
        }
        {sessions.map((session) => <SessionItem sessionId={session.id} key={session.id.toString()}/>)}
    </div>
}

function SessionItem({sessionId}: { sessionId: bigint }) {
    const selectedSessionId = core.useSelectedSession()?.id
    const session = core.useSession(sessionId)
    if (!session) return null

    return <Card
        onClick={() => {
            core.selectedSession.set(session)
        }}
        shadowSize={0.0}
        className={`p-2 bg-muted-foreground/10 ${selectedSessionId === session.id && 'border-muted-foreground/50 bg-muted-foreground/30'} transition-all duration-300 flex border-1 flex-row rounded-2xl items-center gap-2.5 cursor-pointer`}>
        <Avatar className={"p-1 rounded-xl h-9 w-9 bg-yellow-600/90 ring-2 ring-yellow-500/30"}>
            <AvatarImage
                src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
        </Avatar>
        <div className={"flex flex-col gap-0.5"}>
            <p className={`${session.id === selectedSessionId && 'text-white'}`}>{session.peer_name}</p>
            {
                session.is_in_progress
                    ? <p className="text-muted-foreground animate-pulse">
                        Receiving...
                    </p>
                    : <p className={"text-muted-foreground"}>{session.display_datetime}</p>
            }
        </div>
    </Card>
}

function ResourceList() {
    const selectedSessionId = core.useSelectedSession()?.id
    const session = core.useSession(selectedSessionId || BigInt(0))
    if (!session) {
        return <div className="flex items-center justify-center w-full h-full text-muted-foreground">
            <p className="text-muted-foreground">Empty</p>
        </div>
    }

    // Combine all resources
    const allResources = [
        ...(session.file_resources || []).map(r => ({...r, resourceType: 'file' as const})),
        ...(session.image_resources || []).map(r => ({...r, resourceType: 'image' as const})),
        ...(session.video_resources || []).map(r => ({...r, resourceType: 'video' as const})),
    ]

    if (!allResources.length) {
        return <div className="flex items-center justify-center w-full h-full text-muted-foreground">
            <p className="text-muted-foreground">Empty</p>
        </div>
    }

    return <div className="w-full h-full overflow-y-auto px-2 py-2">
        <div className={"h-[90px]"}></div>
        <div className="grid grid-cols-3 h-full gap-1 auto-rows-max">
            {allResources.map((resource, index) => (
                <ResourceItem key={index} sessionId={selectedSessionId} resource={resource} />
            ))}
        </div>
    </div>
}

function ResourceItem({resource, sessionId}: {resource: any, sessionId: any}) {
    const {model, completion, is_completed, resourceType} = resource;
    
    let thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;
    const isFolder = model.type instanceof ResourceTypeVariantFolder;
    const isVideo = resourceType === 'video';
    const isImage = resourceType === 'image';
    
    // Convert absolute path to Tauri asset URL
    const thumbnailUrl = thumbnailPath ? convertFileSrc(thumbnailPath) : null;

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <Card
            onDoubleClick={() => {
                invoke("open_received_resource", {
                    resourceId: resource.id.toString(),
                    sessionId
                })
            }}
            shadowSize={0.0}
            className="border-0 bg-transparent rounded-2xl flex flex-col hover:bg-muted-foreground/10 p-1.5 relative group transition-colors">
            {/* Thumbnail */}
            <div className="w-full aspect-square rounded-xl bg-muted-foreground/40 border overflow-hidden relative mb-2">
                {thumbnailUrl ? (
                    <img 
                        src={thumbnailUrl} 
                        alt={model.name}
                        className="w-full h-full object-cover rounded-md overflow-hidden"/>
                ) : isFolder ? (
                    <div className="w-full h-full flex items-center justify-center">
                        <FolderIcon className="w-8 h-8 text-primary"/>
                    </div>
                ) : (
                    <div className="w-full h-full flex items-center justify-center">
                        <FileIcon className="w-8 h-8 text-primary"/>
                    </div>
                )}
                {isVideo && (
                    <div className="absolute top-2 right-2">
                        <Play className="w-4 h-4 text-white bg-black/50 rounded-md p-1"/>
                    </div>
                )}
                {!is_completed && (
                    <div className="absolute content-center w-full h-full top-0 left-0 flex items-center justify-center">
                        <Loader2 className="animate-spin duration-3000 text-foreground backdrop-blur-2xl rounded-full w-5 h-5" />
                    </div>
                )}
            </div>

            {/* Info */}
            <div className="flex-1 min-w-0">
                <p className="text-xs font-medium text-primaryText truncate mb-0.5">{model.name}</p>
                <p className="text-xs text-muted-foreground">{displaySize}</p>
            </div>
        </Card>
    );
}

export default Window;
