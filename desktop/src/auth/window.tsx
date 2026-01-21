import ReactDOM from "react-dom/client"
import React, {useEffect, useState} from "react"
import core from "@/core.ts"
import {motion, noop} from "motion/react"
import {LayoutTextFlip} from "@/components/ui/layout-text-flip.tsx";
import {Button} from "@/components/ui/button.tsx";
import {ArrowRight} from "lucide-react";
import Iridescence from "@/components/iridescene.tsx";
import {invoke} from "@tauri-apps/api/core";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window/>
    </React.StrictMode>,
);

function Window() {
    const [isLoading, setIsLoading] = useState(false)

    useEffect(() => {
        core.launch()
    }, [])

    const handleLogin = () => {
        if (isLoading) return
        setIsLoading(true)
        invoke("authenticate").then(noop)
        setTimeout(() => setIsLoading(false), 10000)
    }

    return (
        <main className="relative w-screen flex flex-col h-screen dark rounded-b-lg bg-background overflow-hidden">
            <div className={"w-full h-full flex-3/4 flex flex-col"}>
                <div
                    className={"absolute z-20 w-fit h-fit bg-background/40 backdrop-blur-[3px] flex items-center justify-center pb-20 p-2 w-full h-full"}>
                    <Title/>
                </div>
                <div className={"absolute relative z-10 w-full h-full rounded-b-xl overflow-clip bg-black"}>
                    <Iridescence
                        color={[0.55, 0.75, 1.0]} // light blue / cyan
                        mouseReact={false}
                        amplitude={0.06}        // softer movement
                        speed={0.7}             // calmer animation
                    />
                </div>
            </div>
            <div
                className={"absolute z-20 bottom-0 flex-1/4 w-full h-[20vh] flex flex-col justify-center items-center bg-black/50 backdrop-blur-2xl gap-2"}>
                <Button
                    onClick={handleLogin}
                    disabled={isLoading}
                    className={"px-8 py-4 bg-bluePrimary text-white disabled:opacity-70"}>
                    {isLoading ? (
                        <div className="h-4 w-4 animate-spin rounded-full border-2 border-white/20 border-t-white"></div>
                    ) : (
                        <>Get started <ArrowRight className={"scale-y-120 scale-x-120"}/></>
                    )}
                </Button>
            </div>
        </main>
    )
}

export function Title() {
    return (
        <div className={"text-white text-lg"}>
            <motion.div
                className="relative mx-4 flex flex-col items-center justify-center gap-4 text-center sm:mx-0 sm:mb-0 sm:flex-row">
                <LayoutTextFlip
                    text="Welcome to"
                    words={["Bytover", "File shelf", "File transfer", "Peer to Peer"]}
                />
            </motion.div>
        </div>
    );
}

export default Window;
