import {
    Tabs,
    TabsPanel,
    TabsPanels,
    TabsList,
    TabsTab,
} from '@/components/animate-ui/components/base/tabs'
import { Button } from "@/components/ui/button"
import {
    Card,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
} from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
    Lock,
    Mail,
    MapPin,
    SendHorizonal,
    Copy,
    Check,
    Link,
    PersonStanding,
    Users,
    Globe,
    Laptop,
    Phone, Smartphone
} from "lucide-react"
import core from "@/core.ts"
import { Avatar, AvatarImage } from "@/components/ui/avatar"
import {
    PeerViewModel,
} from 'shared_types/types/shared_types'
import CircleProgress from "@/components/ui/progress.tsx"
import { invoke } from "@tauri-apps/api/core"
import { noop } from "motion"
import { Slide } from "@/components/animate-ui/primitives/effects/slide.tsx"
import { MotionGridSignalling } from "@/components/animate-ui/primitives/animate/motion-grid.tsx"
import { useState } from "react"
import { Progress } from "@/components/animate-ui/components/radix/progress"
import { ProgressIndicator } from "@/components/animate-ui/primitives/radix/progress"
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from "@/components/animate-ui/primitives/animate/tooltip"

export function Transfer() {
    return (
        <div className="flex w-full flex-col gap-6 h-full overflow-hidden">
            <Slide
                delay={240}
                direction={"left"}
                offset={380}
                className="h-full flex">
                <Tabs defaultValue="people" className="w-full items-start flex flex-col h-full">
                    <TabsList className={"ml-2 border-2 shadow-background shadow-sm flex-shrink-0"}>
                        <TabsTab value="people"><Users /> People</TabsTab>
                        <TabsTab value="public"><Globe />Link</TabsTab>
                        <TabsTab value="devices"><Smartphone />Device</TabsTab>
                    </TabsList>
                    <div className="pl-2 border-none bg-transparent relative w-full flex-1 flex flex-col min-h-0 overflow-hidden">
                        <TabsPanels className="flex-1 flex flex-col min-h-0 overflow-hidden">
                            <TabsPanel value="people" className="flex flex-col h-full overflow-hidden">
                                <CardContent className={"p-0 flex flex-col gap-2 h-full overflow-hidden"}>
                                    <Card shadowSize={0.5} className="flex flex-col gap-2 py-2 p-1.5 bg-card/95 flex-shrink-0">
                                        <Label htmlFor="tabs-input-email"
                                            className={"flex flex-row items-center gap-1 bg-muted px-2 py-1 w-fit rounded-md"}>
                                            <div
                                                className={"bg-white/10 p-[3px] rounded-sm w-5 h-5 flex items-center justify-center"}>
                                                <Mail />
                                            </div>
                                            Emails:
                                        </Label>
                                        <div className={"flex flex-row gap-2 h-fit"}>
                                            <Input className={"bg-secondary shadow-background"} id="tabs-input-email"
                                                type={"email"} defaultValue="someone@company.com" />
                                            <Button variant={"default"} className={"w-[32px] bg-bluePrimary/80"}>
                                                <SendHorizonal color={"white"} />
                                            </Button>
                                        </div>
                                    </Card>
                                    <Card
                                        shadowSize={0.5}
                                        className="flex flex-col gap-5 bg-card/95 p-1.5 overflow-hidden flex-1 min-h-0">
                                        <Label
                                            className={"flex flex-row items-center gap-2 bg-muted px-2 mb-2 py-1 w-fit rounded-md shadow-black shrink-0"}>
                                            {
                                                <div
                                                    className={"bg-white/10 p-[3px] rounded-sm w-5 h-5 flex items-center justify-center"}>
                                                    <MapPin />
                                                </div>
                                            }
                                            Nearby:
                                        </Label>
                                        <div className="flex-1 min-h-0 overflow-y-auto">
                                            <NearbyList />
                                        </div>
                                    </Card>
                                </CardContent>
                            </TabsPanel>
                            <TabsPanel value="public" className="flex flex-col gap-2">
                                <CardContent className={"p-0 flex flex-col gap-2"}>
                                    <PublicTransfer />
                                </CardContent>
                            </TabsPanel>
                            <TabsPanel value="devices" className="flex flex-col gap-6">
                                <CardHeader>
                                    <CardTitle>Devices</CardTitle>
                                    <CardDescription>
                                        Sharing to your devices.
                                    </CardDescription>
                                </CardHeader>
                                <CardContent className="grid gap-6">
                                    <div className="grid gap-3">
                                        <Label htmlFor="tabs-demo-current">Password</Label>
                                        <Input id="tabs-demo-current" type="password" />
                                    </div>
                                </CardContent>
                                <CardFooter>
                                    <Button>Send</Button>
                                </CardFooter>
                            </TabsPanel>
                        </TabsPanels>
                    </div>
                </Tabs>
            </Slide>
        </div>
    );
}

function NearbyList() {
    const list = core.useNearbyListState();

    return <div className={"flex flex-col gap-2 w-full h-full relative"}>
        {
            list.map((it) => <>
                <NearbyPeer peer={it}></NearbyPeer>
            </>)
        }
        {
            !list.length &&
            <div className={"flex flex-col items-center h-[30px]"}>
                <MotionGridSignalling />
            </div>
        }
        {/* Bottom fade mask */}
        <div
            className="absolute bottom-0 left-0 right-0 h-16 bg-gradient-to-t from-card/95 to-transparent pointer-events-none z-20" />
    </div>
}

