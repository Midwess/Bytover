import ReactDOM from "react-dom/client";
import React, {useCallback, useEffect, useState} from "react";
import { Shelf } from "./shelf";
import { Transfer } from "./transfer.tsx";
import core from "@/core.ts";
import {useOverlayScrollbars} from "@/hooks/use-overlay-scrollbar.ts";
import {getCurrentWindow, LogicalSize} from "@tauri-apps/api/window";
import { ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button.tsx";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window />
    </React.StrictMode>,
);

function Window() {
    useOverlayScrollbars()
    const window = getCurrentWindow()
    const [windowSize, setWindowSize] = useState(new LogicalSize(260, 280))
    const [isExpanded, setIsExpanded] = useState(false)

    useEffect(() => {
        window.setSize(windowSize)
    }, [windowSize, window]);

    useEffect(() => {
        core.launch()
    }, [])

    const toggleExpand = useCallback(() => {
        if (isExpanded) {
            // Collapse - make window smaller
            setIsExpanded(false)
            setWindowSize(new LogicalSize(260, 280))
        } else {
            // Expand - make window larger
            setIsExpanded(true)
            setWindowSize(new LogicalSize(480, 280))
        }
    }, [isExpanded])

    return (
        <main
            className="
    w-screen h-screen dark
    bg-transparent flex flex-col p-1
    transition-all duration-300

    opacity-0 scale-90 animate-[popup_180ms_ease-out_forwards]
  "
        >            <div className={"w-full h-full flex flex-row rounded-2xl bg-transparent space-x-0"}>
                <div className={`h-full bg-transparent relative min-w-[245px] w-[245px]`}>
                   <Shelf/>
                   {/* Toggle button at the middle-right edge */}
                   <Button
                       onClick={toggleExpand}
                       className="absolute top-1/2 -right-3 -translate-y-1/2 z-10 w-3 aspect-square h-auto bg-card border-2 shadow-lg rounded-full flex items-center justify-center"
                   >
                       <ArrowRight
                           className={`w-3 h-3 text-white transition-transform duration-400 ${isExpanded ? 'rotate-180' : 'rotate-0'}`}
                       />
                   </Button>
                </div>
                <div className={`w-full h-full bg-transparent ${isExpanded ? 'flex' : 'hidden'}`}>
                    <Transfer key={isExpanded ? 'expanded' : 'collapsed'}/>
                </div>
           </div>
        </main>
    )
}

export default Window;
