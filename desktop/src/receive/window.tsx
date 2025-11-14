import ReactDOM from "react-dom/client"
import React, {useEffect} from "react"
import core from "@/core.ts"
import {Card} from "@/components/ui/card.tsx";
import {Avatar, AvatarImage} from "@/components/ui/avatar.tsx";
import {Button} from "@/components/ui/button.tsx";
import CircleProgress from "@/components/ui/progress.tsx";
import {Label} from "@/components/ui/label.tsx";
import {Inbox} from "lucide-react";

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
        <main className="flex flex-col w-screen h-screen dark rounded-4xl overflow-clip no-drag p-0 bg-card/30">
            <div className={"flex flex-row h-full w-full z-20 p-1 "}>
                <div className={"h-full flex flex-2/5 flex-col gap-2"}>
                    <Card
                        className={"h-full bg-card/50 backdrop-blur-lg flex flex-col border rounded-3xl gap-1.5 p-2 m-1 overflow-y-auto"}>
                        <Label className={"flex flex-row items-center gap-2 px-1 py-1 text-muted-foreground"}>
                            <Inbox size={20} className={"bg-muted-foreground/10 border rounded-md p-1"}/>
                            <p>Inbox</p>
                        </Label>
                        <Card
                            shadowSize={0.2}
                            className={"p-2 bg-muted/80 border-1 flex flex-row rounded-2xl items-center gap-2.5 cursor-pointer"}>
                            <Avatar
                                className={"p-1 border-none rounded-xl h-9 w-9 bg-yellow-600/90 ring-2 ring-yellow-500/30"}>
                                <AvatarImage
                                    src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                            </Avatar>
                            <div className={"flex flex-col gap-1"}>
                                <p className="font-medium">Jenny</p>
                                <p className={"text-xs text-muted-foreground"}>{"2025/11/20 09:11"}</p>
                            </div>
                        </Card>
                        <Card
                            shadowSize={0.4}
                            className={"p-2 bg-muted/80 flex border-1 flex-row rounded-2xl items-center gap-2.5 cursor-pointer"}>
                            <Avatar className={"p-1 rounded-xl h-9 w-9 bg-yellow-600/90 ring-2 ring-yellow-500/30"}>
                                <AvatarImage
                                    src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                            </Avatar>
                            <div className={"flex flex-col gap-1"}>
                                <p className="font-medium">James</p>
                                <p className={"text-xs text-muted-foreground"}>{"Receiving..."}</p>
                            </div>
                        </Card>
                    </Card>
                </div>
                <div
                    className={"flex-3/5 gap-1 pb-2 rounded-t-4xl h-fit w-full px-2 flex flex-col items-center shadow-lg shadow-background/20 pt-2 border-b-1 border-muted-foreground/10 bg-card/10 backdrop-blur-2xl overflow-clip justify-between text-foreground"}>
                    <div className={"flex flex-row items-center p-1 justify-between w-full"}>
                        <div className={"flex flex-row gap-1 items-center"}>
                            <div
                                className={"bg-bluePrimary rounded-full aspect-square justify-center items-center text-primaryText flex z-10"}>
                                <Avatar className={"p-1 rounded-2xl h-10 w-10 bg-yellow-600/90 ring-2 ring-yellow-500/30"}>
                                    <AvatarImage
                                        src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                                </Avatar>
                            </div>
                            <div className={"flex flex-col items-start p-1 z-10"}>
                                <p className={"text-primaryText text-foreground font-thin"}>{"Ciao,"}</p>
                                <p className={"text-primaryText"}>{"Tien Dang"}</p>
                            </div>
                        </div>
                        <div className={"flex flex-row items-center gap-2"}>
                            <Button className={"bg-bluePrimary text-foreground"}>New shelf</Button>
                        </div>
                    </div>
                    <Card
                        className={"bg-muted/40 border-1 shadow-md h-fit w-full flex flex-row gap-2 items-center px-2 py-1 rounded-2xl overflow-clip justify-between text-foreground"}>
                        <div className={"flex flex-row gap-2 items-center"}>
                            <div className={"flex flex-col items-start p-1 z-10"}>
                                <p className={"text-primaryText"}>{"30 GB"}</p>
                            </div>
                        </div>
                        <div className={"flex flex-row items-center gap-3 h-2"}>
                            <p className={"text-muted-foreground"}>{"3 MB/s"}</p>
                            <CircleProgress progress={0.35} size={22}/>
                        </div>
                    </Card>
                </div>
            </div>
        </main>
    )
}

export default Window;
