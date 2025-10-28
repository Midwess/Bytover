import {Card} from "@/components/ui/card.tsx";
import {getCurrentWindow, PhysicalPosition} from "@tauri-apps/api/window";
import {noop} from "motion";
import {invoke} from "@tauri-apps/api/core";
import {useEffect, useRef, useState} from "react";
import core from "@/core.ts";
import {Upload} from "lucide-react";

export function Shelf() {
    const window = getCurrentWindow()
    const selectedResources = core.useSelectedResources()
    const effectRan = useRef(false);
    const [isDraggingOver, setIsDraggingOver] = useState(false);
    
    useEffect(() => {
        if (effectRan.current) return;

        effectRan.current = true;
        let unlisten: (() => void) | undefined;

        const setup = async () => {
            unlisten = await window.onDragDropEvent(async ({ payload }) => {
                const windowSize = await window.innerSize();
                const eventPosition: PhysicalPosition | undefined = (payload as any)?.position
                const windowPos = await window.outerPosition();
                const isLeftSide = eventPosition?.x && eventPosition.x < windowPos.x + windowSize.width / 2;
                if (payload.type === "over") {
                    // Show drag feedback when hovering over the left side (shelf area)
                    if (isLeftSide) {
                        setIsDraggingOver(true);
                    } else {
                        setIsDraggingOver(false);
                    }
                } else if (payload.type === "leave") {
                    // Hide drag feedback when leaving
                    setIsDraggingOver(false);
                } else if (payload.type === "drop") {
                    // Hide drag feedback and handle drop
                    setIsDraggingOver(false);
                    
                    if (isLeftSide) {
                        invoke("add_resources", payload).then(noop);
                    }
                }
            });
        };

        setup();

        return () => {
            if (unlisten) {
                unlisten();
            }
        };
    }, []);

    return <>
        <Card className={`
            w-full h-full bg-card shadow-md shadow-background border-1 
            transition-all duration-200 relative overflow-hidden
            ${isDraggingOver 
                ? 'border-bluePrimary border-2 shadow-lg shadow-bluePrimary/20' 
                : 'border-border'
            }
        `}>
            {/* Drag overlay */}
            {isDraggingOver && (
                <div className="absolute inset-0 hover:bg-bluePrimary/10 backdrop-blur-[1px] flex items-center justify-center z-10 animate-in fade-in duration-200">
                    <div className="flex flex-col items-center gap-2 text-primary">
                        <Upload className="h-12 w-12 animate-bounce opacity-80" />
                        <span className="text-sm font-bold">Drop files here</span>
                    </div>
                </div>
            )}
        </Card>
    </>
}