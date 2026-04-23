import ReactDOM from "react-dom/client";
import React, {useEffect, useRef, useState} from "react";
import { Shelf, ShelfWrapper } from "./shelf";
import { Transfer } from "./transfer.tsx";
import core from "@/core.ts";
import {useOverlayScrollbars} from "@/hooks/use-overlay-scrollbar.ts";
import {getCurrentWindow, LogicalSize} from "@tauri-apps/api/window";
import {ArrowRight, Check} from "lucide-react";
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
    const isFakeShelf = label === "fake-shelf"
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
                       <UpgradeDialogContent isCollapsed={!effectiveExpanded} />
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

function UpgradeDialogContent({isCollapsed}: {isCollapsed: boolean}) {
    const onUpgrade = () => {
        invoke("show_settings_with_tab", {tab: "account"})
        getCurrentWindow().close()
    }
    const onClose = () => getCurrentWindow().close()
    const features = [
        "Unlimited shelves",
        "No transfer size cap",
        "Password-protected transfers",
    ]
    return (
        <ShelfWrapper isCollapsed={isCollapsed}>
            <div className="absolute inset-0 pt-9 flex items-center justify-center pointer-events-none select-none opacity-30">
                <p className="text-md text-muted-foreground">Drop or paste files here</p>
            </div>
            <div className="absolute inset-0 z-30 bg-black/40 backdrop-blur-[2px] flex flex-col px-4 pt-7 pb-3 select-none">
                <div className="flex flex-col mb-2">
                    <span className="text-[12.5px] font-semibold text-white tracking-tight">Bytover Pro</span>
                    <span className="text-[10.5px] text-white/40 mt-0.5">One shelf at a time on Free</span>
                </div>
                <ul className="flex flex-col gap-1.5 flex-1">
                    {features.map((f) => (
                        <li key={f} className="flex items-start gap-1.5">
                            <Check className="w-3 h-3 text-white/70 mt-[3px] shrink-0" strokeWidth={2.5} />
                            <span className="text-[11px] text-white/85 leading-tight">{f}</span>
                        </li>
                    ))}
                </ul>
                <div className="flex flex-col gap-1.5 mt-2">
                    <Button
                        onClick={onUpgrade}
                        className="w-full h-[26px] text-[11px] font-semibold bg-white text-black hover:bg-white/90 border-none rounded-md shadow-none"
                    >
                        Upgrade · $14.89
                    </Button>
                    <button
                        onClick={onClose}
                        className="text-[10.5px] text-white/40 hover:text-white/60 transition-colors"
                    >
                        Not now
                    </button>
                </div>
            </div>
        </ShelfWrapper>
    )
}

export default Window;