function NearbyPeer(props: { peer: PeerViewModel }) {
    const peer = core.usePeerState(props.peer?.id) || props.peer
    const color = `rgb(${peer.avatar.dominant_color_r}, ${peer.avatar.dominant_color_g}, ${peer.avatar.dominant_color_b})`

    // Check if transfer is completed (progress reaches 1.0)
    const isCompleted = peer.transfer_progress >= 1.0 && peer.transfer_progress > 0;
    const isInProgress = peer.transfer_progress > 0 && !isCompleted;

    return <>
        <Card
            shadowSize={0}
            className={"flex flex-row overflow-clip bg-muted hover:bg-muted-foreground/30 items-center px-2 py-1 h-fit w-full justify-between"}
            onClick={() => {
                invoke("start_transfer", { targetId: peer.id }).then(noop)
            }}>
            <div className={"flex flex-row items-center gap-3"}>
                <div
                    className={"bg-bluePrimary rounded-xl aspect-square justify-center items-center text-primaryText flex h-[34px] w-[34px]"}>
                    <Avatar className={"p-1 rounded-xl h-fit"} style={{ backgroundColor: color }}>
                        <AvatarImage src={peer.avatar.url} />
                    </Avatar>
                </div>
                <div className={"flex flex-col gap-1 items-start"}>
                    <p className={"text-primaryText font-bold text-sm"}>{peer.display_name}</p>
                    {
                        peer.display_upload_speed
                            ? <p className={"text-primaryText/70 text-xs"}>{peer.display_upload_speed}</p>
                            : peer.device.name !== peer.display_name && <>
                                <p className={"text-muted-foreground"}>{peer.device.name}</p>
                            </>
                    }
                </div>
            </div>
            {
                <div className={"w-[40px] h-[40px] flex flex-col justify-center items-center"}>
                    <CircleProgress
                        strokeWidth={3}
                        progress={Number(peer.transfer_progress)}
                        isInProgress={isInProgress}
                        isCompleted={isCompleted}
                        size={30}
                    />
                </div>
            }
        </Card>
    </>
}

function PublicTransfer() {
    const [pwd, setPwd] = useState("");
    const cloudSession = core.useTransferState()?.cloud_session
    const progress = (cloudSession?.progress ?? 0) * 100

    return <>
        <Card shadowSize={0} className="flex flex-col gap-2 p-2">
            <Label
                className={"flex flex-row items-center gap-1 bg-muted px-2 py-1 w-fit rounded-md"}>
                <div
                    className={"bg-white/10 p-[3px] rounded-sm w-5 h-5 flex items-center justify-center"}>
                    <Lock />
                </div>
                Password:
            </Label>
            <div className="grid gap-3">
                <Input className={"bg-secondary shadow-background"} type="password"
                    value={pwd}
                    onChange={(e) => {
                        setPwd(e.target.value)
                    }}
                    placeholder={"Pwd@123"} />
                {
                    cloudSession?.access_url &&
                    <>
                        <Label
                            className={"flex flex-row items-center gap-1 bg-muted px-2 py-1 w-fit rounded-md"}>
                            <div
                                className={"bg-white/10 p-[3px] rounded-sm w-5 h-5 flex items-center justify-center"}>
                                <Link />
                            </div>
                            Generated url:
                        </Label>
                        <UrlInputWithCopy url={cloudSession?.access_url ?? ''} />
                    </>
                }
            </div>
        </Card>
        <Card className="flex flex-row gap-2 p-2 items-center">
            {
                cloudSession?.is_in_progress ? (
                    <Button onClick={() => {
                        invoke("cancel_send", { sessionId: cloudSession?.session_id }).then(noop)
                    }} className={"bg-muted-foreground/30 text-primary w-[70px] h-full shadow-lg"}>Cancel</Button>
                ) : cloudSession?.is_completed ? (
                    <Button onClick={() => {
                        invoke("cancel_send", { sessionId: cloudSession?.session_id }).then(noop)
                    }} className={"bg-greenSecondary/40 text-primary w-[70px] shadow-lg hover:bg-greenSecondary/50"}>Continue</Button>
                ) : (
                    <Button onClick={() => {
                        invoke("public_transfer", { password: pwd }).then(noop)
                    }} className={"bg-bluePrimary text-foreground w-[70px] shadow-lg hover:bg-bluePrimary/60"}>Send</Button>
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
                            <ProgressIndicator className="bg-primary rounded-full h-full w-full flex-1" />
                        </Progress>
                    </div>
                )
            }
        </Card>
    </>
}

function UrlInputWithCopy({ url }: { url: string }) {
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
                            className="pr-12 cursor-default bg-secondary shadow-background" // Add padding for the button and cursor
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
                        <Check className="h-4 w-4 text-green-500" />
                    ) : (
                        <Copy className="h-4 w-4 text-muted-foreground hover:text-foreground" />
                    )}
                </button>
            </div>
        </TooltipProvider>
    )
}