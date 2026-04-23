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
    const [shelfId, setShelfId] = useState<string | undefined>(undefined)
    const [animationSettled, setAnimationSettled] = useState(false)
    const shelfInitializedRef = useRef(false)
    const label = window.label
    const isFakeShelf = label.startsWith("fake-shelf")
    const showExpand = !isFakeShelf
    const effectiveExpanded = showExpand && isExpanded

    useEffect(() => {
        if (isFakeShelf) return
        if (label.startsWith("send-") && !shelfInitializedRef.current) {
            shelfInitializedRef.current = true
            const id = label.substring(5)
            setShelfId(id)
            invoke("get_or_create_shelf", { shelfId: id })
                .then(() => console.log('get_or_create_shelf success'))
                .catch((err) => console.error('get_or_create_shelf error:', err))
        }
    }, [label, isFakeShelf]);

    useEffect(() => {
        core.launch()
        // Wait 1s for the CSS animation to finish before settling window size
        const timer = setTimeout(() => {
            setAnimationSettled(true)
        }, 1000)
        return () => clearTimeout(timer)
    }, [])

    const ANIMATION_PADDING = 15

    useEffect(() => {
        // Initial window size from Rust is 245x270 (collapsed + 15 buffer)
        // We only call setSize if expanded, or when settling after being expanded.
        const targetWidth = (effectiveExpanded ? 412 : 230) + ANIMATION_PADDING
        const targetHeight = 255 + ANIMATION_PADDING

        if (effectiveExpanded || animationSettled) {
            window.setSize(new LogicalSize(targetWidth, targetHeight))
        }
    }, [effectiveExpanded, animationSettled, window])

    return (
        <main className={`w-screen h-screen dark bg-transparent flex items-center justify-start p-3.5 overflow-clip`} data-no-scrollbar>
            <div className={`${effectiveExpanded ? 'w-[412px]' : 'w-[230px]'} h-[255px] flex flex-row rounded-2xl bg-transparent space-x-0 animate-popup transition-all duration-300`}>
                <div className={`h-[230px] bg-transparent relative min-w-[200px] w-[200px]`}>
                   {isFakeShelf ? (
                       <Shelf
                           shelfId={undefined}
                           isCollapsed={!effectiveExpanded}
                           disabled
                           overlay={<UpgradeOverlay />}
                       />
                   ) : (
                       <Shelf shelfId={shelfId} isCollapsed={!effectiveExpanded} />
                   )}
                   {showExpand && (
                       <Button
                           onClick={() => setIsExpanded(!isExpanded)}
                           className="absolute top-1/2 -right-3 -translate-y-1/2 z-10 w-8 aspect-square h-auto bg-card border-2 border-white/20 hover:bg-background hover:opacity-100 shadow-lg rounded-full flex items-center justify-center p-0"
                       >
                           <ArrowRight
                               className={`w-2 h-2 text-white transition-transform duration-400 ${effectiveExpanded ? 'rotate-180' : 'rotate-0'}`}
                           />
                       </Button>
                   )}
                </div>
                <div className={`h-full overflow-hidden bg-transparent ${effectiveExpanded ? 'flex' : 'hidden'}`}>
                    <Transfer shelfId={shelfId} key={effectiveExpanded ? 'expanded' : 'collapsed'}/>
                </div>
           </div>
        </main>
    )
}

function UpgradeOverlay() {
    const onUpgrade = () => {
        invoke("show_settings_with_tab", {tab: "account"})
        getCurrentWindow().close()
    }
    return (
        <div className="absolute inset-0 z-30 bg-card flex flex-col items-stretch px-4 pt-8 pb-4 select-none pointer-events-none">
            <div className="flex-1 flex items-center justify-center">
                <p className="text-[12px] text-white/70 text-center">You've reached your shelf limit</p>
            </div>
            <Button
                onClick={onUpgrade}
                className="w-full h-[28px] px-4 text-[11px] font-semibold bg-bluePrimary text-white hover:bg-bluePrimary/90 border-none rounded-md shadow-none pointer-events-auto"
            >
                Unlock more
            </Button>
        </div>
    )
}

export default Window;
