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
import {Input} from "@/components/ui/input"
import {PasswordInput} from "@/components/ui/password-input"
import {
    Copy,
    Check,
    Users,
    Globe,
} from "lucide-react"
import core from "@/core.ts"
import {Avatar, AvatarImage} from "@/components/ui/avatar"
import {invoke} from "@tauri-apps/api/core"
import {noop} from "motion"
import {Slide} from "@/components/animate-ui/primitives/effects/slide.tsx"
import {useEffect, useState} from "react"
import {Progress} from "@/components/animate-ui/components/radix/progress"
import {ProgressIndicator} from "@/components/animate-ui/primitives/radix/progress"
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from "@/components/animate-ui/primitives/animate/tooltip"

export function Transfer() {
    return (
        <div className="flex w-full flex-col gap-6 h-full overflow-hidden ml-0.5">
            <Slide
                delay={240}
                direction={"left"}
                offset={380}
                className="h-full flex">
                <Tabs defaultValue="p2p" className="w-full items-start flex flex-col h-full">
                    <TabsList className={"ml-2 border-2 shadow-background shadow-sm flex-shrink-0"}>
                        <TabsTab value="p2p"><Users/> P2P</TabsTab>
                        <TabsTab value="public"><Globe/>Link</TabsTab>
                    </TabsList>
                    <div
                        className="pl-2 border-none bg-transparent relative w-full flex-1 flex flex-col min-h-0 overflow-hidden">
                        <TabsPanels className="flex-1 flex flex-col min-h-0 overflow-hidden">
                            <TabsPanel value="p2p" className="flex flex-col h-full overflow-hidden">
                                <CardContent className={"p-0 flex flex-col gap-1.5 h-full overflow-hidden"}>
                                    <P2PSend/>
                                </CardContent>
                            </TabsPanel>
                            <TabsPanel value="public" className="flex flex-col gap-2">
                                <CardContent className={"p-0 flex flex-col gap-2"}>
                                    <PublicTransfer/>
                                </CardContent>
                            </TabsPanel>
                        </TabsPanels>
                    </div>
                </Tabs>
            </Slide>
        </div>
    );
}

