import ReactDOM from "react-dom/client"
import React, {useEffect} from "react"
import core from "@/core.ts"
import {Card} from "@/components/ui/card.tsx";
import {Button} from "@/components/ui/button.tsx";
import {invoke} from "@tauri-apps/api/core";

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
            <Card className={"w-full h-full flex bg-black flex-col relative overflow-hidden rounded-lg gap-10 container border-1 border-white/20"}>
                <div className="absolute inset-0 pointer-events-none">
                    <div
                        className="absolute rounded-full opacity-10 blur-[150px]"
                        style={{
                            width: '100vw',
                            height: '100vw',
                            right: '30vw',
                            bottom: '-10vh',
                            backgroundColor: 'var(--color-greenSecondary)'
                        }}
                    />
                    <div
                        className="absolute rounded-full opacity-40 blur-[150px]"
                        style={{
                            width: '100vw',
                            height: '100vw',
                            left: '50vw',
                            bottom: '50vh',
                            backgroundColor: 'var(--color-greenSecondary)'
                        }}
                    />
                </div>
                <div className="relative w-full h-[45%] mt-4 z-10">
                    <img
                        src="/earth.png"
                        alt="earth"
                        className="absolute inset-0 w-full h-full object-contain
                        [mask-image:linear-gradient(to_bottom,black_20%,transparent_100%)]
                        filter hue-rotate-10 saturate-95 contrast-90"
                    />
                </div>
                <div className={"w-full h-fit flex flex-col z-20 gap-4"}>
                    <p className={"text-md font-bold text-green-100"}>We feel thankful that you're here 🙌</p>
                    <div className={"flex flex-col gap-1"}> 
                        <span className={"text-2xl font-bold text-foreground"}>The file transfer</span>
                        <span className={"text-2xl font-bold text-foreground"}>that we can trust 🚀 </span>
                    </div>
                </div>
                <Button onClick={() => {
                    invoke("sign_in").then(console.log)
                }} className={"bg-bluePrimary text-foreground hover:bg-bluePrimary/60"}>Get started</Button>
            </Card>
        </main>
    )
}

export default Window;
