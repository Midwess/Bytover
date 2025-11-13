import ReactDOM from "react-dom/client"
import React, {useEffect} from "react"
import core from "@/core.ts"
import {Card} from "@/components/ui/card.tsx";
import {Avatar, AvatarImage} from "@/components/ui/avatar.tsx";
import {Button} from "@/components/ui/button.tsx";
import {Settings} from "lucide-react";

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
        <main className="flex flex-col w-screen h-screen dark rounded-4xl overflow-clip no-drag p-2 gap-2">
           <Card className={"relative w-full h-full flex px-2 gap-2 overflow-clip border-2 flex-col rounded-3xl z-20 overflow-x-clip"}>
               <div className={"flex flex-row gap-2 h-full w-full py-1.5"}>
               <div className={"bg-card flex flex-5/12 flex-col border rounded-2xl gap-1.5 p-1.5 overflow-y-auto"}>
                   <Card className={"p-1.5 bg-muted border-1 flex flex-row rounded-xl items-center gap-2.5 cursor-pointer"}>
                       <Avatar className={"p-1 border-none rounded-xl h-8 w-8 bg-yellow-600/90 ring-2 ring-yellow-500/30"}>
                           <AvatarImage src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                       </Avatar>
                       <div className={"flex flex-col"}>
                           <p className="font-medium">Jenny</p>
                           <p className={"text-xs text-muted-foreground"}>{"2025/11/20 09:11"}</p>
                       </div>
                   </Card>
                   <Card className={"p-1.5 flex bg-muted border-1 flex-row rounded-xl items-center gap-2.5 cursor-pointer"}>
                       <Avatar className={"p-1 rounded-xl h-8 w-8 bg-yellow-600/90 ring-2 ring-yellow-500/30"}>
                           <AvatarImage src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                       </Avatar>
                       <div className={"flex flex-col"}>
                           <p className="font-medium">James</p>
                           <p className={"text-xs text-muted-foreground"}>{"Receiving"}</p>
                       </div>
                   </Card>
               </div>
               <Card className={"flex flex-7/12 rounded-2xl border-1"}>
               </Card>
               </div>
           </Card>
           <Card className={"relative border-1 h-fit w-[190px] flex flex-row gap-2 items-center py-3 px-3 rounded-2xl overflow-clip justify-between"}>
               <div className={"flex flex-row gap-2 items-center"}>
                <div className={"bg-bluePrimary rounded-full aspect-square justify-center items-center text-primaryText flex z-10"}>
                    <Avatar className={"p-1 rounded-xl h-8 w-8 bg-yellow-600/90 ring-2 ring-yellow-500/30"}>
                        <AvatarImage src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                    </Avatar>
               </div>
                <div className={"flex flex-col items-start p-1 z-10"}>
                    <p className={"text-primaryText text-xs text-foreground/90"}>{"Hello,"}</p>
                    <p className={"text-primaryText font-medium"}>{"Tien Dang"}</p>
                </div>
               </div>
               <Button variant={"secondary"} className={"rounded-xl"}>
                   <Settings className={"h-4 w-4"}/>
               </Button>
           </Card>
        </main>
    )
}

export default Window;
