'use client';

import * as React from "react";
import { useEffect, useMemo } from "react";
import { useParams } from "next/navigation";
import {
    AppEventVariantTransfer, MessageReasonVariantFailedToFindPublicSession,
    ReceiveSessionViewModel,
    TransferEventVariantFindSession,
    TransferEventVariantViewSession,
    TransferTypeVariantReceive
} from 'shared_types/types/shared_types';
import { LoaderCircle } from 'lucide-react';
import core from "@/wasm/wasm_core";
import Footer from "@/components/web/footer";
import {
    IncompatibleBrowser,
    EmptyState,
    LoadingState,
    PasswordPrompt,
    SessionHeader,
    ResourceGrid,
} from "../../../components/main";

export default function SessionPage() {
    const params = useParams();
    const sessionName = params.session_name as string;
    const coreReady = core.useCoreReady();
    const coreCompatible = core.useIsCoreCompatible();
    const findSessionFailedMessage = core.useMessage(new MessageReasonVariantFailedToFindPublicSession())

    useEffect(() => {
        if (coreReady && coreCompatible) {
            core.launchNearby();
        }
    }, [coreReady, coreCompatible]);

    const searchSessions = core.useSearchSessionsList();
    const allSessions = core.useAllSessionsList();
    const selectedSession = core.useSelectedSession();

    useEffect(() => {
        if (coreReady && sessionName) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantFindSession(sessionName)));
        }
    }, [coreReady, sessionName]);

    const targetSessionFromList = useMemo(() => {
        return searchSessions.find(s => s.alias === sessionName) ||
            allSessions.find(s => s.alias === sessionName);
    }, [searchSessions, allSessions, sessionName]);

    const session = useMemo(() => {
        if (selectedSession?.alias === sessionName) return selectedSession;
        return targetSessionFromList;
    }, [selectedSession, targetSessionFromList, sessionName]);

    const isLoading = session?.is_loading;

    useEffect(() => {
        if (session && !session.resources?.length && !session.error_message) {
            if (!session.password_required || session.password) {
                core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession(
                    session.password || null,
                    BigInt(session.id),
                    new TransferTypeVariantReceive(),
                )));
            }
        }
    }, [session?.id, session?.password_required, session?.password, session?.resources?.length, session?.error_message]);

    const handlePasswordSubmit = (password: string) => {
        if (session) {
            core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession(
                password || null,
                BigInt(session.id),
                new TransferTypeVariantReceive()
            )));
        }
    };

    return (
        <div className="min-h-screen bg-[#0F0F0F] text-[#E0E0E0] flex flex-col font-sans selection:bg-bluePrimary/30 relative">
            {!coreCompatible ? (
                <main className="flex-1 flex flex-col pt-10 pb-32 container mx-auto px-6 w-full min-h-screen">
                    <IncompatibleBrowser />
                </main>
            ) : !session ? (
                <main className="flex-1 flex flex-col pt-6 md:pt-10 pb-12 md:pb-32 container mx-auto px-4 md:px-6 w-full min-h-screen">
                    <div className="flex-1 flex flex-col items-center justify-center gap-4">
                        {findSessionFailedMessage.message ? (
                            <div className="flex flex-col items-center gap-6">
                                <p className="text-foreground font-medium text-xl">
                                    {findSessionFailedMessage.message}
                                </p>
                            </div>
                        ) : (
                            <>
                                <LoaderCircle className="animate-spin w-6 h-6 text-zinc-800" />
                                <p className="text-sm text-zinc-700 font-medium tracking-tight">Finding session...</p>
                            </>
                        )}
                    </div>
                </main>
            ) : (
                <>
                    <SessionHeader session={session as ReceiveSessionViewModel} sessionName={sessionName} />
                    <main className="flex-1 flex flex-col pt-6 md:pt-10 pb-12 md:pb-32 container mx-auto px-4 md:px-6 w-full">
                        <div className="space-y-6 animate-in fade-in duration-700">
                            {isLoading && (!session.resources || session.resources.length === 0) ? (
                                <div className="py-20 md:py-32 flex flex-col items-center justify-center">
                                    {session.password_required && !session.password ? (
                                        <PasswordPrompt
                                            errorMessage={session.error_message ?? undefined}
                                            onSubmit={handlePasswordSubmit}
                                        />
                                    ) : (
                                        <LoadingState status={session.loading_status ?? undefined} />
                                    )}
                                </div>
                            ) : (
                                <ResourceGrid session={session as ReceiveSessionViewModel} />
                            )}

                            {!isLoading && session.resources?.length === 0 && (
                                <EmptyState />
                            )}
                        </div>
                    </main>
                </>
            )}

            <Footer isFullWidth={true} theme="dark" />
        </div>
    );
}
