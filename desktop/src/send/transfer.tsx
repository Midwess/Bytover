import {
    Tabs,
    TabsPanel,
    TabsPanels,
    TabsList,
    TabsTab,
} from '@/components/animate-ui/components/base/tabs'
import {Button} from "@/components/ui/button"
import {
    Card,
    CardContent,
} from "@/components/ui/card"
import {TransferPasswordField} from "@/send/transfer-password-field"
import {
    Copy,
    Check,
    Zap,
    Globe,
    Mail,
    ChevronRight,
    Upload,
} from "lucide-react"
import core from "@/core.ts"
import {Avatar, AvatarImage} from "@/components/ui/avatar"
import {invoke} from "@tauri-apps/api/core"
import {noop} from "motion"
import {Slide} from "@/components/animate-ui/primitives/effects/slide.tsx"
import {useEffect, useState} from "react"
import {Progress} from "@/components/animate-ui/components/radix/progress"
import {ProgressIndicator} from "@/components/animate-ui/primitives/radix/progress"
import {UnlimitedLineText} from "@/components/ui/unlimited-line-text"
import {EmailTransfer} from "@/send/email-transfer"
import {formatUpdateLabel, openForceUpdate} from "@/components/force-update-overlay"
import {UpdateStatus} from "@/lib/updater"

export function Transfer({ shelfId, forceUpdate }: { shelfId: string | undefined; forceUpdate: UpdateStatus | null }) {
    const isUpdateBlocked = forceUpdate?.is_critical === true
    return (
        <div className="flex w-[208px] flex-col gap-6 h-full overflow-hidden">
            <Slide
                delay={240}
                direction={"left"}
                offset={380}
                className="h-full flex">
                <Tabs defaultValue="quick" className="items-start flex flex-col h-full">
                    <TabsList className={"ml-2 rounded-xl border-2 shadow-background shadow-sm"}>
                        <TabsTab value="quick" className="w-[64px] px-1"><Zap/> Quick</TabsTab>
                        <TabsTab value="public" className="w-[64px] px-1"><Globe/> Cloud</TabsTab>
                        <TabsTab value="email" className="w-[64px] px-1"><Mail/> Email</TabsTab>
                    </TabsList>
                    <div
                        className="pl-2 border-none bg-transparent relative w-full flex-1 flex flex-col min-h-0 overflow-hidden">
                        <TabsPanels className="flex-1 flex flex-col min-h-0 overflow-hidden">
                            <TabsPanel value="quick" className="flex flex-col h-full overflow-hidden">
                                <CardContent className={"p-0 flex flex-col gap-1.5 h-full overflow-hidden"}>
                                    <P2PSend shelfId={shelfId} forceUpdate={isUpdateBlocked ? forceUpdate : null} />
                                </CardContent>
                            </TabsPanel>
                            <TabsPanel value="public" className="flex flex-col gap-2">
                                <CardContent className={"p-0 flex flex-col gap-1.5"}>
                                    <PublicTransfer shelfId={shelfId} forceUpdate={isUpdateBlocked ? forceUpdate : null} />
                                </CardContent>
                            </TabsPanel>
                            <TabsPanel value="email" className="flex flex-col gap-2 w-[190px]">
                                <CardContent className={"p-0 flex flex-col gap-1.5"}>
                                    <EmailTransfer shelfId={shelfId} forceUpdate={isUpdateBlocked ? forceUpdate : null} />
                                </CardContent>
                            </TabsPanel>
                        </TabsPanels>
                    </div>
                </Tabs>
            </Slide>
        </div>
    );
}

