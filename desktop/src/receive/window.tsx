import ReactDOM from "react-dom/client"
import React, {useEffect} from "react"
import core from "@/core.ts"
import {Card} from "@/components/ui/card.tsx";
import {Avatar, AvatarImage} from "@/components/ui/avatar.tsx";

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
        <main className="relative flex flex-col w-screen h-screen dark rounded-4xl overflow-clip no-drag p-2 gap-2">
           <Card className={"w-full h-full flex px-2 gap-2 flex-row rounded-3xl z-20 overflow-x-clip p-2 shadow-black border-white/30"}>
               <div className={"flex flex-5/12 flex-col border rounded-2xl gap-1.5 p-1 overflow-y-auto"}>
                   <Card className={"p-1.5 bg-muted flex flex-row rounded-xl items-center gap-1.5"}>
                       <Avatar className={"p-1 rounded-xl h-8 w-8 bg-yellow-600/90"}>
                           <AvatarImage src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                       </Avatar>
                       <p>Thieu Ngoc</p>
                   </Card>
                   <Card className={"p-1.5 flex bg-muted flex-row rounded-xl items-center gap-1.5"}>
                       <Avatar className={"p-1 rounded-xl h-8 w-8 bg-yellow-600/90"}>
                           <AvatarImage src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                       </Avatar>
                       <p>Thieu Ngoc</p>
                   </Card>
               </div>
               <Card className={"flex flex-7/12 bg-muted"}>

               </Card>
           </Card>
           <Card className={"relative h-fit w-full flex flex-row gap-2 items-center py-3 px-3 rounded-3xl overflow-clip shadow-black/50 border-white/30"}>
                <div className={"w-[90%] h-full top-0 scale-x-125 absolute z-0 opacity-50"}>
                    <img src={"/lineillus.svg"} className={"w-full h-full object-cover"}/>
                </div>
                <div className={"bg-bluePrimary rounded-full aspect-square justify-center items-center text-primaryText flex z-10"}>
                    <Avatar className={"p-1 rounded-xl h-8 w-8 bg-yellow-600/90"}>
                        <AvatarImage src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Chicken.png?r=215&g=179&b=100"}/>
                    </Avatar>
                </div>
                <div className={"flex flex-col items-start p-1 z-10"}>
                    <p className={"text-primaryText text-xs text-foreground/90"}>{"Hello,"}</p>
                    <p className={"text-primaryText font-medium"}>{"Tien Dang"}</p>
                </div>
           </Card>
        </main>
    )
}

export default Window;
