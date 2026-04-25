import {startDrag} from "@crabnebula/tauri-plugin-drag";
import {convertFileSrc} from "@tauri-apps/api/core";
import {FileIcon, FolderIcon} from "lucide-react";
import {
    ResourceTypeVariantFolder,
    SelectedResourceViewModel,
} from "shared_types/types/shared_types";

const MAX_VISIBLE_PEEKS = 2;
const THUMBNAIL_WIDTH = 144;
const THUMBNAIL_HEIGHT = 81;
const MAX_ROTATION_DEG = 15;

type StackViewProps = {
    resources: SelectedResourceViewModel[],
    onOpen: (resourceId: string) => void,
};

function hashSeed(s: string): number {
    let h = 5381;
    for (let i = 0; i < s.length; i++) {
        h = ((h << 5) + h) + s.charCodeAt(i);
        h |= 0;
    }
    return h >>> 0 || 1;
}

function makeRng(seed: number): () => number {
    let s = seed;
    return () => {
        s = (s * 1664525 + 1013904223) >>> 0;
        return s / 0x100000000;
    };
}

function rotationFor(orderId: string): string {
    const rng = makeRng(hashSeed(orderId));
    const rot = (rng() * 2 - 1) * MAX_ROTATION_DEG;
    return `rotate(${rot.toFixed(2)}deg)`;
}

function Thumbnail({model}: {model: SelectedResourceViewModel}) {
    const thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;
    const thumbnailUrl = thumbnailPath ? convertFileSrc(thumbnailPath) : null;
    const isFolder = model.type instanceof ResourceTypeVariantFolder;

    return (
        <div
            className="overflow-hidden flex items-center justify-center rounded"
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
                <FolderIcon className="w-12 h-12 text-primary"/>
            ) : (
                <FileIcon className="w-12 h-12 text-primary"/>
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
        <div className="w-full h-full flex items-center justify-center overflow-visible">
            <div
                draggable
                onDragStart={onDragStart}
                onDoubleClick={() => onOpen(top.order_id)}
                className="relative select-none overflow-visible"
                style={{width: THUMBNAIL_WIDTH, height: THUMBNAIL_HEIGHT}}
            >
                {peeks.map((resource, index) => (
                    <div
                        key={resource.order_id}
                        className="absolute inset-0 pointer-events-none"
                        style={{
                            transform: rotationFor(resource.order_id),
                            zIndex: 20 - index * 5,
                        }}
                        aria-hidden="true"
                    >
                        <Thumbnail model={resource}/>
                    </div>
                ))}

                <div
                    className="absolute inset-0"
                    style={{
                        transform: rotationFor(top.order_id),
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
