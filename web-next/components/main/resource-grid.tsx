'use client';

import { useMemo } from 'react';
import { ReceiveSessionViewModel, ResourceTypeVariantImage, ResourceTypeVariantVideo } from 'shared_types/types/shared_types';
import { ResourceCard } from "./resource-card.tsx";

interface ResourceGridProps {
    session: ReceiveSessionViewModel;
}

export function ResourceGrid({ session }: ResourceGridProps) {
    const images = useMemo(() =>
        session?.resources.filter(r => r.model.type instanceof ResourceTypeVariantImage) || [],
        [session?.resources]
    );
    const videos = useMemo(() =>
        session?.resources.filter(r => r.model.type instanceof ResourceTypeVariantVideo) || [],
        [session?.resources]
    );
    const files = useMemo(() =>
        session?.resources.filter(r =>
            !(r.model.type instanceof ResourceTypeVariantImage) &&
            !(r.model.type instanceof ResourceTypeVariantVideo)
        ) || [],
        [session?.resources]
    );

    return (
        <div className="flex flex-col gap-8">
            {images.length > 0 && (
                <div className="flex flex-col md:grid md:grid-cols-3 lg:grid-cols-4 gap-8">
                    {images.map(image => (
                        <div key={image.model.order_id} className="h-[300px]">
                            <ResourceCard id={image.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                        </div>
                    ))}
                </div>
            )}
            {videos.length > 0 && (
                <div className="flex flex-col md:grid md:grid-cols-3 lg:grid-cols-4 gap-8">
                    {videos.map(video => (
                        <div key={video.model.order_id} className="h-[300px]">
                            <ResourceCard id={video.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                        </div>
                    ))}
                </div>
            )}
            {files.length > 0 && (
                <div className="flex flex-col md:grid md:grid-cols-3 lg:grid-cols-4 gap-8">
                    {files.map(file => (
                        <div key={file.model.order_id} className="h-[300px]">
                            <ResourceCard id={file.model.order_id} isCloud={session.is_cloud} sessionId={session.id} />
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
