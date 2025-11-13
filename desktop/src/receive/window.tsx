import ReactDOM from "react-dom/client"
import React, {useEffect} from "react"
import core from "@/core.ts"
import {Card} from "@/components/ui/card.tsx";
import {Avatar, AvatarImage} from "@/components/ui/avatar.tsx";
import {Button} from "@/components/ui/button.tsx";

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
        <main className="flex flex-col w-screen h-screen dark rounded-4xl overflow-clip no-drag p-2.5 bg-card/30">
            <Card
                className={"border-1 bg-card/90 h-fit w-full flex flex-row gap-2 items-center p-3 rounded-3xl overflow-clip justify-between text-foreground"}>
                <div className={"flex flex-row gap-2 items-center"}>
                    <div
                        className={"bg-bluePrimary rounded-full aspect-square justify-center items-center text-primaryText flex z-10"}>
                        <Avatar className={"p-1 rounded-2xl h-11 w-11 bg-yellow-600/90 ring-2 ring-yellow-500/30"}>
                            <AvatarImage
                                src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                        </Avatar>
                    </div>
                    <div className={"flex flex-col items-start p-1 z-10"}>
                        <p className={"text-primaryText text-foreground font-thin"}>{"Ciao 👋"}</p>
                        <p className={"text-primaryText"}>{"Tien Dang"}</p>
                    </div>
                </div>
                <div className={"flex flex-row items-center gap-2"}>
                    <Button className={"bg-bluePrimary text-foreground"}>New shelf</Button>
                </div>
            </Card>
                <div className={"flex mt-2 flex-row gap-2 h-full w-full py-1"}>
                    <div className={"h-full flex flex-5/12 flex-col gap-2"}>
                        <Card
                            className={"h-full flex flex-col border rounded-3xl gap-1.5 p-2 overflow-y-auto bg-card/80"}>
                            <div
                                className={"p-2 border-none bg-muted-foreground/10 flex flex-row rounded-2xl items-center gap-2.5 cursor-pointer"}>
                                <Avatar
                                    className={"p-1 border-none rounded-xl h-9 w-9 bg-yellow-600/90 ring-2 ring-yellow-500/30"}>
                                    <AvatarImage
                                        src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                                </Avatar>
                                <div className={"flex flex-col gap-1"}>
                                    <p className="font-medium">Jenny</p>
                                    <p className={"text-xs text-muted-foreground"}>{"2025/11/20 09:11"}</p>
                                </div>
                            </div>
                            <div
                                className={"p-2 border-none flex border-1 bg-muted-foreground/10 flex-row rounded-2xl items-center gap-2.5 cursor-pointer"}>
                                <Avatar className={"p-1 rounded-xl h-9 w-9 bg-yellow-600/90 ring-2 ring-yellow-500/30"}>
                                    <AvatarImage
                                        src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                                </Avatar>
                                <div className={"flex flex-col gap-1"}>
                                    <p className="font-medium">James</p>
                                    <p className={"text-xs text-muted-foreground"}>{"Receiving..."}</p>
                                </div>
                            </div>
                        </Card>
                    </div>
                    <div className={"flex flex-7/12 border-none items-center justify-center bg-transparent"}>
                        <p className={"text-muted-foreground"}>No transfer session selected.</p>
                    </div>
                </div>
        </main>
    )
}

export default Window;
