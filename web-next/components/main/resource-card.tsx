'use client'
import * as React from "react";
import { useEffect, useState } from "react";
import {
    ReceiveSessionViewModel,
    ResourceTypeVariantFolder,
    ResourceTypeVariantImage,
    ResourceTypeVariantVideo,
    SelectedResourceViewModel,
} from 'shared_types/types/shared_types'
import { Play } from 'lucide-react'
import core from "@/wasm/wasm_core.ts";
import { formatFileSize } from "@/utils/format-file-size.ts";
import { useIsMobile } from "@/hooks/use-mobile.ts";
import { ResourceDownload } from "@/components/main/resource-download.tsx";

export function ResourceCard(props: {
    id: string,
    isCloud: boolean,
    sessionId: string
}) {
    const { id, isCloud, sessionId } = props;
    const resource = core.useReceiveResource(id, isCloud);
    const session = core.useSession(sessionId);

    const model: SelectedResourceViewModel | undefined = resource?.model;
    const isFolder = model?.type instanceof ResourceTypeVariantFolder;
    const isImage = model?.type instanceof ResourceTypeVariantImage;
    const isVideo = model?.type instanceof ResourceTypeVariantVideo;
    const isMobile = useIsMobile();

    const fallbackThumbnail = isFolder ? "/folder.svg" : "/file.svg";
    const [thumbnailSource, setThumbnailSource] = useState<string | undefined>();

    useEffect(() => {
        if (!model?.thumbnail_path) {
            setThumbnailSource(undefined)
            return
        }

        if (model.thumbnail_path && !thumbnailSource) {
            core.getDownloadUrl(model.thumbnail_path).then(setThumbnailSource)
        }
    }, [model, model?.thumbnail_path, thumbnailSource]);

    if (!resource || !model || !session) return null;

    const displaySize = formatFileSize(model);

    if (isMobile) {
        return (
            <div className="w-full flex items-center gap-3 p-3 rounded-lg border border-border hover:bg-accent/50 transition-colors pointer-events-auto">
                <div className="w-10 h-10 shrink-0 flex items-center justify-center rounded-md bg-muted">
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img
                        className="w-6 h-6 object-contain opacity-70"
                        alt={model.name}
                        src={thumbnailSource || fallbackThumbnail}
                        onError={() => setThumbnailSource(fallbackThumbnail)}
                    />
                    {isVideo && (
                        <div className="absolute inset-0 flex items-center justify-center bg-black/30">
                            <Play className="w-3 h-3 text-white fill-white" />
                        </div>
                    )}
                </div>

                <div className="flex-1 min-w-0">
                    <p className="text-sm font-bold truncate text-white">
                        {model.name}
                    </p>
                    <div className="flex items-center gap-2 mt-0.5">
                        <p className="text-xs text-muted-foreground">
                            {displaySize}
                        </p>
                        <span className="text-xs text-muted-foreground/60">•</span>
                        <p className="text-xs text-muted-foreground">
                            {isFolder ? "Folder" : isVideo ? "Video" : isImage ? "Image" : "File"}
                        </p>
                    </div>
                </div>

                <div className="shrink-0">
                    <ResourceDownload
                        resource={resource}
                        session={session as ReceiveSessionViewModel}
                        className="w-8 h-8"
                    />
                </div>
            </div>
        );
    }

    const isMedia = isImage || isVideo;

    return (
        <div className="w-full h-full flex flex-col overflow-hidden group hover:border-white/20 transition-colors pointer-events-auto">
            <div className={`relative bg-muted/30 rounded-xl overflow-clip h-[70%] ${!isMedia ? 'flex items-center justify-center' : ''}`}>
                {thumbnailSource ? (
                    /* eslint-disable-next-line @next/next/no-img-element */
                    <img
                        className={isMedia ? "w-full h-full object-cover" : "w-24 h-24 object-contain"}
                        alt={model.name}
                        src={thumbnailSource}
                        onError={() => setThumbnailSource(fallbackThumbnail)}
                    />
                ) : (
                    <div className="w-full h-full flex items-center justify-center">
                        {/* eslint-disable-next-line @next/next/no-img-element */}
                        <img
                            className={isMedia ? "w-full h-full object-cover" : "w-16 h-16 object-contain opacity-40"}
                            alt={model.name}
                            src={fallbackThumbnail}
                        />
                    </div>
                )}

                {isVideo && (
                    <div className="absolute top-2 right-2 bg-black/60 rounded-full p-1.5">
                        <Play className="w-3 h-3 text-white fill-white" />
                    </div>
                )}
            </div>

            <div className="flex items-center gap-3 h-fit mt-5 flex-shrink-0">
                <div className="flex-1 min-w-0">
                    <p className="text-sm font-bold truncate text-white mb-1">
                        {model.name}
                    </p>
                    <div className="flex flex-col items-start gap-0">
                        <p className="text-xs text-muted-foreground">
                            {displaySize}
                        </p>
                    </div>
                </div>

                <div className="shrink-0">
                    <ResourceDownload
                        resource={resource}
                        session={session as ReceiveSessionViewModel}
                        className="w-8 h-8"
                    />
                </div>
            </div>
        </div>
    );
}

export const FileView = ResourceCard;
export const MediaView = ResourceCard;
