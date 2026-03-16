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
    const isMedia = isImage || isVideo;

    return (
        <div className="w-full group flex items-center justify-between py-3 px-2 border-b border-white/[0.03] last:border-0 transition-all duration-300">
            <div className="flex items-center gap-6 min-w-0">
                <div className="w-12 h-12 shrink-0 flex items-center justify-center rounded-xl relative overflow-hidden text-zinc-500 group-hover:text-white transition-colors">
                    {thumbnailSource ? (
                        /* eslint-disable-next-line @next/next/no-img-element */
                        <img
                            className={isMedia ? "w-full h-full object-cover rounded-lg" : "w-6 h-6 object-contain opacity-40 group-hover:opacity-100 transition-opacity"}
                            alt={model.name}
                            src={thumbnailSource}
                            onError={() => setThumbnailSource(fallbackThumbnail)}
                        />
                    ) : (
                        <div className="w-full h-full flex items-center justify-center">
                            {/* eslint-disable-next-line @next/next/no-img-element */}
                            <img
                                className={isMedia ? "w-full h-full object-cover" : "w-6 h-6 object-contain opacity-40 group-hover:opacity-100 transition-opacity"}
                                alt={model.name}
                                src={fallbackThumbnail}
                            />
                        </div>
                    )}
                    {isVideo && (
                        <div className="absolute inset-0 flex items-center justify-center bg-black/20">
                            <Play className="w-3 h-3 text-white fill-white" />
                        </div>
                    )}
                </div>

                <div className="flex flex-col min-w-0">
                    <h3 className="text-[15px] font-medium text-zinc-200 group-hover:text-white transition-colors truncate">
                        {model.name}
                    </h3>
                    <div className="flex items-center gap-2 mt-1">
                        <span className="text-[10px] font-bold text-zinc-600 uppercase tracking-widest">
                            {displaySize}
                        </span>
                        <span className="w-1 h-1 rounded-full bg-zinc-800" />
                        <span className="text-[10px] font-bold text-zinc-600 uppercase tracking-widest">
                            {isFolder ? "Folder" : isVideo ? "Video" : isImage ? "Image" : "File"}
                        </span>
                    </div>
                </div>
            </div>

            <div className="shrink-0 ml-4">
                <ResourceDownload
                    resource={resource}
                    session={session as ReceiveSessionViewModel}
                    className="!w-10 !h-10 !rounded-full bg-transparent border border-white/5 hover:border-white/20 hover:bg-white hover:text-black transition-all duration-500 shadow-none"
                />
            </div>
        </div>
    );
}

export const FileView = ResourceCard;
export const MediaView = ResourceCard;
