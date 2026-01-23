import {Card} from "@/components/ui/card.tsx";
import {getCurrentWindow, PhysicalPosition} from "@tauri-apps/api/window";
import {startDrag} from "@crabnebula/tauri-plugin-drag";
import {noop} from "motion";
import {invoke} from "@tauri-apps/api/core";
import {convertFileSrc} from "@tauri-apps/api/core";
import {useEffect, useRef, useState, ReactNode} from "react";
import {useShelfClipboard} from "@/hooks/use-shelf-clipboard.ts";
import core from "@/core.ts";
import {
    Play,
    FolderIcon,
    FileIcon,
    MoreHorizontal,
    MoreVertical,
    Trash2,
    Minus,
    Plus,
    X,
    Loader2,
    ClipboardPaste,
    ExternalLink,
} from "lucide-react";
import {Button} from "@/components/ui/button.tsx";
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
import {formatFileSize} from "@/utils/format-file-size";
import useWindow from "@/hooks/use-window.ts";
import {throttle} from "lodash";
import {UnlimitedLineText} from "@/components/ui/unlimited-line-text";

function ShelfWrapper({children, isDraggingOver = false, shelfName}: {
    children: ReactNode,
    isDraggingOver?: boolean,
    shelfName?: string
}) {
    const handleClose = () => {
        getCurrentWindow()?.close()
    }

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
                className="flex flex-col absolute top-0 left-0 right-0 h-5 bg-gradient-to-b from-card to-transparent pointer-events-none z-20"/>
            <div data-tauri-drag-region
                 className={"w-full py-1 absolute top-0 flex justify-center items-center z-[60] peer group flex-col cursor-pointer"}>
               <Minus
                    className={"pointer-events-none scale-x-200 scale-y-200 text-primary transition-transform duration-200 group-hover:scale-x-[3] group-hover:scale-y-[2.5]"}/>
            </div>
            {/* Close button - rotated rectangle with X at center-left, visible on header hover */}
            <button
                onClick={handleClose}
                className="hover:cursor-pointer absolute -top-0 -right-4.5 w-20 h-4.5 bg-amber-500/50 rounded-xl z-100 rotate-45 flex items-center justify-start pl-10 transition-all group z-50 -pb-5.5 opacity-0 peer-hover:opacity-100 hover:opacity-100 rounded-2xl"
            >
                <Minus className="w-4 h-4.5 scale-y-180 text-lg font-bold text-amber-200 -rotate-45">-</Minus>
            </button>
            {children}
        </Card>
    )
}

