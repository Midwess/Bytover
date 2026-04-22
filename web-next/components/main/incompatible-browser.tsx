'use client';

import { useSyncExternalStore } from 'react';
import { Lock, ShieldAlert } from 'lucide-react';

const subscribe = () => () => {};

function getHttpsUrl(): string | null {
    if (typeof window === 'undefined' || window.isSecureContext) return null;
    const { host, pathname, search, hash } = window.location;
    return `https://${host}${pathname}${search}${hash}`;
}

const getServerHttpsUrl = (): string | null => null;

export function IncompatibleBrowser() {
    const httpsUrl = useSyncExternalStore(subscribe, getHttpsUrl, getServerHttpsUrl);

    if (httpsUrl) {
        return (
            <div className="flex-1 flex flex-col items-center justify-center gap-6 text-center max-w-sm mx-auto">
                <div className="w-16 h-16 rounded-3xl bg-[#1A1A1A] flex items-center justify-center border border-white/5 shadow-2xl">
                    <Lock className="w-8 h-8 text-zinc-600" />
                </div>
                <div className="space-y-3">
                    <h2 className="text-xl font-bold text-white">HTTPS Required</h2>
                    <p className="text-sm text-zinc-500 leading-relaxed">
                        Secure P2P transfer needs an encrypted connection. This page is currently loaded over HTTP, which iOS Safari and most modern browsers block from using the required storage APIs.
                    </p>
                    <a
                        href={httpsUrl}
                        className="inline-block mt-2 text-sm font-medium text-white underline underline-offset-4 decoration-white/30 hover:decoration-white break-all"
                    >
                        Open over HTTPS
                    </a>
                </div>
            </div>
        );
    }

    return (
        <div className="flex-1 flex flex-col items-center justify-center gap-6 text-center max-w-sm mx-auto">
            <div className="w-16 h-16 rounded-3xl bg-[#1A1A1A] flex items-center justify-center border border-white/5 shadow-2xl">
                <ShieldAlert className="w-8 h-8 text-zinc-600" />
            </div>
            <div className="space-y-2">
                <h2 className="text-xl font-bold text-white">Unsupported Browser</h2>
                <p className="text-sm text-zinc-500 leading-relaxed">
                    Secure P2P transfer needs the Cache Storage and Storage Manager APIs. Please update to the latest Chrome, Safari, Edge, or Firefox, and avoid Private Browsing mode.
                </p>
            </div>
        </div>
    );
}
