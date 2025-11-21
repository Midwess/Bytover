import { OverlayScrollbars } from "overlayscrollbars";
import { useEffect, useRef } from "react";

export const useOverlayScrollbars = () => {
    const instancesRef = useRef(new Map<HTMLElement, ReturnType<typeof OverlayScrollbars>>());
    const isCleaningUpRef = useRef(false);

    useEffect(() => {
        const instances = instancesRef.current;

        // Clean up instances for elements that are no longer in the DOM
        const cleanupRemovedElements = () => {
            if (isCleaningUpRef.current) return;
            
            const elementsToRemove: HTMLElement[] = [];
            instances.forEach((instance, element) => {
                if (!document.contains(element)) {
                    elementsToRemove.push(element);
                }
            });

            elementsToRemove.forEach(element => {
                const instance = instances.get(element);
                if (instance) {
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
            if (isCleaningUpRef.current) return;
            
            // First, clean up any removed elements
            cleanupRemovedElements();

            // Target the main scrollable areas
            const scrollableElements = [
                document.body,
                document.querySelector('[data-scrollable="true"]'),
                ...document.querySelectorAll(".overflow-y-auto"),
                ...document.querySelectorAll(".overflow-auto"),
                ...document.querySelectorAll(".overflow-y-scroll"),
            ].filter(Boolean) as HTMLElement[];

            scrollableElements.forEach((element) => {
                // Only initialize if not already tracked and still in DOM
                if (!instances.has(element) && document.contains(element) && !isCleaningUpRef.current) {
                    try {
                        const instance = OverlayScrollbars(element, {
                            scrollbars: {
                                theme: "os-theme-custom",
                                autoHide: "never",
                                autoHideDelay: 500,
                            },
                            overflow: {
                                x: "hidden",
                                y: "scroll"
                            }
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
            if (isCleaningUpRef.current) return;
            if (timeoutId) clearTimeout(timeoutId);
            timeoutId = setTimeout(initScrollbars, 100);
        });

        observer.observe(document.body, {
            childList: true,
            subtree: true,
        });

        return () => {
            isCleaningUpRef.current = true;
            observer.disconnect();
            if (timeoutId) clearTimeout(timeoutId);
            
            // Clean up all OverlayScrollbars instances safely
            const allElements = Array.from(instances.keys());
            allElements.forEach((element) => {
                const instance = instances.get(element);
                if (instance) {
                    try {
                        instance.destroy();
                    } catch (error) {
                        // Silently ignore cleanup errors
                    }
                }
            });
            instances.clear();
            isCleaningUpRef.current = false;
        };
    }, []);
};