import * as React from 'react';
import { useDownloadResource } from '@/app/hooks/use-download-resource.ts';
import DownloadButtonWithProgress from './download-button-with-progress.tsx';
import {
    ReceiveResourceViewModel,
    ReceiveSessionViewModel
} from '../../../shared_types/generated/typescript/types/shared_types.ts';

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
