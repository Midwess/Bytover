import * as React from 'react';
import { useDownloadResource } from '@/app/hooks/use-download-resource.ts';
import DownloadButtonWithProgress from './download-button-with-progress.tsx';
import {
    ReceiveResourceViewModel,
    ReceiveSessionViewModel
} from 'shared_types/types/shared_types';

interface ResourceDownloadProps {
    resource: ReceiveResourceViewModel;
    session: ReceiveSessionViewModel;
    className?: string;
    buttonText?: string;
    buttonVariant?: 'default' | 'outline' | 'ghost';
    buttonSize?: 'default' | 'sm' | 'lg';
    showButtonText?: boolean;
}

export function ResourceDownload({
    resource,
    session,
    className,
    buttonText,
    buttonVariant,
    buttonSize,
    showButtonText = false
}: ResourceDownloadProps) {
    const { handleDownload, handleCancel } = useDownloadResource({
        resource,
        session,
        isDownloadAll: false
    });

    return (
        <DownloadButtonWithProgress
            progress={resource.completion}
            isReady={resource.is_ready}
            isCompleted={resource.is_completed}
            isInProgress={resource.completion > 0 && !resource.is_completed}
            isCloud={session.is_cloud}
            onDownloadClick={handleDownload}
            onCancelClick={handleCancel}
            buttonText={showButtonText ? buttonText : undefined}
            buttonVariant={buttonVariant}
            buttonSize={buttonSize}
            className={className}
            containerClass={className}
        />
    );
}
