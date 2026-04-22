import {useEffect, useState} from "react";
import {getCurrentWindow, Window} from "@tauri-apps/api/window";
import {invoke} from "@tauri-apps/api/core";

export type DockEdge = "left" | "right";

export interface ShelfDockState {
    isDocked: boolean;
    edge: DockEdge | null;
    expand: () => void;
}

export default function useShelfDock(onWindow?: Window): ShelfDockState {
    const window = onWindow || getCurrentWindow();
    const [isDocked, setIsDocked] = useState(false);
    const [edge, setEdge] = useState<DockEdge | null>(null);

    useEffect(() => {
        let unlistenDocked: (() => void) | undefined;
        let unlistenExpanded: (() => void) | undefined;

        const setup = async () => {
            unlistenDocked = await window.listen<{ edge: DockEdge }>("shelf-docked", (event) => {
                setIsDocked(true);
                setEdge(event.payload.edge);
            });
            unlistenExpanded = await window.listen("shelf-expanded", () => {
                setIsDocked(false);
                setEdge(null);
            });
        };

        setup();

        return () => {
            unlistenDocked?.();
            unlistenExpanded?.();
        };
    }, [window]);

    const expand = () => {
        invoke("expand_shelf", {label: window.label}).catch(() => {});
    };

    return {isDocked, edge, expand};
}
