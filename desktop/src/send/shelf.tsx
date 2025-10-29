import {Card} from "@/components/ui/card.tsx";
import {getCurrentWindow, PhysicalPosition} from "@tauri-apps/api/window";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import {noop} from "motion";
import {invoke} from "@tauri-apps/api/core";
import {convertFileSrc} from "@tauri-apps/api/core";
import {useEffect, useRef, useState} from "react";
import core from "@/core.ts";
import {Upload, Play, FolderIcon, FileIcon, MoreVertical, Trash2, Minus} from "lucide-react";
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
            unlisten = await window.onDragDropEvent(async ({payload}) => {
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
            p-0
            w-full h-full bg-card shadow-md shadow-background border-1 
            transition-all duration-200 relative overflow-hidden
            ${isDraggingOver
            ? 'border-bluePrimary border-2 shadow-lg shadow-bluePrimary/20'
            : 'border-border'
        }
        `}>
            <div data-tauri-drag-region
                 className={"w-full absolute top-0 flex justify-center items-center py-1 z-10 group"}>
                <Minus
                    className={"scale-x-200 scale-y-200 pointer-events-none transition-transform duration-200 group-hover:scale-x-[3] group-hover:scale-y-[2.5]"}/>
            </div>
            {isDraggingOver && (
                <div
                    className="absolute inset-0 bg-bluePrimary/10 backdrop-blur-[1px] flex items-center justify-center z-10 animate-in fade-in duration-200">
                    <div className="flex flex-col items-center gap-2 text-primary">
                        <Upload className="h-12 w-12 animate-bounce opacity-80"/>
                        <span className="text-sm font-bold">Drop files here</span>
                    </div>
                </div>
            )}

            {/* Resources List */}
            <div className="w-full h-full overflow-y-auto px-2 z-0 pt-9">
                {selectedResources.length === 0 ? (
                    <div data-tauri-drag-region
                         className="flex flex-col items-center justify-center h-full text-muted-foreground gap-2">
                        <Upload className="h-8 w-8 opacity-40"/>
                        <p className="text-sm opacity-70">Drop files here</p>
                    </div>
                ) : (
                    <div data-tauri-drag-region className="flex flex-col gap-2">
                        {selectedResources.map((resource, index) => (
                            <ResourceView key={index} model={resource}/>
                        ))}
                    </div>
                )}
            </div>
        </Card>
    </>
}

function ResourceView(props: { model: SelectedResourceViewModel }) {
    const {model} = props;
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
                ? <FileView model={model}/>
                : <MediaView model={model}/>
        }
    </div>
}

function FileView(props: { model: SelectedResourceViewModel }) {
    const {model} = props;

    let thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;
    const isFolder = model.type instanceof ResourceTypeVariantFolder;

    // Convert absolute path to Tauri asset URL
    const thumbnailUrl = thumbnailPath ? convertFileSrc(thumbnailPath) : null;

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <div
            className="w-full bg-muted rounded-lg flex flex-row hover:opacity-70 items-center gap-3 p-2 relative group transition-colors border border-primaryText/5">
            {/* Thumbnail */}
            <div className="w-12 h-12 flex-shrink-0 rounded-md bg-muted-foreground/15 p-1 overflow-hidden relative">
                {thumbnailUrl ? (
                    <img
                        src={thumbnailUrl} alt={model.name}
                        className="w-full h-full object-cover rounded-sm overflow-hidden"/>
                ) : isFolder ? (
                    <FolderIcon className="w-6 h-6 text-primary"/>
                ) : (
                    <FileIcon className="w-6 h-6 text-primary"/>
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
                        <MoreVertical className="w-4 h-4"/>
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                    <DropdownMenuItem variant="destructive" className="bg-card" onClick={() => {
                        invoke("remove_resource", {resourceId: model.order_id})
                    }}>
                        <Trash2 className="w-4 h-4 mr-2"/>
                        Remove
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        </div>
    );
}

function MediaView(props: { model: SelectedResourceViewModel }) {
    const {model} = props;

    const isVideo = (model.type as any) === 'Video';
    const thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;

    // Convert absolute path to Tauri asset URL
    const thumbnailUrl = thumbnailPath ? convertFileSrc(thumbnailPath) : null;

    let displaySize = `${model.size_mb} MB`;
    if (model.size_gb > 0) {
        displaySize = `${model.size_gb} GB`;
    }

    return (
        <div
            className="w-full bg-muted rounded-lg flex hover:opacity-70 flex-row items-center gap-3 p-2 relative group transition-colors border border-primaryText/5">
            {/* Thumbnail */}
            <div className="w-12 h-12 flex-shrink-0 rounded-md bg-muted-foreground/15 p-1 overflow-hidden relative">
                {thumbnailUrl ? (
                    <img src={thumbnailUrl} alt={model.name}
                         className="w-full h-full object-cover rounded-sm overflow-clip"/>
                ) : (
                    <FileIcon
                        className="w-6 h-6 text-primary absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2"/>
                )}
                {isVideo && (
                    <div className="absolute top-1.5 right-1.5">
                        <Play className="w-3 h-3 text-white bg-black/50 rounded-sm p-0.5"/>
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
                        <MoreVertical className="w-4 h-4"/>
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                    <DropdownMenuItem variant="destructive" className="bg-card" onClick={() => {
                        invoke("remove_resource", {resourceId: String(model.order_id)})
                    }}>
                        <Trash2 className="w-4 h-4 mr-2"/>
                        Remove
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        </div>
    );
}