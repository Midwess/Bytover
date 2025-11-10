import ReactDOM from "react-dom/client"
import React, {useEffect} from "react"
import core from "@/core.ts"
import {Card} from "@/components/ui/card.tsx";
import {Button} from "@/components/ui/button.tsx";
import {invoke} from "@tauri-apps/api/core";
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
        <main className="w-screen h-screen dark rounded-lg">
            <Card className={"w-full h-full flex bg-black flex-col relative overflow-hidden rounded-3xl gap-8 border-white/20 container border-1"}>
                <div className="absolute inset-0 pointer-events-none opacity-100">
                    <div
                        className="absolute rounded-full opacity-30 blur-[100px]"
                        style={{
                            width: '100vw',
                            height: '100vw',
                            right: '-20vw',
                            bottom: '80vh',
                            backgroundColor: 'var(--color-greenSecondary)'
                        }}
                    />
                    <div
                        className="absolute rounded-full opacity-30 blur-[200px]"
                        style={{
                            width: '100vw',
                            height: '100vw',
                            left: '-50vw',
                            bottom: '-50vh',
                            backgroundColor: 'var(--color-greenSecondary)'
                        }}
                    />
                </div>
                <div className={"flex flex-col gap-2"}>
                    <div className={"flex flex-row justify-between items-center"}>
                        <p className={"text-greenSecondary bg-green-900/20 rounded-lg py-1 px-2 h-fit"}>Bytes Ciao</p>
                        <Button variant={"ghost"}>
                            <Settings size={15}/>
                        </Button>
                    </div>
                    <div className={"bg-card rounded-xl p-2 flex flex-row gap-5 justify-between items-center"}>
                        <div className={"flex flex-row gap-2 items-center"}>
                            <img src={"https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/avatars/Tiger.png?r=193&g=138&b=78"} className={"aspect-square w-10 bg-foreground/10 rounded-full p-1.5"}/>
                            <p className={"font-bold"}>Tien dang</p>
                        </div>
                   </div>
                </div>
            </Card>
        </main>
    )
}

export default Window;
