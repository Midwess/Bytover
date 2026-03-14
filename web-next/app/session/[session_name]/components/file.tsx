'use client'
import * as React from "react";
import { useEffect, useState } from "react";
import {
    ReceiveSessionViewModel,
    ResourceTypeVariantFolder,
} from 'shared_types/types/shared_types'
import core from "@/wasm/wasm_core";
import { formatFileSize } from "@/utils/format-file-size";
import { ResourceDownload } from "../../../transfer/components/resource-download";

export function FileView(props: {
    id: string,
    isCloud: boolean,
    sessionId: string
}) {
    const { id, isCloud, sessionId } = props;
    const file = core.useReceiveResource(id, isCloud);
    const session = core.useSession(sessionId);
    const model = file?.model;

    const isFolder = model?.type instanceof ResourceTypeVariantFolder;
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

    if (!file || !model || !session) return null;

    const displaySize = formatFileSize(model);

    return (
        <div className="w-full flex items-center gap-3 p-3 rounded-lg border border-white/10 bg-zinc-900/80 backdrop-blur-md hover:bg-zinc-900 transition-colors pointer-events-auto">
            <div className="w-10 h-10 shrink-0 flex items-center justify-center rounded-md bg-muted">
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img
                    className="w-6 h-6 object-contain opacity-70"
                    alt={model.name}
                    src={thumbnailSource || fallbackThumbnail}
                    onError={() => setThumbnailSource(fallbackThumbnail)}
                />
            </div>

            <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate text-foreground">
                    {model.name}
                </p>
                <div className="flex items-center gap-2 mt-0.5">
                    <p className="text-xs text-muted-foreground">
                        {displaySize}
                    </p>
                    <span className="text-xs text-muted-foreground/60">•</span>
                    <p className="text-xs text-muted-foreground">
                        {isFolder ? "Folder" : "File"}
                    </p>
                </div>
            </div>

            <div className="shrink-0">
                <ResourceDownload
                    resource={file}
                    session={session as ReceiveSessionViewModel}
                    size={40}
                    strokeWidth={4}
                />
            </div>
        </div>
    );
}