function P2PSend({ shelfId, forceUpdate }: { shelfId: string | undefined; forceUpdate: UpdateStatus | null }) {
    const p2pSession = core.useP2PSessionForShelf(shelfId)
    const selectedResources = core.useSelectedResourcesForShelf(shelfId)
    const [password, setPassword] = useState(p2pSession?.password || '')
    const [isLoading, setIsLoading] = useState(false)
    const isInProgress = p2pSession?.is_in_progress ?? false

    useEffect(() => {
        if (p2pSession?.password) {
            setPassword(p2pSession.password)
        }
    }, [p2pSession?.password])

    useEffect(() => {
        if (p2pSession?.access_url) {
            setIsLoading(false)
        }
    }, [p2pSession?.access_url]);

    const handleStartTransfer = () => {
        if (!shelfId || isLoading) return
        if (selectedResources.length > 0) {
            setIsLoading(true)
            setTimeout(() => setIsLoading(false), 6000)
        }
        const pwd = password || null
        invoke("p2p_transfer", { shelfId, password: pwd }).then(noop)
        setPassword('')
    }

    const handleStopTransfer = () => {
        if (p2pSession?.session_id) {
            invoke("cancel_send", {sessionId: p2pSession.session_id}).then(noop)
        }
    }

    return <div className={"flex flex-col items-start w-full gap-2"}>
        <Card shadowSize={0.5} className="flex flex-col px-2 py-1 justify-center items-center w-full">
            <MyPeerInfo/>
        </Card>
        <Card shadowSize={0.5} className="flex flex-col p-1 w-full">
            <TransferPasswordField
                className={"bg-secondary shadow-background h-9"}
                value={password}
                onChange={setPassword}
                maxLength={20}
                disabled={isInProgress}
            />
        </Card>
        {
            p2pSession?.access_url &&
            <Card shadowSize={0.5} className="flex flex-col p-1 bg-card/95 w-full">
                <UrlInputWithCopy url={p2pSession?.access_url ?? ''}/>
            </Card>
        }
        <Card shadowSize={0.5} className={`flex flex-row gap-2 p-1 items-center ${forceUpdate ? "w-auto" : "w-[100px]"}`}>
            {
                forceUpdate ? (
                    <Button onClick={() => openForceUpdate(forceUpdate)}
                            className={"bg-bluePrimary text-foreground shadow-lg hover:bg-bluePrimary/60 px-3 w-auto"}>
                        {formatUpdateLabel(forceUpdate)}
                    </Button>
                ) : isInProgress ? (
                    <Button onClick={handleStopTransfer}
                            className={"bg-muted-foreground/30 text-primary h-full shadow-lg w-full"}>Cancel</Button>
                ) : (
                    <Button onClick={handleStartTransfer}
                            disabled={isLoading}
                            className={"bg-bluePrimary text-foreground shadow-lg hover:bg-bluePrimary/60 w-full disabled:opacity-70"}>
                        {isLoading ? (
                            <div className="h-4 w-4 animate-spin rounded-full border-2 border-white/20 border-t-white"></div>
                        ) : (
                            <>Start <ChevronRight className={"scale-x-120"}/></>
                        )}
                    </Button>
                )
            }
        </Card>
    </div>
}

function MyPeerInfo() {
    const myPeer = core.useMyPeer()

    if (!myPeer) {
        return (
            <div className="w-full mb-2">
                <div className="relative overflow-hidden rounded-2xl backdrop-blur-sm">
                    <div className="flex items-center justify-center gap-3 py-2">
                        <div
                            className="h-4 w-4 animate-spin rounded-full border-1 border-white/20 border-t-white"></div>
                        <span className="text-sm font-medium text-muted-foreground animate-pulse">Initializing...</span>
                    </div>
                </div>
            </div>
        )
    }

    const color = `rgb(${myPeer.avatar.dominant_color_r}, ${myPeer.avatar.dominant_color_g}, ${myPeer.avatar.dominant_color_b})`

    return (
        <div className="flex flex-col w-full items-center gap-2">
            <div className="flex flex-row rounded-2xl items-center w-full">
                <div className="flex flex-row items-center gap-3 justify-between flex-1 rounded-xl">
                    <div className="flex flex-col gap-[0.5] items-start justify-center h-full">
                        <p className="text-primaryText font-bold text-sm">{myPeer.display_name}</p>
                    </div>
                    <div
                        className="relative aspect-square justify-center items-center text-primaryText flex h-[38px] w-[38px] border-greenSecondary p-3 border-1 rounded-2xl">
                        <Avatar className="p-1 rounded-xl" style={{backgroundColor: color}}>
                            <AvatarImage src={myPeer.avatar.url}/>
                        </Avatar>
                        {/* Online status indicator */}
                        <div
                            className="absolute -bottom-0.5 -right-0.5 w-3 h-3 bg-greenSecondary rounded-full border-1 border-background"/>
                    </div>
                </div>
            </div>
        </div>
    )
}

