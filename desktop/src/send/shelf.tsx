import { Card } from "@/components/ui/card.tsx";
import { getCurrentWindow, PhysicalPosition } from "@tauri-apps/api/window";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { noop } from "motion";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState, ReactNode } from "react";
import core from "@/core.ts";
import {
    Play,
    FolderIcon,
    FileIcon,
    MoreVertical,
    Trash2,
    Minus,
    Plus,
    X,
    Loader2,
} from "lucide-react";
import { Button } from "@/components/ui/button.tsx";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@/components/animate-ui/components/radix/dropdown-menu.tsx";
import {
    ResourceTypeVariantFolder,
    SelectedResourceViewModel,
} from "shared_types/types/shared_types";
import useWindow from "@/hooks/use-window.ts";
import { throttle } from "lodash";

function ShelfWrapper({ children, isDraggingOver = false }: { children: ReactNode, isDraggingOver?: boolean }) {
    return (
        <Card
            shadowSize={0.0}
            className={`
                rounded-[30px]
                bg-card
                flex flex-col
                justify-center
                items-center
                px-0
                w-full h-full border-2
                transition-all duration-200 relative overflow-hidden
                ${isDraggingOver
                    ? 'border-bluePrimary shadow-[0_0_8px_2px_rgb(var(--bluePrimary))_inset]'
                    : 'border-white/20'
                }
            `}>
            <div
                className="absolute top-0 left-0 right-0 h-5 bg-gradient-to-b from-card to-transparent pointer-events-none z-20" />
            <div data-tauri-drag-region
                onDoubleClick={() => {
                    getCurrentWindow()?.close()
                }}
                className={"w-full py-1 absolute top-0 flex justify-center items-center z-30 group"}>
                <Minus
                    className={"pointer-events-none scale-x-200 scale-y-200 text-primary transition-transform duration-200 group-hover:scale-x-[3] group-hover:scale-y-[2.5]"} />
            </div>
            {children}
        </Card>
    )
}

export function Shelf({ shelfId }: { shelfId: string | undefined }) {
    const window = getCurrentWindow()
    const windowInfo = useWindow(window)
    const selectedResources = core.useSelectedResourcesForShelf(shelfId)
    const isResourceRemoveAllowed = core.useTransferState()?.is_resource_remove_allowed ?? true
    const effectRan = useRef(false);
    const [isDraggingOver, setIsDraggingOver] = useState(false);

    useEffect(() => {
        if (effectRan.current || !shelfId) return;

        effectRan.current = true;
        let unlisten: (() => void) | undefined;

        const setup = async () => {
            unlisten = await window.onDragDropEvent(throttle(({ payload }) => {
                const eventPosition: PhysicalPosition | undefined = (payload as any)?.position
                console.log(eventPosition)
                const isLeftSide = eventPosition?.x && eventPosition.x < windowInfo.position.x + windowInfo.size.width / 2;
                if (payload.type === "over") {
                    if (isLeftSide) {
                        setIsDraggingOver(true);
                    } else {
                        setIsDraggingOver(false);
                    }
                } else if (payload.type === "leave") {
                    setIsDraggingOver(false);
                } else if (payload.type === "drop") {
                    setIsDraggingOver(false);

                    if (isLeftSide) {
                        invoke("add_resources", { shelfId, paths: payload.paths }).then(noop);
                    }
                }
            }, 120, { leading: true, trailing: true }));
        };

        setup();

        return () => {
            if (unlisten) {
                unlisten();
            }
        };
    }, [windowInfo, shelfId]);

    if (!shelfId) {
        return (
            <ShelfWrapper>
                <Loader2 className="h-6 w-6 text-foreground animate-spin" />
            </ShelfWrapper>
        )
    }

    return (
        <ShelfWrapper isDraggingOver={isDraggingOver}>
            <div
                className={`absolute z-40 inset-0 bg-bluePrimary/10 backdrop-blur-[3px] flex items-center justify-center animate-in fade-in duration-200 ${!isDraggingOver && 'hidden'}`}>
                <div className="flex flex-col items-center w-full gap-2 text-primary">
                    <Plus className="h-10 w-10 text-bluePrimary" />
                </div>
            </div>
            {/* Resources List */}
            <div
                className="w-full h-full overflow-y-auto px-2.5 z-0 pt-9 shadow-[inset_0_20px_20px_-10px_hsl(var(--card)),inset_0_-20px_20px_-10px_hsl(var(--card))]">
                {selectedResources.length === 0 ? (
                    <div
                        className="flex flex-col items-center justify-center h-full text-muted-foreground gap-2 absolute left-0 top-0 w-full">
                        <p className="text-md text-muted-foreground animate-pulse duration-1500">Drop files here</p>
                    </div>
                ) : (
                    <div className="flex flex-col gap-2">
                        {selectedResources.map((resource, index) => (
                            <ResourceView
                                key={index}
                                model={resource}
                                isRemoveAllowed={isResourceRemoveAllowed}
                                onRemove={(resourceId) => {
                                    invoke("remove_resource", { shelfId, resourceId })
                                }}
                            />
                        ))}
                        <div className={"h-5"}></div>
                    </div>
                )}
            </div>

            <div
                className="absolute bottom-0 left-0 right-0 h-fit bg-gradient-to-t from-card to-transparent z-20 w-full justify-center flex flex-row pb-3">
                {!!selectedResources.length &&
                    <Button
                        disabled={!isResourceRemoveAllowed}
                        onClick={() => {
                            invoke("clear_shelf", { shelfId })
                        }}
                        className="group z-20 flex-col items-center justify-between border-none overflow-hidden w-fit border rounded-full bg-transparent text-muted-foreground transition-all duration-500 ease-out hover:h-18 hover:py-2 hover:rounded-2xl disabled:opacity-50 disabled:cursor-not-allowed">
                        <div
                            className="overflow-hidden text-foreground bg-muted px-1 rounded-md opacity-0 transition-all duration-100 ease-out group-hover:opacity-100 group-hover:mt-1 border border-border">
                            <p>
                                Clear all
                            </p>
                        </div>
                        <X
                            className="h-8 w-8 scale-125 flex-shrink-0 transition-transform text-foreground border border-foreground/80 p-[2px] duration-500 ease-out group-hover:rotate-95 bg-muted/90 rounded-full" />
                    </Button>
                }
            </div>
        </ShelfWrapper>
    )
}

