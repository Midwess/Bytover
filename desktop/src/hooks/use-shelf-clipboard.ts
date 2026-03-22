import {useEffect} from "react";
import {invoke} from "@tauri-apps/api/core";

interface UseShelfClipboardOptions {
    shelfId: string | undefined;
    enabled?: boolean;
}

export function useShelfClipboard(options: UseShelfClipboardOptions): void {
    const {shelfId, enabled = true} = options;

    useEffect(() => {
        if (!enabled || !shelfId) return;

        const handlePaste = async (e: ClipboardEvent) => {
            const target = e.target as HTMLElement;
            if (
                target instanceof HTMLInputElement ||
                target instanceof HTMLTextAreaElement ||
                target.isContentEditable
            ) {
                return;
            }

            e.preventDefault();

            try {
                await invoke('paste_from_clipboard', {shelfId});
            } catch (err) {
                console.error('Failed to paste from clipboard:', err);
            }
        };

        window.addEventListener('paste', handlePaste, true);

        return () => {
            window.removeEventListener('paste', handlePaste, true);
        };
    }, [shelfId, enabled]);
}
