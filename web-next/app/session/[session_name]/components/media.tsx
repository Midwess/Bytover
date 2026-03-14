'use client'
import * as React from "react";
import { useEffect, useState } from "react";
import {
    ReceiveSessionViewModel,
    ResourceTypeVariantVideo,
    SelectedResourceViewModel,
} from 'shared_types/types/shared_types'
import { Play, ImageUpIcon } from 'lucide-react'
import core from "@/wasm/wasm_core";
import { formatFileSize } from "@/utils/format-file-size";
import { ResourceDownload } from "../../../transfer/components/resource-download";
import { useIsMobile } from "@/hooks/use-mobile";

export function MediaView(props: {
    id: string,
    isCloud: boolean,
    sessionId: string
}) {
    const { id, isCloud, sessionId } = props;
    const media = core.useReceiveResource(id, isCloud);
    const session = core.useSession(sessionId);

    const model: SelectedResourceViewModel | undefined = media?.model;
    const isVideo = model?.type instanceof ResourceTypeVariantVideo;
    const isMobile = useIsMobile();
    const [thumbnailSource, setThumbnailSource] = useState<string | undefined>();

    useEffect(() => {
        if (model?.thumbnail_path) {
            core.getDownloadUrl(model.thumbnail_path).then(setThumbnailSource)
        }
    }, [model?.thumbnail_path]);

    if (!media || !model || !session) return null;

    const displaySize = formatFileSize(model);

    if (isMobile) {
        return (
            <div className="w-full flex items-center gap-3 p-3 rounded-lg border border-border bg-card hover:bg-accent/50 transition-colors pointer-events-auto">
                <div className="w-10 h-10 shrink-0 rounded-md overflow-hidden bg-muted relative">
                    {thumbnailSource ? (
                        /* eslint-disable-next-line @next/next/no-img-element */
                        <img className="w-full h-full object-cover" src={thumbnailSource} alt={model.name} />
                    ) : (
                        <div className="w-full h-full flex items-center justify-center">
                            <ImageUpIcon className="w-5 h-5 opacity-40" />
                        </div>
                    )}
                    {isVideo && (
                        <div className="absolute inset-0 flex items-center justify-center bg-black/30">
                            <Play className="w-3 h-3 text-white fill-white" />
                        </div>
                    )}
                </div>

                <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate text-foreground">
                        {model.name}
                    </p>
                    <div className="flex flex-col md:items-center gap-2 mt-0.5">
                        <p className="text-xs text-muted-foreground">
                            {displaySize}
                        </p>
                    </div>
                </div>

                <div className="shrink-0">
                    <ResourceDownload
                        resource={media}
                        session={session as ReceiveSessionViewModel}
                        size={32}
                        strokeWidth={3}
                    />
                </div>
            </div>
        );
    }

    return (
        <div className="w-full h-full flex flex-col rounded-lg border border-white/10 bg-zinc-900/80 backdrop-blur-md overflow-hidden group hover:border-white/30 transition-colors pointer-events-auto">
            <div className="relative bg-muted/30 h-[calc(100%-76px)]">
                {thumbnailSource ? (
                    /* eslint-disable-next-line @next/next/no-img-element */
                    <img
                        className="w-full h-full object-cover"
                        alt={model.name}
                        src={thumbnailSource}
                    />
                ) : (
                    <div className="w-full h-full flex items-center justify-center">
                        <ImageUpIcon className="w-12 h-12 opacity-20" />
                    </div>
                )}

                {isVideo && (
                    <div className="absolute top-2 right-2 bg-black/60 rounded-full p-1.5">
                        <Play className="w-3 h-3 text-white fill-white" />
                    </div>
                )}
            </div>

            <div className="p-3 border-t border-border flex items-center gap-3 h-[76px] flex-shrink-0">
                <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate text-foreground mb-1">
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
                        resource={media}
                        session={session as ReceiveSessionViewModel}
                        size={36}
                        strokeWidth={3}
                    />
                </div>
            </div>
        </div>
    );
}
