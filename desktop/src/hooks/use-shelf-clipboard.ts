import {useEffect, RefObject} from "react";
import {invoke} from "@tauri-apps/api/core";

interface UseShelfClipboardOptions {
    shelfId: string | undefined;
    containerRef: RefObject<HTMLElement | null>;
    enabled?: boolean;
}

export function useShelfClipboard(options: UseShelfClipboardOptions): void {
    const {shelfId, containerRef, enabled = true} = options;

    useEffect(() => {
        if (!enabled || !shelfId || !containerRef.current) return;

        const handlePaste = async (e: ClipboardEvent) => {
            e.preventDefault();

            if (!shelfId) return;

            try {
                await invoke('paste_from_clipboard', {shelfId});
            } catch (err) {
                console.error('Failed to paste from clipboard:', err);
            }
        };

        const element = containerRef.current;
        element.addEventListener('paste', handlePaste);

        return () => {
            element.removeEventListener('paste', handlePaste);
        };
    }, [shelfId, enabled, containerRef]);
}
