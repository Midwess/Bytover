import { useCallback, useEffect } from 'react';
import core from '@/wasm/wasm_core';
import {
    AppEventVariantTransfer,
    ReceiveResourceViewModel,
    ReceiveSessionViewModel,
    TransferEventVariantRequestDownloadResource,
    TransferEventVariantRequestDownloadAllResources,
    TransferEventVariantCancelResourceTransfer,
    TransferTypeVariantReceive,
} from 'shared_types/types/shared_types';

interface UseDownloadResourceParams {
    resource: ReceiveResourceViewModel | null;
    session: ReceiveSessionViewModel | null;
    isDownloadAll?: boolean;
}

interface UseDownloadResourceReturn {
    handleDownload: () => void;
    handleCancel: () => void;
}

export function useDownloadResource({
    resource,
    session,
    isDownloadAll = false
}: UseDownloadResourceParams): UseDownloadResourceReturn {
    const handleDownload = useCallback(() => {
        if (!resource || !session) return;

        const isCloud = session.is_cloud;
        const isSuccess = resource.is_success;

        if (!isCloud && !isSuccess) {
            const peerId = session.sender_id;
            const sessionOrderId = BigInt(session.id);

            if (isDownloadAll) {
                core.update(new AppEventVariantTransfer(
                    new TransferEventVariantRequestDownloadAllResources(peerId, sessionOrderId)
                ));
            } else {
                const resourceOrderId = BigInt(resource.model.order_id);
                core.update(new AppEventVariantTransfer(
                    new TransferEventVariantRequestDownloadResource(
                        peerId,
                        sessionOrderId,
                        resourceOrderId
                    )
                ));
            }
        } else {
            core.downloadFile(resource.model.path, resource.model.name);
        }
    }, [resource, session, isDownloadAll]);

    const handleCancel = useCallback(() => {
        if (!resource || !session) return;

        const sessionOrderId = BigInt(session.id);
        const resourceOrderId = BigInt(resource.model.order_id);

        core.update(new AppEventVariantTransfer(
            new TransferEventVariantCancelResourceTransfer(
                sessionOrderId,
                new TransferTypeVariantReceive(),
                resourceOrderId
            )
        ));
    }, [resource, session]);

    useEffect(() => {
        if (session?.is_scope_online && resource?.is_success && resource?.model.path) {
            core.downloadFile(resource.model.path, resource.model.name);
        }
    }, [resource?.is_success, session?.is_scope_online]);

    return {
        handleDownload,
        handleCancel
    };
}