function P2PSend() {
    const p2pSession = core.useP2PSession()
    const [password, setPassword] = useState(p2pSession?.password || '')
    const isInProgress = p2pSession?.is_in_progress ?? false

    useEffect(() => {
        if (p2pSession?.password) {
            setPassword(p2pSession?.password || password)
        }
    }, [p2pSession?.password, password])

    const handleStartTransfer = () => {
        const pwd = password || null
        invoke("p2p_transfer", {password: pwd}).then(noop)
        setPassword('')
    }

    const handleStopTransfer = () => {
        if (p2pSession?.session_id) {
            invoke("cancel_send", {sessionId: p2pSession.session_id}).then(noop)
        }
    }

    return <>
        <Card shadowSize={0.5} className="flex flex-col gap-3 px-2 py-1 justify-center items-center bg-card/95">
            <MyPeerInfo/>
        </Card>
        <Card shadowSize={0.5} className="flex flex-row gap-1 p-1">
            <div
                className={"flex flex-row items-center gap-1 w-fit rounded-lg"}>
                <PasswordInput
                    className={"bg-secondary shadow-background"}
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    maxLength={20}
                    placeholder="Password (Optional)"
                    disabled={isInProgress}
                />
            </div>
        </Card>
        {
            p2pSession?.access_url &&
            <Card shadowSize={0.5} className="flex flex-col gap-3 p-1 bg-card/95">
                <div
                    className={"flex flex-row items-center gap-2 w-fit rounded-lg"}>
                    <UrlInputWithCopy url={p2pSession?.access_url ?? ''}/>
                </div>
            </Card>
        }
        <Card className="flex flex-row gap-2 p-2 items-center">
            {
                isInProgress ? (
                    <Button onClick={handleStopTransfer}
                            className={"bg-muted-foreground/30 text-primary w-[70px] h-full shadow-lg"}>Cancel</Button>
                ) : (
                    <Button onClick={handleStartTransfer}
                            className={"bg-bluePrimary text-foreground w-[70px] shadow-lg hover:bg-bluePrimary/60"}>Start</Button>
                )
            }
        </Card>
    </>
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
                        <p className="text-start w-full text-primaryText/70 text-xs">
                            You're online as
                        </p>
                        <p className="text-primaryText font-bold text-sm">{myPeer.display_name}</p>
                    </div>
                    <div
                        className="relative aspect-square justify-center items-center text-primaryText flex h-[40px] w-[40px] border-greenSecondary p-3 border-1 rounded-2xl">
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

function PublicTransfer() {
    const [pwd, setPwd] = useState("");
    const cloudSession = core.useTransferState()?.cloud_session
    const progress = (cloudSession?.progress ?? 0) * 100

    return <>
        <Card shadowSize={0} className="flex flex-col gap-2 p-1">
            <div className="grid gap-3">
                <PasswordInput className={"bg-secondary shadow-background"}
                       value={pwd}
                       onChange={(e) => {
                           setPwd(e.target.value)
                       }}
                       placeholder={"Password (Optional)"}/>
            </div>
        </Card>
        {
            cloudSession?.access_url &&
            <Card shadowSize={0} className="flex flex-col gap-2 p-1">
                <UrlInputWithCopy url={cloudSession?.access_url ?? ''}/>
            </Card>
        }
        <Card className="flex flex-row gap-2 p-2 items-center">
            {
                cloudSession?.is_in_progress ? (
                    <Button onClick={() => {
                        invoke("cancel_send", {sessionId: cloudSession?.session_id}).then(noop)
                    }} className={"bg-muted-foreground/30 text-primary w-[70px] h-full shadow-lg"}>Cancel</Button>
                ) : cloudSession?.is_completed ? (
                    <Button onClick={() => {
                        invoke("cancel_send", {sessionId: cloudSession?.session_id}).then(noop)
                    }}
                            className={"bg-greenSecondary/40 text-primary w-[70px] shadow-lg hover:bg-greenSecondary/50"}>Continue</Button>
                ) : (
                    <Button onClick={() => {
                        invoke("public_transfer", {password: pwd}).then(noop)
                    }}
                            className={"bg-bluePrimary text-foreground w-[70px] shadow-lg hover:bg-bluePrimary/60"}>Send</Button>
                )
            }
            {
                !!cloudSession?.progress && (
                    <div className="flex flex-col w-full gap-2 pb-2">
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
            setTimeout(() => setIsCopied(false), 2000) // Reset after 2 seconds
        } catch (err) {
            console.error('Failed to copy text: ', err)
        }
    }

    // Function to trim from the center
    const getTrimmedUrl = (url: string, maxLength: number = 40) => {
        if (url.length <= maxLength) return url

        const ellipsis = '...'
        const availableLength = maxLength - ellipsis.length
        const frontLength = Math.ceil(availableLength / 2)
        const backLength = Math.floor(availableLength / 2)

        return url.slice(0, frontLength) + ellipsis + url.slice(-backLength)
    }

    return (
        <TooltipProvider>
            <div className="relative">
                <Tooltip side="top">
                    <TooltipTrigger asChild>
                        <Input
                            value={getTrimmedUrl(url)}
                            disabled={true}
                            className="pr-12 disabled:opacity-100 cursor-default bg-secondary shadow-background"
                        />
                    </TooltipTrigger>
                    <TooltipContent className="max-w-xs break-all">
                        {url}
                    </TooltipContent>
                </Tooltip>
                <button
                    onClick={handleCopy}
                    className="absolute right-2 top-1/2 transform -translate-y-1/2 p-1.5 rounded-md hover:bg-muted transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
                    title={isCopied ? "Copied!" : "Copy to clipboard"}
                >
                    {isCopied ? (
                        <Check className="h-4 w-4 text-green-500"/>
                    ) : (
                        <Copy className="h-4 w-4 text-muted-foreground hover:text-foreground"/>
                    )}
                </button>
            </div>
        </TooltipProvider>
    )
}