function PublicTransfer({ shelfId, forceUpdate }: { shelfId: string | undefined; forceUpdate: UpdateStatus | null }) {
    const [pwd, setPwd] = useState("");
    const [isLoading, setIsLoading] = useState(false)
    const selectedResources = core.useSelectedResourcesForShelf(shelfId)
    const cloudSession = core.useCloudSessionForShelf(shelfId)
    const progress = (cloudSession?.progress ?? 0) * 100

    const handleUpload = () => {
        if (!shelfId || isLoading) return
        if (selectedResources.length > 0) {
            setIsLoading(true)
            const delay = Math.random() * 2000 + 2000
            setTimeout(() => setIsLoading(false), delay)
        }
        invoke("public_transfer", { shelfId, password: pwd }).then(noop)
    }

    return <>
        <Card shadowSize={0.5} className="flex flex-col gap-2 p-2 rounded-xl">
            <p className="text-xs text-muted-foreground">
                Create a sharable link. Files are stored for 7 days.
            </p>
        </Card>
        <Card shadowSize={0.5} className="flex flex-col p-1 w-full">
            <TransferPasswordField
                className={"h-9 bg-secondary shadow-background"}
                value={pwd}
                onChange={setPwd}
            />
        </Card>
        {
            cloudSession?.access_url &&
            <Card shadowSize={0} className="flex flex-col p-1">
                <UrlInputWithCopy url={cloudSession?.access_url ?? ''}/>
            </Card>
        }
        <Card shadowSize={0.5} className={`flex flex-row gap-2 p-1 items-center ${cloudSession?.progress ? "w-full" : "w-fit"}`}>
            {
                forceUpdate ? (
                    <Button onClick={() => openForceUpdate(forceUpdate)}
                            className={"bg-bluePrimary text-foreground shadow-lg hover:bg-bluePrimary/60 px-3 w-auto"}>
                        {formatUpdateLabel(forceUpdate)}
                    </Button>
                ) : cloudSession?.is_in_progress ? (
                    <Button onClick={() => {
                        invoke("cancel_send", {sessionId: cloudSession?.session_id}).then(noop)
                    }} className={"bg-muted-foreground/30 text-primary w-[100px] h-full shadow-lg"}>Cancel</Button>
                ) : cloudSession?.is_completed ? (
                    <Button onClick={() => {
                        invoke("cancel_send", {sessionId: cloudSession?.session_id}).then(noop)
                    }}
                            className={"bg-greenSecondary/40 text-primary flex-2/5 shadow-lg hover:bg-greenSecondary/50"}>Continue</Button>
                ) : (
                    <Button onClick={handleUpload}
                            disabled={isLoading}
                            className={"bg-bluePrimary text-foreground w-[100px] shadow-lg hover:bg-bluePrimary/60 disabled:opacity-70"}>
                        {isLoading ? (
                            <div className="h-4 w-4 animate-spin rounded-full border-2 border-white/20 border-t-white"></div>
                        ) : (
                            <>Upload <Upload/></>
                        )}
                    </Button>
                )
            }
            {
                !!cloudSession?.progress && (
                    <div className="flex flex-col gap-2 pb-2 flex-3/5">
                        <div className="flex items-center justify-between gap-1">
                            <span className="text-sm">
                                {cloudSession?.display_download_speed}
                            </span>
                        </div>
                        <Progress value={progress} className="w-full space-y-2">
                            <ProgressIndicator className="bg-primary rounded-full h-full w-full flex-1"/>
                        </Progress>
                    </div>
                )
            }
        </Card>
    </>
}

function UrlInputWithCopy({url}: { url: string }) {
    const [isCopied, setIsCopied] = useState(false)

    const handleCopy = async () => {
        try {
            await navigator.clipboard.writeText(url)
            setIsCopied(true)
            setTimeout(() => setIsCopied(false), 2000)
        } catch (err) {
            console.error('Failed to copy text: ', err)
        }
    }

    return (
        <div className="flex items-center w-full bg-secondary rounded-lg h-9">
            <div className="w-44 px-2">
                <UnlimitedLineText
                    text={url}
                    className="text-xs text-foreground"
                    startChars={8}
                    endChars={14}
                    speed={20}
                />
            </div>
            <button
                onClick={handleCopy}
                className="w-fit p-1.5 rounded-md hover:bg-muted transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
                title={isCopied ? "Copied!" : "Copy to clipboard"}
            >
                {isCopied ? (
                    <Check className="h-4 w-4 text-green-500"/>
                ) : (
                    <Copy className="h-4 w-4 text-muted-foreground hover:text-foreground"/>
                )}
            </button>
        </div>
    )
}