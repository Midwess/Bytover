import {startDrag} from "@crabnebula/tauri-plugin-drag";
import {convertFileSrc} from "@tauri-apps/api/core";
import {FileIcon, FolderIcon} from "lucide-react";
import {
    ResourceTypeVariantFolder,
    SelectedResourceViewModel,
} from "shared_types/types/shared_types";

const MAX_VISIBLE_PEEKS = 2;
const THUMBNAIL_WIDTH = 112;
const THUMBNAIL_HEIGHT = 63;

const FAN_ANGLES = [0, 14, -12, 8, -10];

type StackViewProps = {
    resources: SelectedResourceViewModel[],
    onOpen: (resourceId: string) => void,
};

function angleForStack(stackIndex: number): number {
    return FAN_ANGLES[stackIndex % FAN_ANGLES.length];
}

function Thumbnail({model}: {model: SelectedResourceViewModel}) {
    const thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;
    const thumbnailUrl = thumbnailPath ? convertFileSrc(thumbnailPath) : null;
    const isFolder = model.type instanceof ResourceTypeVariantFolder;

    return (
        <div
            className="overflow-hidden flex items-center justify-center rounded shadow-lg bg-white"
            style={{width: THUMBNAIL_WIDTH, height: THUMBNAIL_HEIGHT}}
        >
            {thumbnailUrl ? (
                <img
                    src={thumbnailUrl}
                    alt=""
                    className="w-full h-full object-cover block"
                    draggable={false}
                />
            ) : isFolder ? (
                <FolderIcon className="w-8 h-8 text-primary"/>
            ) : (
                <FileIcon className="w-8 h-8 text-primary"/>
            )}
        </div>
    );
}

export function StackView({resources, onOpen}: StackViewProps) {
    if (resources.length === 0) return null;

    const top = resources[0];
    const peeks = resources.slice(1, 1 + MAX_VISIBLE_PEEKS);
    const overflowCount = Math.max(0, resources.length - 1 - MAX_VISIBLE_PEEKS);

    const onDragStart = async (e: React.DragEvent<HTMLDivElement>) => {
        e.preventDefault();
        const paths = resources
            .map(r => (r.path as any)?.AbsolutePath)
            .filter((p): p is string => typeof p === 'string' && p.length > 0);
        if (paths.length === 0) return;
        const topThumbnail = (top.thumbnail_path as any)?.AbsolutePath;
        await startDrag({
            item: paths,
            icon: topThumbnail,
        }, console.log);
    };

    return (
        <div className="w-full h-full flex items-center justify-center overflow-visible pb-10">
            <div
                draggable
                onDragStart={onDragStart}
                onDoubleClick={() => onOpen(top.order_id)}
                className="relative select-none overflow-visible"
                style={{width: THUMBNAIL_WIDTH, height: THUMBNAIL_HEIGHT}}
            >
                {peeks.map((resource, peekIndex) => {
                    const stackIndex = peekIndex + 1;
                    return (
                        <div
                            key={resource.order_id}
                            className="absolute inset-0 pointer-events-none"
                            style={{
                                transform: `rotate(${angleForStack(stackIndex)}deg)`,
                                zIndex: 20 - peekIndex * 5,
                            }}
                            aria-hidden="true"
                        >
                            <Thumbnail model={resource}/>
                        </div>
                    );
                })}

                <div
                    className="absolute inset-0"
                    style={{
                        transform: `rotate(${angleForStack(0)}deg)`,
                        zIndex: 30,
                    }}
                >
                    <Thumbnail model={top}/>
                </div>

                {overflowCount > 0 && (
                    <div
                        className="absolute -top-1.5 -right-1.5 z-40 bg-white/80 text-black backdrop-blur-md text-xs font-semibold rounded-full h-6 min-w-6 px-1.5 flex items-center justify-center pointer-events-none shadow-md"
                        aria-label={`${overflowCount} more files`}
                    >
                        +{overflowCount}
                    </div>
                )}
            </div>
        </div>
    );
}
