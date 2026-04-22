import { useCallback, useEffect } from 'react';
import core from '@/wasm/wasm_core.ts';
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
    const handleDownload = useCallback(async () => {
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
                return;
            }

            const resourceOrderId = BigInt(resource.model.order_id);
            const filename = resource.model.name;

            if (core.isSaveFilePickerSupported()) {
                const handle = await core.pickSaveLocation(filename);
                if (!handle) {
                    return;
                }
                try {
                    await core.registerPickedHandle(sessionOrderId, resourceOrderId, filename, handle);
                } catch (err) {
                    console.error('Failed to register picked handle, falling back to OPFS', err);
                }
            }

            core.update(new AppEventVariantTransfer(
                new TransferEventVariantRequestDownloadResource(
                    peerId,
                    sessionOrderId,
                    resourceOrderId
                )
            ));
        }
        else {
            core.downloadFile(resource.model.path, resource.model.name);
        }
    }, [resource, session, isDownloadAll]);

    const handleCancel = useCallback(() => {
        if (!resource || !session) return;

        const sessionOrderId = BigInt(session.id);
        const resourceOrderId = BigInt(resource.model.order_id);

        core.abortPickedHandle(sessionOrderId, resourceOrderId).catch(() => {});

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
            if (!core.hasAutoDownloaded(session.id, resource.model.order_id)) {
                core.downloadFile(resource.model.path, resource.model.name);
                core.markAutoDownloaded(session.id, resource.model.order_id);
            }
        }
    }, [resource?.is_success, session?.is_scope_online, session?.id, resource?.model.order_id]);

    return {
        handleDownload: () => { handleDownload(); },
        handleCancel
    };
}
