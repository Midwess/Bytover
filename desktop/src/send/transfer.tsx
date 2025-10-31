import {
    Tabs,
    TabsPanel,
    TabsPanels,
    TabsList,
    TabsTab,
} from '@/components/animate-ui/components/base/tabs';
import {Button} from '@/components/ui/button';
import {
    Card,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
} from '@/components/ui/card';
import {Input} from '@/components/ui/input';
import {Label} from '@/components/ui/label';
import {Mail, MapPin, SendHorizonal} from "lucide-react";
import core from "@/core.ts";
import {Avatar, AvatarImage} from "@/components/ui/avatar";
import {
    PeerViewModel,
} from 'shared_types/types/shared_types'
import CircleProgress from "@/components/ui/progress.tsx";
import {invoke} from "@tauri-apps/api/core";
import {noop} from "motion";
import {Slide} from "@/components/animate-ui/primitives/effects/slide.tsx";
import {MotionGridSignalling} from "@/components/animate-ui/primitives/animate/motion-grid.tsx";

export function Transfer() {
    return (
        <div className="flex w-full max-w-sm flex-col gap-6">
            <Slide
                delay={240}
                direction={"left"}
                offset={380}>
                <Tabs defaultValue="people" className="w-full items-start">
                    <TabsList className={"ml-2 border shadow-background shadow-sm"}>
                        <TabsTab value="people">People</TabsTab>
                        <TabsTab value="public">Public</TabsTab>
                        <TabsTab value="devices">Devices</TabsTab>
                    </TabsList>
                    <Card className="px-2 border-none bg-transparent relative">
                        <TabsPanels>
                            <TabsPanel value="people" className="flex flex-col">
                                <CardContent className={"p-0 flex flex-col gap-2"}>
                                    <Card className="flex flex-col gap-2 py-2 p-1.5 bg-card/95 rounded-lg min-h-fit">
                                        <Label htmlFor="tabs-input-email"
                                               className={"flex flex-row items-center gap-1 bg-muted px-2 py-1 w-fit rounded-md"}>
                                            <div
                                                className={"bg-white/10 p-[3px] rounded-sm w-5 h-5 flex items-center justify-center"}>
                                                <Mail/>
                                            </div>
                                            Emails:
                                        </Label>
                                        <div className={"flex flex-row gap-2 h-fit"}>
                                            <Input className={"bg-secondary shadow-background"} id="tabs-input-email"
                                                   type={"email"} defaultValue="someone@company.com"/>
                                            <Button variant={"default"} className={"w-[32px] bg-bluePrimary/80"}>
                                                <SendHorizonal color={"white"}/>
                                            </Button>
                                        </div>
                                    </Card>
                                    <Card
                                        className="flex flex-col gap-5 py-2 bg-card/95 border p-1.5 overflow-y-scroll">
                                        <Label
                                            className={"flex flex-row items-center gap-2 bg-muted px-2 mb-2 py-1 w-fit rounded-md shadow-black"}>
                                            {
                                                <div
                                                    className={"bg-white/10 p-[3px] rounded-sm w-5 h-5 flex items-center justify-center"}>
                                                    <MapPin/>
                                                </div>
                                            }
                                            Nearby:
                                        </Label>
                                        <NearbyList/>
                                    </Card>
                                </CardContent>
                            </TabsPanel>
                            <TabsPanel value="public" className="flex flex-col gap-6">
                                <CardHeader>
                                    <CardTitle>Public</CardTitle>
                                    <CardDescription>
                                        Public your resources with protected password.
                                    </CardDescription>
                                </CardHeader>
                                <CardContent className="grid gap-6">
                                    <div className="grid gap-3">
                                        <Label htmlFor="tabs-demo-current">Password</Label>
                                        <Input id="tabs-demo-current" type="password"/>
                                    </div>
                                </CardContent>
                                <CardFooter>
                                    <Button>Send</Button>
                                </CardFooter>
                            </TabsPanel>
                            <TabsPanel value="public" className="flex flex-col gap-6">
                                <CardHeader>
                                    <CardTitle>Devices</CardTitle>
                                    <CardDescription>
                                        Sharing to your devices.
                                    </CardDescription>
                                </CardHeader>
                                <CardContent className="grid gap-6">
                                    <div className="grid gap-3">
                                        <Label htmlFor="tabs-demo-current">Password</Label>
                                        <Input id="tabs-demo-current" type="password"/>
                                    </div>
                                </CardContent>
                                <CardFooter>
                                    <Button>Send</Button>
                                </CardFooter>
                            </TabsPanel>
                        </TabsPanels>
                    </Card>
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
            <div className={"flex flex-col items-center pb-1 rounded-full"}>
                <MotionGridSignalling size={3}/>
            </div>
        }
        {/* Bottom fade mask */}
        <div className="absolute bottom-0 left-0 right-0 h-16 bg-gradient-to-t from-card/95 to-transparent pointer-events-none z-20"/>
    </div>
}

function NearbyPeer(props: { peer: PeerViewModel }) {
    const peer = core.usePeerState(props.peer?.id) || props.peer
    const color = `rgb(${peer.avatar.dominant_color_r}, ${peer.avatar.dominant_color_g}, ${peer.avatar.dominant_color_b})`

    return <>
        <Card
            className={"flex flex-row overflow-clip bg-muted hover:bg-muted-foreground/30 items-center px-2 py-1 h-fit w-full justify-between"}
            onClick={() => {
                invoke("start_transfer", {targetId: peer.id}).then(noop)
            }}>
            <div className={"flex flex-row items-center gap-3"}>
                <div
                    className={"bg-bluePrimary rounded-xl aspect-square justify-center items-center text-primaryText flex h-[34px] w-[34px]"}>
                    <Avatar className={"p-1 rounded-xl h-fit"} style={{backgroundColor: color}}>
                        <AvatarImage src={peer.avatar.url}/>
                    </Avatar>
                </div>
                <div className={"flex flex-col gap-1 items-start"}>
                    <p className={"text-primaryText font-bold text-sm"}>{peer.display_name}</p>
                    {
                        peer.display_upload_speed
                            ? <p className={"text-primaryText/70 text-xs"}>{peer.display_upload_speed}</p>
                            : <></>
                    }
                </div>
            </div>
            {
                <div className={"w-[40px] h-[40px] flex justify-center items-center"}>
                    {peer.transfer_progress ? <CircleProgress progress={peer.transfer_progress} size={35}/> : <></>}
                </div>
            }
        </Card>
    </>
}