import ReactDOM from "react-dom/client";
import React, {useEffect, useRef, useState} from "react";
import { Shelf } from "./shelf";
import { Transfer } from "./transfer.tsx";
import core from "@/core.ts";
import {useOverlayScrollbars} from "@/hooks/use-overlay-scrollbar.ts";
import {getCurrentWindow, LogicalSize} from "@tauri-apps/api/window";
import {ArrowRight} from "lucide-react";
import {Button} from "@/components/ui/button.tsx";
import {invoke} from "@tauri-apps/api/core";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window />
    </React.StrictMode>,
);

function Window() {
    useOverlayScrollbars()
    const window = getCurrentWindow()
    const [isExpanded, setIsExpanded] = useState(false)
    const [showAnimation, setShowAnimation] = useState(false)
    const [shelfId, setShelfId] = useState<string | undefined>(undefined)
    const shelfInitializedRef = useRef(false)

    // Extract shelf ID from window label on mount (label format: "send-{shelf_id}")
    const label = window.label
    useEffect(() => {
        if (label.startsWith("send-") && !shelfInitializedRef.current) {
            shelfInitializedRef.current = true
            const id = label.substring(5) // Remove "send-" prefix
            setShelfId(id)
            invoke("get_or_create_shelf", { shelfId: id })
                .then(() => console.log('get_or_create_shelf success'))
                .catch((err) => console.error('get_or_create_shelf error:', err))

            // Trigger popup animation on mount
            setTimeout(() => {
                setShowAnimation(true)
                setTimeout(() => setShowAnimation(false), 300)
            }, 50)
        }
    }, [label]);

    useEffect(() => {
        let unlistenShow: () => void;

        window.listen("window-shown", () => {
            setIsExpanded(false)
            setTimeout(() => {
                setShowAnimation(true)
                setTimeout(() => setShowAnimation(false), 300)
            }, 50)
        }).then(unlisten => { unlistenShow = unlisten });

        return () => {
            if (unlistenShow) unlistenShow();
        };
    }, [window]);

    useEffect(() => {
        core.launch()
    }, [])

    useEffect(() => {
        const width = isExpanded ? 400 : 220
        window.setSize(new LogicalSize(width, 260))
    }, [isExpanded, window])

    return (
        <main className={`w-screen h-screen dark bg-transparent rounded-2xl flex flex-col p-1 overflow-clip transition-all duration-300 ${showAnimation ? 'animate-popup' : ''}`}>
            <div className={"w-full h-full flex flex-row rounded-2xl bg-transparent space-x-0"}>
                <div className={`h-[220px] bg-transparent relative min-w-[195px] w-[190px]`}>
                   <Shelf shelfId={shelfId} />
                   <Button
                       onClick={() => setIsExpanded(!isExpanded)}
                       className="absolute top-1/2 -right-3 -translate-y-1/2 z-10 w-8 aspect-square h-auto bg-card border-2 border-white/20 hover:bg-background hover:opacity-100 shadow-lg rounded-full flex items-center justify-center p-0"
                   >
                       <ArrowRight
                           className={`w-2 h-2 text-white transition-transform duration-400 ${isExpanded ? 'rotate-180' : 'rotate-0'}`}
                       />
                   </Button>
                </div>
                <div className={`w-full h-[100%] overflow-hidden bg-transparent ${isExpanded ? 'flex' : 'hidden'}`}>
                    <Transfer shelfId={shelfId} key={isExpanded ? 'expanded' : 'collapsed'}/>
                </div>
           </div>
        </main>
    )
}

export default Window;
