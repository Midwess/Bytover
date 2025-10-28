import {Card} from "@/components/ui/card.tsx";
import {getCurrentWindow, PhysicalPosition} from "@tauri-apps/api/window";
import {noop} from "motion";
import {invoke} from "@tauri-apps/api/core";
import {useEffect, useRef} from "react";
import core from "@/core.ts";

export function Shelf() {
    const window = getCurrentWindow()
    const selectedResources = core.useSelectedResources()
    console.log('tiendang-debug', `selected resources`, selectedResources)
    const effectRan = useRef(false);
    useEffect(() => {
        if (effectRan.current) return;

        effectRan.current = true;
        let unlisten: (() => void) | undefined;

        const setup = async () => {
            unlisten = await window.onDragDropEvent(async ({ payload }) => {
                if (payload.type !== "drop") return;

                const windowSize = await window.innerSize();
                const windowPos = await window.outerPosition();
                const dropPosition: PhysicalPosition = (payload as any).position
                if (dropPosition.x < windowPos.x + windowSize.width / 2) {
                    invoke("add_resources", payload).then(noop);
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
        <Card className={"w-full h-full bg-card border-border shadow-md shadow-background border-1"}>

        </Card>
    </>
}