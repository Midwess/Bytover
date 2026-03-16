import * as React from 'react';
import { useDownloadResource } from '@/app/hooks/use-download-resource.ts';
import DownloadButtonWithProgress from './download-button-with-progress.tsx';
import {
    ReceiveSessionViewModel
} from 'shared_types/types/shared_types';

interface DownloadAllButtonProps {
    session: ReceiveSessionViewModel;
    className?: string;
    containerClass?: string;
}

export function DownloadAllButton({
    session,
    className,
    containerClass
}: DownloadAllButtonProps) {
    const { handleDownload, handleCancel } = useDownloadResource({
        resource: session.download_all_resource ?? null,
        session,
        isDownloadAll: true
    });

    if (!session.download_all_resource) {
        return null;
    }

    const resource = session.download_all_resource;

    return (
        <DownloadButtonWithProgress
            progress={resource.completion}
            isReady={resource.is_ready}
            isCompleted={resource.is_completed}
            isInProgress={resource.completion > 0 && !resource.is_completed}
            onDownloadClick={handleDownload}
            onCancelClick={handleCancel}
            buttonText="Download All"
            buttonVariant="outline"
            buttonSize="sm"
            className={className}
            containerClass={containerClass}
        />
    );
}
