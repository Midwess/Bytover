'use client';

import { ShieldAlert } from 'lucide-react';

export function IncompatibleBrowser() {
    return (
        <div className="flex-1 flex flex-col items-center justify-center gap-6 text-center max-w-sm mx-auto">
            <div className="w-16 h-16 rounded-3xl bg-[#1A1A1A] flex items-center justify-center border border-white/5 shadow-2xl">
                <ShieldAlert className="w-8 h-8 text-zinc-600" />
            </div>
            <div className="space-y-2">
                <h2 className="text-xl font-bold text-white">Access Denied</h2>
                <p className="text-sm text-zinc-500 leading-relaxed">Secure P2P requires a modern browser context. Please use Chrome or Safari.</p>
            </div>
        </div>
    );
}
