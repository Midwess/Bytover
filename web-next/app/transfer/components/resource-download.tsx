import * as React from 'react';
import { useDownloadResource } from '../hooks/use-download-resource';
import DownloadButtonWithProgress from '../download-button-with-progress';
import {
    ReceiveResourceViewModel,
    ReceiveSessionViewModel
} from 'shared_types/types/shared_types';

interface ResourceDownloadProps {
    resource: ReceiveResourceViewModel;
    session: ReceiveSessionViewModel;
    className?: string;
    size?: number;
    strokeWidth?: number;
    buttonText?: string;
    buttonVariant?: 'default' | 'outline' | 'ghost';
    buttonSize?: 'default' | 'sm' | 'lg';
    showButtonText?: boolean;
}

export function ResourceDownload({
    resource,
    session,
    className,
    size = 32,
    strokeWidth = 3,
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
            onDownloadClick={handleDownload}
            onCancelClick={handleCancel}
            size={size}
            strokeWidth={strokeWidth}
            buttonText={showButtonText ? buttonText : undefined}
            buttonVariant={buttonVariant}
            buttonSize={buttonSize}
            className={className}
        />
    );
}
