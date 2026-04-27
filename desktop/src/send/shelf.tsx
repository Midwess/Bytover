import {Card} from "@/components/ui/card.tsx";
import {getCurrentWindow, PhysicalPosition} from "@tauri-apps/api/window";
import {startDrag} from "@crabnebula/tauri-plugin-drag";
import {noop} from "motion";
import {motion, AnimatePresence} from "motion/react";
import {invoke} from "@tauri-apps/api/core";
import {convertFileSrc} from "@tauri-apps/api/core";
import {useEffect, useRef, useState, ReactNode} from "react";
import {createPortal} from "react-dom";
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
    Loader2,
    ClipboardPaste,
    ExternalLink,
    Layers,
    LayoutList,
} from "lucide-react";
import {Button} from "@/components/ui/button.tsx";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu.tsx";
import {
    ResourceTypeVariantFolder,
    SelectedResourceViewModel,
} from "shared_types/types/shared_types";
import {formatFileSize} from "@/utils/format-file-size";
import useWindow from "@/hooks/use-window.ts";
import useShelfDock, {DockEdge} from "@/hooks/use-shelf-dock.ts";
import {throttle} from "lodash";
import {UnlimitedLineText} from "@/components/ui/unlimited-line-text";
import {PeerAvatarGroup} from "@/send/peer-avatar-group";
import {StackView} from "@/send/stack-view";

export type ShelfViewMode = 'list' | 'stack';

export function ShelfWrapper({
    children,
    isDraggingOver = false,
    shelfName,
    isDocked = false,
    dockEdge = null,
    onExpand,
    progress = 0,
    progressEdge = null,
    isOnline = false,
    isCollapsed = false,
}: {
    children: ReactNode,
    isDraggingOver?: boolean,
    shelfName?: string,
    isDocked?: boolean,
    dockEdge?: DockEdge | null,
    onExpand?: () => void,
    progress?: number,
    progressEdge?: DockEdge | null,
    isOnline?: boolean,
    isCollapsed?: boolean,
}) {
    const handleClose = () => {
        getCurrentWindow()?.close()
    }

    const activeEdge = dockEdge ?? progressEdge;
    const clampedProgress = Math.min(Math.max(progress, 0), 1);
    const state2Opacity = clampedProgress;
    const nameRotation = activeEdge === "left" ? "rotate(180deg)" : undefined;
    const innerEdgePos = activeEdge === "right" ? "left-0" : "right-0";

    return (
        <>
            <Card
                className={`
                    rounded-[30px]
                    justify-center
                    items-center
                    px-0
                    w-full h-full
                    relative overflow-hidden
                    animate-in fade-in duration-300
                    transition-[border-radius,box-shadow,border-color] duration-200
                    ${isDraggingOver
                    ? 'border-bluePrimary shadow-[0_0_8px_2px_rgb(var(--bluePrimary))_inset]'
                    : 'border-white/20'
                }
                `}>
                <div className="absolute inset-0">
                    <div
                        className="flex flex-col absolute top-0 left-0 right-0 h-5 bg-gradient-to-b from-card to-transparent pointer-events-none z-20"/>
                    <div data-tauri-drag-region
                         className={"w-full py-1 absolute top-0 flex justify-center items-center z-[60] peer group flex-col cursor-pointer"}>
                       <Minus
                            className={"pointer-events-none scale-x-200 scale-y-200 text-primary transition-transform duration-200 group-hover:scale-x-[3] group-hover:scale-y-[2.5]"}/>
                    </div>
                    <button
                        onClick={handleClose}
                        className="hover:cursor-pointer absolute -top-0 -right-4.5 w-20 h-4.5 bg-muted-foreground/10 rounded-xl z-100 rotate-45 flex items-center justify-start pl-10 transition-all group z-50 -pb-5.5 opacity-0 peer-hover:opacity-100 hover:opacity-100 rounded-2xl"
                    >
                        <Minus className="w-4 h-4.5 scale-y-180 text-lg font-bold text-foreground -rotate-45"></Minus>
                    </button>
                    {children}
                </div>
            </Card>

            {activeEdge && createPortal(
                <button
                    onClick={onExpand}
                    disabled={!isDocked}
                    aria-label="Expand shelf"
                    className={`dark group fixed top-[7.5px] left-0 right-0 h-[230px] z-[200] p-0 ${isDocked ? "rounded-[26px]" : "rounded-[30px]"} border-2 border-white/20 bg-card text-card-foreground overflow-hidden cursor-pointer transition-[opacity,background-color,border-radius] duration-200 ease-out hover:bg-muted disabled:cursor-default`}
                    style={{
                        opacity: state2Opacity,
                        pointerEvents: isDocked ? "auto" : "none",
                    }}
                >
                    <div
                        className={`absolute top-0 bottom-0 w-[36px] ${innerEdgePos} overflow-hidden`}
                    >
                        <span className="absolute top-3 left-1/2 -translate-x-1/2 h-4 flex items-center justify-center">
                            {isOnline && (
                                <span
                                    className="w-2 h-2 rounded-full"
                                    style={{
                                        backgroundColor: "var(--color-greenSecondary)",
                                        boxShadow: "0 0 6px var(--color-greenSecondary)",
                                    }}
                                />
                            )}
                        </span>
                        <span className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 flex items-center justify-center">
                            <Minus className="pointer-events-none rotate-90 scale-x-200 scale-y-200 text-primary transition-transform duration-200 group-hover:scale-x-[3] group-hover:scale-y-[2.5]"/>
                        </span>
                        <div className="absolute bottom-3 left-1/2 -translate-x-1/2 max-h-[calc(50%-2rem)] flex items-end justify-center overflow-hidden">
                            {shelfName && (
                                <span
                                    className="text-[10px] font-medium text-foreground/60 group-hover:text-foreground transition-opacity duration-150 opacity-70 group-hover:opacity-100 whitespace-nowrap select-none tracking-wide"
                                    style={{writingMode: "vertical-rl", transform: nameRotation}}
                                >
                                    {shelfName}
                                </span>
                            )}
                        </div>
                    </div>
                </button>,
                document.body
            )}
        </>
    )
}

