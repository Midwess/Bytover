import ReactDOM from "react-dom/client";
import React, {useEffect} from "react";
import { Shelf } from "./shelf";
import { Transfer } from "./transfer.tsx";
import core from "@/core.ts";
import {useOverlayScrollbars} from "@/hooks/use-overlay-scrollbar.ts";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window />
    </React.StrictMode>,
);

function Window() {
    useOverlayScrollbars()
    useEffect(() => {
        console.log("launching core")
        core.launch()
    }, [])

    return (
        <main className="w-screen h-screen overflow-hidden p-2 dark bg-transparent">
            <div className={"w-full h-full flex flex-row rounded-2xl bg-transparent space-x-1"}>
                <div className={"flex-6/12 w-full h-full bg-transparent"}>
                    <Shelf/>
                </div>
                <div className={"flex-7/12 w-full h-full bg-transparent"}>
                    <Transfer/>
                </div>
            </div>
        </main>
    )
}

export default Window;