function ResourceView(props: { model: SelectedResourceViewModel, isRemoveAllowed: boolean, onRemove: (resourceId: string) => void }) {
    const { model, isRemoveAllowed, onRemove } = props;
    let filePath = (model.path as any).AbsolutePath;
    let thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;

    const isFile = ['Folder', 'File'].includes(model.type as any);

    return <div
        draggable={true}
        onDragStart={async (e) => {
            e.preventDefault()
            await startDrag({
                item: [filePath],
                icon: thumbnailPath,
            }, console.log)
        }}>
        {
            isFile
                ? <FileView model={model} isRemoveAllowed={isRemoveAllowed} onRemove={onRemove} />
                : <MediaView model={model} isRemoveAllowed={isRemoveAllowed} onRemove={onRemove} />
        }
    </div>
}

function FileView(props: { model: SelectedResourceViewModel, isRemoveAllowed: boolean, onRemove: (resourceId: string) => void }) {
    const { model, isRemoveAllowed, onRemove } = props;

    let thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;
    const isFolder = model.type instanceof ResourceTypeVariantFolder;

    // Convert absolute path to Tauri asset URL
    const thumbnailUrl = thumbnailPath ? convertFileSrc(thumbnailPath) : null;

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <Card
            shadowSize={0.35}
            className="w-full border-1 bg-muted rounded-xl flex flex-row hover:bg-muted-foreground/30 items-center gap-3 p-1 relative group transition-colors">
            {/* Thumbnail */}
            <div className="w-12 h-12 flex-shrink-0 rounded-lg bg-muted-foreground/15 p-1 overflow-hidden relative">
                {thumbnailUrl ? (
                    <img
                        src={thumbnailUrl} alt={model.name}
                        className="w-full h-full object-cover rounded-md overflow-hidden" />
                ) : isFolder ? (
                    <FolderIcon className="w-6 h-6 text-primary" />
                ) : (
                    <FileIcon className="w-6 h-6 text-primary" />
                )}
            </div>

            {/* Info */}
            <div className="flex-1 min-w-0">
                <p className="text-sm font-medium text-primaryText truncate">{model.name}</p>
                <p className="text-xs text-primaryText/70">{displaySize}</p>
            </div>

            {/* Dropdown Menu */}
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button variant="ghost" className="p-0">
                        <MoreVertical className="w-4 h-4" />
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="dark">
                    <DropdownMenuItem
                        variant="destructive"
                        disabled={!isRemoveAllowed}
                        onClick={() => onRemove(model.order_id)}>
                        <Trash2 className="w-4 h-4 mr-2" />
                        Remove
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        </Card>
    );
}

function MediaView(props: { model: SelectedResourceViewModel, isRemoveAllowed: boolean, onRemove: (resourceId: string) => void }) {
    const { model, isRemoveAllowed, onRemove } = props;

    const isVideo = (model.type as any) === 'Video';
    const thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;

    // Convert absolute path to Tauri asset URL
    const thumbnailUrl = thumbnailPath ? convertFileSrc(thumbnailPath) : null;

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <Card
            shadowSize={0.35}
            className="border-1 w-full bg-muted rounded-xl flex hover:bg-muted-foreground/30 flex-row items-center gap-3 p-1 relative group transition-colors">
            {/* Thumbnail */}
            <div className="w-12 h-12 flex-shrink-0 rounded-lg bg-muted-foreground/15 p-1 overflow-hidden relative">
                {thumbnailUrl ? (
                    <img src={thumbnailUrl} alt={model.name}
                        className="w-full h-full object-cover rounded-md overflow-clip" />
                ) : (
                    <FileIcon
                        className="w-6 h-6 text-primary absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2" />
                )}
                {isVideo && (
                    <div className="absolute top-1.5 right-1.5">
                        <Play className="w-3 h-3 text-white bg-black/50 rounded-md p-0.5" />
                    </div>
                )}
            </div>

            {/* Info */}
            <div className="flex-1 min-w-0">
                <p className="text-xs font-medium text-primaryText truncate">{model.name}</p>
                <p className="text-xs text-primaryText/70">{displaySize}</p>
            </div>

            {/* Dropdown Menu */}
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button variant="ghost" className="p-0">
                        <MoreVertical className="w-4 h-4" />
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className={"dark"}>
                    <DropdownMenuItem
                        variant="destructive"
                        disabled={!isRemoveAllowed}
                        onClick={() => onRemove(model.order_id)}>
                        <Trash2 className="w-4 h-4 mr-2" />
                        Remove
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        </Card>
    );
}
