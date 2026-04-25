import {startDrag} from "@crabnebula/tauri-plugin-drag";
import {convertFileSrc} from "@tauri-apps/api/core";
import {FileIcon, FolderIcon} from "lucide-react";
import {
    ResourceTypeVariantFolder,
    SelectedResourceViewModel,
} from "shared_types/types/shared_types";

const MAX_VISIBLE_PEEKS = 2;
const THUMBNAIL_WIDTH = 70;
const THUMBNAIL_HEIGHT = 105;
const FAN_SPREAD_DEG = 16;
const ARC_STEP_DEG = FAN_SPREAD_DEG / Math.max(1, Math.ceil(MAX_VISIBLE_PEEKS / 2));
const JITTER_DEG = 5;

type StackViewProps = {
    resources: SelectedResourceViewModel[],
    onOpen: (resourceId: string) => void,
};

function baseAngleForStack(stackIndex: number): number {
    if (stackIndex === 0) return 0;
    const sign = stackIndex % 2 === 1 ? 1 : -1;
    const magnitude = Math.ceil(stackIndex / 2) * ARC_STEP_DEG;
    return sign * magnitude;
}

function jitterFor(seed: string): number {
    let hash = 5381;
    for (let i = 0; i < seed.length; i++) {
        hash = ((hash << 5) + hash + seed.charCodeAt(i)) | 0;
    }
    const normalized = ((hash >>> 0) % 1000) / 999;
    return (normalized * 2 - 1) * JITTER_DEG;
}

function angleForStack(stackIndex: number, seed: string): number {
    return baseAngleForStack(stackIndex) + jitterFor(seed);
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
                                transform: `rotate(${angleForStack(stackIndex, resource.order_id)}deg)`,
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
                        transform: `rotate(${angleForStack(0, top.order_id)}deg)`,
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
