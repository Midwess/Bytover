import {CSSProperties} from "react";
import {startDrag} from "@crabnebula/tauri-plugin-drag";
import {convertFileSrc} from "@tauri-apps/api/core";
import {FileIcon, FolderIcon} from "lucide-react";
import {
    ResourceTypeVariantFolder,
    SelectedResourceViewModel,
} from "shared_types/types/shared_types";
import {Card} from "@/components/ui/card";

const MAX_VISIBLE_PEEKS = 2;

const CARD_WIDTH = 128;
const CARD_HEIGHT = 144;

const SLOT_STYLES: Record<number, CSSProperties> = {
    0: {transform: 'none', zIndex: 30, opacity: 1},
    1: {transform: 'translate(6px, 4px) rotate(4deg) scale(0.95)', zIndex: 20, opacity: 0.85},
    2: {transform: 'translate(-6px, 8px) rotate(-4deg) scale(0.9)', zIndex: 10, opacity: 0.65},
};

type StackViewProps = {
    resources: SelectedResourceViewModel[],
    onOpen: (resourceId: string) => void,
};

function StackCard({
    model,
    onDoubleClick,
}: {
    model: SelectedResourceViewModel,
    onDoubleClick?: () => void,
}) {
    const thumbnailPath = (model.thumbnail_path as any)?.AbsolutePath;
    const thumbnailUrl = thumbnailPath ? convertFileSrc(thumbnailPath) : null;
    const isFolder = model.type instanceof ResourceTypeVariantFolder;

    return (
        <Card
            shadowSize={0.35}
            onDoubleClick={onDoubleClick}
            className="bg-muted rounded-xl flex flex-col p-1.5 gap-1.5"
            style={{width: CARD_WIDTH, height: CARD_HEIGHT}}
        >
            <div className="flex-1 rounded-lg bg-muted-foreground/15 overflow-hidden flex items-center justify-center">
                {thumbnailUrl ? (
                    <img
                        src={thumbnailUrl}
                        alt=""
                        className="w-full h-full object-cover"
                    />
                ) : isFolder ? (
                    <FolderIcon className="w-10 h-10 text-primary"/>
                ) : (
                    <FileIcon className="w-10 h-10 text-primary"/>
                )}
            </div>
            <p className="text-xs font-medium text-primaryText text-center truncate px-1">
                {model.name}
            </p>
        </Card>
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
        <div className="w-full h-full flex items-center justify-center">
            <div
                draggable
                onDragStart={onDragStart}
                className="relative select-none"
                style={{width: CARD_WIDTH, height: CARD_HEIGHT}}
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
                                <StackCard model={resource}/>
                            </div>
                        );
                    })}

                <div className="relative" style={SLOT_STYLES[0]}>
                    <StackCard model={top} onDoubleClick={() => onOpen(top.order_id)}/>
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