export function Shelf({
    shelfId,
    isCollapsed = false,
    disabled = false,
    overlay,
}: {
    shelfId: string | undefined,
    isCollapsed?: boolean,
    disabled?: boolean,
    overlay?: ReactNode,
}) {
    const window = getCurrentWindow()
    const windowInfo = useWindow(window)
    const dock = useShelfDock(window)
    const selectedResources = core.useSelectedResourcesForShelf(shelfId)
    const currentShelf = core.useCurrentShelf(shelfId)
    const p2pSession = core.useP2PSessionForShelf(shelfId)
    const isOnline = !!p2pSession
    const isResourceRemoveAllowed = currentShelf?.is_resource_remove_allowed ?? true
    const effectRan = useRef(false);
    const [isDraggingOver, setIsDraggingOver] = useState(false);
    const [viewMode, setViewMode] = useState<ShelfViewMode>('stack');
    const containerRef = useRef<HTMLDivElement>(null);

    useShelfClipboard({shelfId, enabled: !disabled});

    useEffect(() => {
        if (shelfId && containerRef.current) {
            containerRef.current.focus();
        }
    }, [shelfId]);

    useEffect(() => {
        if (effectRan.current || !shelfId) return;

        effectRan.current = true;
        let unlisten: (() => void) | undefined;

        const setup = async () => {
            unlisten = await window.onDragDropEvent((event) => {
                const {payload} = event
                console.log('[dragdrop]', event)
                console.log('[dragdrop]', payload)
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

                    if (isLeftSide) {
                        if (payload.paths.length > 0) {
                            invoke("add_resources", {shelfId, paths: payload.paths}).then(noop);
                        } else {
                            invoke("add_resources_from_drag_pasteboard", {shelfId}).then(noop);
                        }
                    }
                }
            })
        };

        setup();

        return () => {
            if (unlisten) {
                unlisten();
            }
        };
    }, [windowInfo, shelfId]);

    useEffect(() => {
        if (!containerRef.current || !shelfId) return;

        const container = containerRef.current;

        const onDragEnter = (e: DragEvent) => {
            e.preventDefault();
            e.stopPropagation();
            setIsDraggingOver(true);
        };

        const onDragOver = (e: DragEvent) => {
            e.preventDefault();
            e.stopPropagation();
        };

        const onDragLeave = (e: DragEvent) => {
            e.preventDefault();
            e.stopPropagation();
            if (!container.contains(e.relatedTarget as Node)) {
                setIsDraggingOver(false);
            }
        };

        const onDrop = (e: DragEvent) => {
            e.preventDefault();
            e.stopPropagation();
            setIsDraggingOver(false);
            invoke("add_resources_from_drag_pasteboard", {shelfId}).then(noop);
        };

        container.addEventListener("dragenter", onDragEnter);
        container.addEventListener("dragover", onDragOver);
        container.addEventListener("dragleave", onDragLeave);
        container.addEventListener("drop", onDrop);

        return () => {
            container.removeEventListener("dragenter", onDragEnter);
            container.removeEventListener("dragover", onDragOver);
            container.removeEventListener("dragleave", onDragLeave);
            container.removeEventListener("drop", onDrop);
        };
    }, [shelfId]);

    const wrapperDockProps = {
        isDocked: dock.isDocked,
        dockEdge: dock.edge,
        onExpand: dock.expand,
        progress: dock.progress,
        progressEdge: dock.progressEdge,
        isOnline,
        isCollapsed,
    };

    if (!shelfId && !disabled) {
        return (
            <ShelfWrapper {...wrapperDockProps} shelfName={currentShelf?.name}>
                <Loader2 className="h-6 w-6 text-foreground animate-spin"/>
            </ShelfWrapper>
        )
    }

    return (
        <ShelfWrapper
            isDraggingOver={isDraggingOver}
            shelfName={currentShelf?.name}
            {...wrapperDockProps}
        >
            <div ref={containerRef} tabIndex={0} className={`w-full h-full outline-none ${disabled ? 'pointer-events-none opacity-30' : ''}`}>
            <div
                className={`absolute z-40 inset-0 bg-bluePrimary/10 backdrop-blur-[3px] flex items-center justify-center animate-in fade-in duration-200 ${!isDraggingOver && 'hidden'}`}>
                <div className="flex flex-col items-center w-full gap-2 text-primary">
                    <Plus className="h-9 w-10 text-bluePrimary"/>
                </div>
            </div>
            {/* Resources List */}
            <div
                data-no-scrollbar
                className="w-full h-full overflow-y-auto px-2.5 z-0 pt-9 shadow-[inset_0_20px_20px_-10px_hsl(var(--card)),inset_0_-20px_20px_-10px_hsl(var(--card))]">
                {selectedResources.length === 0 && (
                    <div
                        className="flex flex-col items-center justify-center h-full text-muted-foreground gap-2 absolute left-0 top-0 w-full pointer-events-none">
                        <p className="text-md text-muted-foreground animate-pop-down-pulse">Drop or paste files here</p>
                    </div>
                )}
                <AnimatePresence mode="wait" initial={false}>
                    {viewMode === 'list' ? (
                        <motion.div
                            key="list"
                            initial={{opacity: 0, scale: 0.96, y: 4}}
                            animate={{opacity: 1, scale: 1, y: 0}}
                            exit={{opacity: 0, scale: 1.02, y: -4}}
                            transition={{duration: 0.18, ease: [0.22, 1, 0.36, 1]}}
                            className="w-full min-h-full flex flex-col"
                            style={{transformOrigin: "center top", willChange: "transform, opacity"}}
                        >
                            <div className="w-full flex flex-col gap-2 my-auto">
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
                        </motion.div>
                    ) : (
                        <motion.div
                            key="stack"
                            initial={{opacity: 0, scale: 0.6}}
                            animate={{opacity: 1, scale: 1}}
                            exit={{opacity: 0, scale: 0.9}}
                            transition={{
                                type: "spring",
                                stiffness: 380,
                                damping: 16,
                                mass: 0.8,
                                opacity: {duration: 0.12, ease: "easeOut"},
                            }}
                            className="h-full"
                            style={{transformOrigin: "center bottom", willChange: "transform, opacity"}}
                        >
                            <StackView
                                resources={selectedResources}
                                onOpen={(resourceId) => {
                                    invoke("open_shelf_resource", {shelfId, resourceId})
                                }}
                            />
                        </motion.div>
                    )}
                </AnimatePresence>
            </div>

            <div
                className="absolute bottom-0 left-0 right-0 h-fit bg-gradient-to-t from-card to-transparent z-20 w-full justify-center flex flex-row pb-2">
                <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                        <button
                            type="button"
                            aria-label="Shelf options"
                            className="h-7 w-7 flex-shrink-0 flex items-center justify-center text-foreground p-[2px] bg-muted/90 hover:bg-muted rounded-full cursor-pointer transition-colors duration-200 ease-out">
                            <MoreHorizontal className="h-full w-full"/>
                        </button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="center" side="top" sideOffset={6} className="dark">
                        <DropdownMenuItem onClick={() => {
                            setTimeout(() => {
                                setViewMode(prev => prev === 'list' ? 'stack' : 'list');
                            }, 0);
                        }}>
                            {viewMode === 'list' ? (
                                <Layers className="w-4 h-4 mr-2"/>
                            ) : (
                                <LayoutList className="w-4 h-4 mr-2"/>
                            )}
                            {viewMode === 'list' ? 'Stack' : 'List'}
                        </DropdownMenuItem>
                        <DropdownMenuItem onClick={() => invoke('paste_from_clipboard', {shelfId})}>
                            <ClipboardPaste className="w-4 h-4 mr-2"/>
                            Paste
                        </DropdownMenuItem>
                        <DropdownMenuItem
                            variant="destructive"
                            disabled={!selectedResources.length || !isResourceRemoveAllowed}
                            onClick={() => {
                                setTimeout(() => {
                                    invoke('clear_shelf', {shelfId});
                                }, 0);
                            }}>
                            <Trash2 className="w-4 h-4 mr-2"/>
                            Clear all
                        </DropdownMenuItem>
                    </DropdownMenuContent>
                </DropdownMenu>
            </div>
            </div>
            {overlay}
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

export function FileView(props: {
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

    const hasReceivers = model.received_by_peers?.length > 0;

    return (
        <Card
            shadowSize={0.35}
            onDoubleClick={() => onOpen(model.order_id)}
            className="w-full border bg-muted rounded-xl flex flex-row hover:bg-muted-foreground/30 p-1 relative group transition-colors cursor-pointer gap-2">
            {/* Thumbnail */}
            <div className={`${hasReceivers ? 'w-14 h-14' : 'w-12 h-12'} shrink-0 rounded-lg bg-muted-foreground/15 p-1 overflow-hidden flex items-center justify-center transition-all`}>
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

            {/* Info + Receivers */}
            <div className="flex-1 min-w-0 flex flex-col justify-center">
                <UnlimitedLineText
                    text={model.name}
                    className="text-sm font-medium text-primaryText"
                    startChars={8}
                    endChars={6}
                    speed={30}
                />
                <p className="text-xs text-primaryText/70">{displaySize}</p>
                <PeerAvatarGroup peers={model.received_by_peers}/>
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

export function MediaView(props: {
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

    const hasReceivers = model.received_by_peers?.length > 0;

    return (
        <Card
            shadowSize={0.35}
            onDoubleClick={() => onOpen(model.order_id)}
            className="border-1 w-full bg-muted rounded-xl flex flex-row hover:bg-muted-foreground/30 p-1 relative group transition-colors cursor-pointer gap-2">
            {/* Thumbnail */}
            <div className={`${hasReceivers ? 'w-14 h-14' : 'w-12 h-12'} flex-shrink-0 rounded-lg bg-muted-foreground/15 p-1 overflow-hidden relative flex items-center justify-center transition-all`}>
                {thumbnailUrl ? (
                    <img src={thumbnailUrl} alt={model.name}
                         className="w-full h-full object-cover rounded-md overflow-clip"/>
                ) : (
                    <FileIcon
                        className="w-6 h-6 text-primary"/>
                )}
                {isVideo && (
                    <div className="absolute top-1.5 right-1.5">
                        <Play className="w-3 h-3 text-white bg-black/50 rounded-md p-0.5"/>
                    </div>
                )}
            </div>

            {/* Info + Receivers */}
            <div className="flex-1 min-w-0 flex flex-col justify-center">
                <UnlimitedLineText
                    text={model.name}
                    className="text-xs font-medium text-primaryText"
                    startChars={8}
                    endChars={6}
                    speed={30}
                />
                <p className="text-xs text-primaryText/70">{displaySize}</p>
                <PeerAvatarGroup peers={model.received_by_peers}/>
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