export function Shelf({shelfId}: { shelfId: string | undefined }) {
    const window = getCurrentWindow()
    const windowInfo = useWindow(window)
    const selectedResources = core.useSelectedResourcesForShelf(shelfId)
    const currentShelf = core.useCurrentShelf(shelfId)
    const isResourceRemoveAllowed = currentShelf?.is_resource_remove_allowed ?? true
    const effectRan = useRef(false);
    const [isDraggingOver, setIsDraggingOver] = useState(false);
    const containerRef = useRef<HTMLDivElement>(null);

    useShelfClipboard({shelfId, containerRef});

    useEffect(() => {
        if (effectRan.current || !shelfId) return;

        effectRan.current = true;
        let unlisten: (() => void) | undefined;

        const setup = async () => {
            unlisten = await window.onDragDropEvent(throttle((event) => {
                const {payload} = event
                const eventPosition: PhysicalPosition | undefined = (payload as any)?.position
                const isLeftSide = eventPosition?.x !== undefined && eventPosition.x < windowInfo.position.x + windowInfo.size.width / 2;
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

                    if (isLeftSide && payload.paths.length > 0) {
                        invoke("add_resources", {shelfId, paths: payload.paths}).then(noop);
                    }
                }
            }, 50, {leading: true, trailing: true}));
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
                <Loader2 className="h-6 w-6 text-foreground animate-spin"/>
            </ShelfWrapper>
        )
    }

    return (
        <ShelfWrapper isDraggingOver={isDraggingOver} shelfName={currentShelf?.name}>
            <div
                ref={containerRef}
                tabIndex={0}
                className="w-full h-full outline-none"
            >
            <div
                className={`absolute z-40 inset-0 bg-bluePrimary/10 backdrop-blur-[3px] flex items-center justify-center animate-in fade-in duration-200 ${!isDraggingOver && 'hidden'}`}>
                <div className="flex flex-col items-center w-full gap-2 text-primary">
                    <Plus className="h-9 w-10 text-bluePrimary"/>
                </div>
            </div>
            {/* Resources List */}
            <div
                className="w-full h-full overflow-y-auto px-2.5 z-0 pt-9 shadow-[inset_0_20px_20px_-10px_hsl(var(--card)),inset_0_-20px_20px_-10px_hsl(var(--card))]">
                {selectedResources.length === 0 ? (
                    <div
                        className="flex flex-col items-center justify-center h-full text-muted-foreground gap-2 absolute left-0 top-0 w-full">
                        <p className="text-md text-muted-foreground animate-pulse duration-1500">Drop or paste files here</p>
                    </div>
                ) : (
                    <div className="flex flex-col gap-2">
                        {selectedResources.map((resource) => (
                            <ResourceView
                                key={resource.order_id}
                                model={resource}
                                isRemoveAllowed={isResourceRemoveAllowed}
                                onRemove={(resourceId) => {
                                    invoke("remove_resource", {shelfId, resourceId})
                                }}
                                onOpen={(resourceId) => {
                                    invoke("open_shelf_resource", {shelfId, resourceId})
                                }}
                            />
                        ))}
                        <div className={"h-5"}></div>
                    </div>
                )}
            </div>

            <div
                className="absolute bottom-0 left-0 right-0 h-fit bg-gradient-to-t from-card to-transparent z-20 w-full justify-center flex flex-row pb-2">
                <div className="group z-20 flex flex-col items-center justify-end bg-transparent text-muted-foreground transition-all duration-500 ease-out hover:pb-2 gap-2">
                    <div className="flex flex-col gap-1.5 overflow-hidden max-h-0 opacity-0 transition-all duration-300 ease-out group-hover:max-h-24 group-hover:opacity-100 group-hover:mb-1">
                        {!!selectedResources.length && (
                            <Button
                                variant="ghost"
                                size="sm"
                                disabled={!isResourceRemoveAllowed}
                                onClick={() => invoke("clear_shelf", {shelfId})}
                                className="w-24 flex items-center justify-center gap-1.5 text-foreground text-xs bg-muted px-2 py-1 h-auto rounded-lg border disabled:opacity-50 disabled:cursor-not-allowed">
                                <Trash2 className="h-3.5 w-3.5"/>
                                <span>Clear all</span>
                            </Button>
                        )}
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => invoke('paste_from_clipboard', {shelfId})}
                            className="flex items-center justify-center gap-1.5 text-foreground text-xs bg-muted px-2 py-1 h-auto w-24 rounded-lg border">
                            <ClipboardPaste className="h-3.5 w-3.5"/>
                            <span>Paste</span>
                        </Button>
                    </div>
                    <MoreHorizontal
                        className="h-7 w-7 flex-shrink-0 transition-transform text-foreground p-[2px] duration-500 ease-out bg-muted/90 rounded-full cursor-pointer"/>
                </div>
            </div>
            </div>
        </ShelfWrapper>
    )
}

function ResourceView(props: {
    model: SelectedResourceViewModel,
    isRemoveAllowed: boolean,
    onRemove: (resourceId: string) => void,
    onOpen: (resourceId: string) => void
}) {
    const {model, isRemoveAllowed, onRemove, onOpen} = props;
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
                ? <FileView model={model} isRemoveAllowed={isRemoveAllowed} onRemove={onRemove} onOpen={onOpen}/>
                : <MediaView model={model} isRemoveAllowed={isRemoveAllowed} onRemove={onRemove} onOpen={onOpen}/>
        }
    </div>
}

