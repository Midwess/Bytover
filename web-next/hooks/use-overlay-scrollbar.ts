import { OverlayScrollbars } from "overlayscrollbars";
import { useEffect } from "react";

export const useOverlayScrollbars = () => {
    useEffect(() => {
        const instances = new Map<HTMLElement, ReturnType<typeof OverlayScrollbars>>();

        // Clean up instances for elements that are no longer in the DOM
        const cleanupRemovedElements = () => {
            instances.forEach((instance, element) => {
                if (!document.contains(element)) {
                    try {
                        instance.destroy();
                    } catch (error) {
                        // Silently ignore
                    }
                    instances.delete(element);
                }
            });
        };

        // Initialize OverlayScrollbars on all scrollable elements
        const initScrollbars = () => {
            // First, clean up any removed elements
            cleanupRemovedElements();

            // Target the main scrollable areas
            const scrollableElements = [
                document.body,
                document.querySelector('[data-scrollable="true"]'),
                ...document.querySelectorAll(".overflow-y-auto"),
                ...document.querySelectorAll(".overflow-auto"),
                ...document.querySelectorAll(".overflow-scroll"),
                ...document.querySelectorAll(".overflow-y-scroll"),
            ].filter(Boolean) as HTMLElement[];

            scrollableElements.forEach((element) => {
                // Only initialize if not already tracked and still in DOM
                if (!instances.has(element) && document.contains(element)) {
                    try {
                        const instance = OverlayScrollbars(element, {
                            scrollbars: {
                                theme: "os-theme-custom",
                                autoHide: "never",
                            },
                        });
                        instances.set(element, instance);
                    } catch (error) {
                        console.warn("Error initializing OverlayScrollbars:", error);
                    }
                }
            });
        };

        // Initialize immediately
        initScrollbars();

        // Re-initialize when DOM changes (for dynamic content)
        let timeoutId: ReturnType<typeof setTimeout> | undefined;
        const observer = new MutationObserver(() => {
            if (timeoutId) clearTimeout(timeoutId);
            timeoutId = setTimeout(initScrollbars, 100);
        });

        observer.observe(document.body, {
            childList: true,
            subtree: true,
        });

        return () => {
            observer.disconnect();
            if (timeoutId) clearTimeout(timeoutId);
            
            // Clean up all OverlayScrollbars instances safely
            instances.forEach((instance, element) => {
                try {
                    if (document.contains(element)) {
                        instance.destroy();
                    }
                } catch (error) {
                    // Silently ignore cleanup errors
                }
            });
            instances.clear();
        };
    }, []);
};