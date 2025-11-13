import {Card} from "@/components/ui/card.tsx";
import {getCurrentWindow, PhysicalPosition} from "@tauri-apps/api/window";
import {startDrag} from "@crabnebula/tauri-plugin-drag";
import {noop} from "motion";
import {invoke} from "@tauri-apps/api/core";
import {convertFileSrc} from "@tauri-apps/api/core";
import {useEffect, useRef, useState} from "react";
import core from "@/core.ts";
import {
    Upload,
    Play,
    FolderIcon,
    FileIcon,
    MoreVertical,
    Trash2,
    Minus, UploadCloud, ImportIcon, Circle, CircleChevronDown, Plus,
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
import useWindow from "@/hooks/use-window.ts";
import {throttle} from "lodash";

export function Shelf() {
    const window = getCurrentWindow()
    const windowInfo = useWindow(window)
    const selectedResources = core.useSelectedResources()
    const effectRan = useRef(false);
    const [isDraggingOver, setIsDraggingOver] = useState(false);

    useEffect(() => {
        if (effectRan.current) return;

        effectRan.current = true;
        let unlisten: (() => void) | undefined;

        const setup = async () => {
            unlisten = await window.onDragDropEvent(throttle(({payload}) => {
                const eventPosition: PhysicalPosition | undefined = (payload as any)?.position
                console.log(eventPosition)
                const isLeftSide = eventPosition?.x && eventPosition.x < windowInfo.position.x + windowInfo.size.width / 2;
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
            }, 120, {leading: true, trailing: true}));
        };

        setup();

        return () => {
            if (unlisten) {
                unlisten();
            }
        };
    }, [windowInfo]);

    return <>
        <Card
            className={`
            rounded-3xl
            flex flex-col
            justify-center
            items-center
            w-full h-full border-2
            transition-all duration-200 relative overflow-hidden
            ${isDraggingOver
            ? 'border-2 border-bluePrimary shadow-[0_0_8px_2px_rgb(var(--bluePrimary))_inset]'
            : 'border-border'
        }
        }
        `}>
            <div
                className="absolute top-0 left-0 right-0 h-5 bg-gradient-to-b from-card to-transparent pointer-events-none z-20"/>
            <div data-tauri-drag-region
                 onDoubleClick={() => {
                     console.log("close")
                     getCurrentWindow()?.hide()
                 }}
                 className={"w-full py-1 absolute top-0 flex justify-center items-center z-30 group"}>
                <Minus
                    className={"pointer-events-none scale-x-200 scale-y-200 text-primary transition-transform duration-200 group-hover:scale-x-[3] group-hover:scale-y-[2.5]"}/>
            </div>
            <div
                className={`absolute inset-0 bg-bluePrimary/10 backdrop-blur-[3px] flex items-center justify-center z-10 animate-in fade-in duration-200 ${!isDraggingOver && 'hidden'}`}>
                <div className="flex flex-col items-center w-full gap-2 text-primary">
                    <Plus className="h-12 w-12 text-bluePrimary"/>
                </div>
            </div>
            {/* Resources List */}
            <div
                className="w-full h-full overflow-y-auto px-2.5 z-0 pt-9 shadow-[inset_0_20px_20px_-10px_hsl(var(--card)),inset_0_-20px_20px_-10px_hsl(var(--card))]">
                {selectedResources.length === 0 ? (
                    <div
                        className="flex flex-col items-center justify-center h-full text-muted-foreground gap-2">
                        <p className="text-sm opacity-70">Drop files here</p>
                    </div>
                ) : (
                    <div className="flex flex-col gap-2">
                        {selectedResources.map((resource, index) => (
                            <ResourceView key={index} model={resource}/>
                        ))}
                        {/*Padding item*/}
                        <div className={"h-5"}></div>
                    </div>
                )}
            </div>

            {/* Bottom fade mask */}
            <div className="absolute bottom-0 left-0 right-0 h-5 bg-gradient-to-t from-card to-transparent pointer-events-none z-20"/>
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
        <Card
            className="w-full border-none bg-muted rounded-xl flex flex-row hover:bg-muted-foreground/30 items-center gap-3 p-1 relative group transition-colors">
            {/* Thumbnail */}
            <div className="w-12 h-12 flex-shrink-0 rounded-lg bg-muted-foreground/15 p-1 overflow-hidden relative">
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
                <DropdownMenuContent align="end" className="dark">
                    <DropdownMenuItem variant="destructive" onClick={() => {
                        invoke("remove_resource", {resourceId: model.order_id})
                    }}>
                        <Trash2 className="w-4 h-4 mr-2"/>
                        Remove
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        </Card>
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
        <Card
            className="border-none w-full bg-muted rounded-xl flex hover:bg-muted-foreground/30 flex-row items-center gap-3 p-1 relative group transition-colors">
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
                <DropdownMenuContent align="end" className={"dark"}>
                    <DropdownMenuItem variant="destructive" onClick={() => {
                        invoke("remove_resource", {resourceId: String(model.order_id)})
                    }}>
                        <Trash2 className="w-4 h-4 mr-2"/>
                        Remove
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        </Card>
    );
}