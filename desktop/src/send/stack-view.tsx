import {CSSProperties} from "react";
import {startDrag} from "@crabnebula/tauri-plugin-drag";
import {convertFileSrc} from "@tauri-apps/api/core";
import {FileIcon, FolderIcon} from "lucide-react";
import {
    ResourceTypeVariantFolder,
    SelectedResourceViewModel,
} from "shared_types/types/shared_types";
import {Card} from "@/components/ui/card";
import {FileView, MediaView} from "@/send/shelf";

const MAX_VISIBLE_PEEKS = 2;

const SLOT_STYLES: Record<number, CSSProperties> = {
    0: {transform: 'none', zIndex: 30, opacity: 1},
    1: {transform: 'translate(2px, 8px) rotate(2deg) scale(0.96)', zIndex: 20, opacity: 0.85},
    2: {transform: 'translate(-2px, 16px) rotate(-2deg) scale(0.92)', zIndex: 10, opacity: 0.65},
};

type StackViewProps = {
    resources: SelectedResourceViewModel[],
    isRemoveAllowed: boolean,
    onOpen: (resourceId: string) => void,
    onRemove: (resourceId: string) => void,
};

function isFileResource(resource: SelectedResourceViewModel): boolean {
    return ['Folder', 'File'].includes(resource.type as any);
}

function PeekCard({model}: {model: SelectedResourceViewModel}) {
    const thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;
    const thumbnailUrl = thumbnailPath ? convertFileSrc(thumbnailPath) : null;
    const isFolder = model.type instanceof ResourceTypeVariantFolder;

    return (
        <Card
            shadowSize={0.35}
            className="w-full border bg-muted rounded-xl flex flex-row p-1 gap-2"
        >
            <div className="w-12 h-12 shrink-0 rounded-lg bg-muted-foreground/15 p-1 overflow-hidden flex items-center justify-center">
                {thumbnailUrl ? (
                    <img
                        src={thumbnailUrl}
                        alt=""
                        className="w-full h-full object-cover rounded-md"
                    />
                ) : isFolder ? (
                    <FolderIcon className="w-6 h-6 text-primary"/>
                ) : (
                    <FileIcon className="w-6 h-6 text-primary"/>
                )}
            </div>
            <div className="flex-1 min-w-0 flex flex-col justify-center">
                <p className="text-sm font-medium text-primaryText truncate">{model.name}</p>
            </div>
        </Card>
    );
}

function TopCard({
    model,
    isRemoveAllowed,
    onOpen,
    onRemove,
}: {
    model: SelectedResourceViewModel,
    isRemoveAllowed: boolean,
    onOpen: (resourceId: string) => void,
    onRemove: (resourceId: string) => void,
}) {
    return isFileResource(model) ? (
        <FileView model={model} isRemoveAllowed={isRemoveAllowed} onOpen={onOpen} onRemove={onRemove}/>
    ) : (
        <MediaView model={model} isRemoveAllowed={isRemoveAllowed} onOpen={onOpen} onRemove={onRemove}/>
    );
}

export function StackView({resources, isRemoveAllowed, onOpen, onRemove}: StackViewProps) {
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
        <div
            draggable
            onDragStart={onDragStart}
            className="relative w-full select-none"
        >
            {peeks
                .slice()
                .reverse()
                .map((resource, reverseIndex) => {
                    const slot = peeks.length - reverseIndex;
                    return (
                        <div
                            key={resource.order_id}
                            className="absolute top-0 left-0 right-0 pointer-events-none"
                            style={SLOT_STYLES[slot]}
                            aria-hidden="true"
                        >
                            <PeekCard model={resource}/>
                        </div>
                    );
                })}

            <div className="relative" style={SLOT_STYLES[0]}>
                <TopCard
                    model={top}
                    isRemoveAllowed={isRemoveAllowed}
                    onOpen={onOpen}
                    onRemove={onRemove}
                />
            </div>

            {overflowCount > 0 && (
                <div
                    className="absolute -top-1.5 -right-1.5 z-40 bg-bluePrimary text-primary-foreground text-xs font-semibold rounded-full h-6 min-w-6 px-1.5 flex items-center justify-center pointer-events-none shadow-md"
                    aria-label={`${overflowCount} more files`}
                >
                    +{overflowCount}
                </div>
            )}
        </div>
    );
}