function FileView(props: {
    model: SelectedResourceViewModel,
    isRemoveAllowed: boolean,
    onRemove: (resourceId: string) => void,
    onOpen: (resourceId: string) => void
}) {
    const {model, isRemoveAllowed, onRemove, onOpen} = props;

    let thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;
    const isFolder = model.type instanceof ResourceTypeVariantFolder;

    // Convert absolute path to Tauri asset URL
    const thumbnailUrl = thumbnailPath ? convertFileSrc(thumbnailPath) : null;

    const displaySize = formatFileSize(model);

    return (
        <Card
            shadowSize={0.35}
            onDoubleClick={() => onOpen(model.order_id)}
            className="w-full border bg-muted rounded-xl flex flex-row hover:bg-muted-foreground/30 items-center gap-3 p-1 relative group transition-colors cursor-pointer">
            {/* Thumbnail */}
            <div className="w-12 h-12 shrink-0 rounded-lg bg-muted-foreground/15 p-1 overflow-hidden relative">
                {thumbnailUrl ? (
                    <img
                        src={thumbnailUrl} alt={model.name}
                        className="w-full h-full object-cover rounded-md overflow-hidden"/>
                ) : isFolder ? (
                    <FolderIcon className="w-6 h-6 text-primary"/>
                ) : (
                    <FileIcon className="w-6 h-6 text-primary"/>
                )}
            </div>

            {/* Info */}
            <div className="flex-1 min-w-0">
                <UnlimitedLineText
                    text={model.name}
                    className="text-sm font-medium text-primaryText"
                    startChars={8}
                    endChars={6}
                    speed={30}
                />
                <p className="text-xs text-primaryText/70">{displaySize}</p>
            </div>

            {/* Dropdown Menu */}
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button variant="ghost" className="p-0">
                        <MoreVertical className="w-4 h-4"/>
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="dark">
                    <DropdownMenuItem onClick={() => onOpen(model.order_id)}>
                        <ExternalLink className="w-4 h-4 mr-2"/>
                        Open
                    </DropdownMenuItem>
                    <DropdownMenuItem
                        variant="destructive"
                        disabled={!isRemoveAllowed}
                        onClick={() => onRemove(model.order_id)}>
                        <Trash2 className="w-4 h-4 mr-2"/>
                        Remove
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        </Card>
    );
}

function MediaView(props: {
    model: SelectedResourceViewModel,
    isRemoveAllowed: boolean,
    onRemove: (resourceId: string) => void,
    onOpen: (resourceId: string) => void
}) {
    const {model, isRemoveAllowed, onRemove, onOpen} = props;

    const isVideo = (model.type as any) === 'Video';
    const thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;

    // Convert absolute path to Tauri asset URL
    const thumbnailUrl = thumbnailPath ? convertFileSrc(thumbnailPath) : null;

    const displaySize = formatFileSize(model);

    return (
        <Card
            shadowSize={0.35}
            onDoubleClick={() => onOpen(model.order_id)}
            className="border-1 w-full bg-muted rounded-xl flex hover:bg-muted-foreground/30 flex-row items-center gap-3 p-1 relative group transition-colors cursor-pointer">
            {/* Thumbnail */}
            <div className="w-12 h-12 flex-shrink-0 rounded-lg bg-muted-foreground/15 p-1 overflow-hidden relative">
                {thumbnailUrl ? (
                    <img src={thumbnailUrl} alt={model.name}
                         className="w-full h-full object-cover rounded-md overflow-clip"/>
                ) : (
                    <FileIcon
                        className="w-6 h-6 text-primary absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2"/>
                )}
                {isVideo && (
                    <div className="absolute top-1.5 right-1.5">
                        <Play className="w-3 h-3 text-white bg-black/50 rounded-md p-0.5"/>
                    </div>
                )}
            </div>

            {/* Info */}
            <div className="flex-1 min-w-0">
                <UnlimitedLineText
                    text={model.name}
                    className="text-xs font-medium text-primaryText"
                    startChars={8}
                    endChars={6}
                    speed={30}
                />
                <p className="text-xs text-primaryText/70">{displaySize}</p>
            </div>

            {/* Dropdown Menu */}
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button variant="ghost" className="p-0">
                        <MoreVertical className="w-4 h-4"/>
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className={"dark"}>
                    <DropdownMenuItem onClick={() => onOpen(model.order_id)}>
                        <ExternalLink className="w-4 h-4 mr-2"/>
                        Open
                    </DropdownMenuItem>
                    <DropdownMenuItem
                        variant="destructive"
                        disabled={!isRemoveAllowed}
                        onClick={() => onRemove(model.order_id)}>
                        <Trash2 className="w-4 h-4 mr-2"/>
                        Remove
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        </Card>
    );
}
