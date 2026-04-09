'use client';

import { LoaderCircle } from 'lucide-react';
import { SignallingAnimation, DEFAULT_WORDS } from "@/components/ui/signalling-animation.tsx";

interface LoadingStateProps {
    status?: string;
}

export function LoadingState({ status }: LoadingStateProps) {
    return (
        <div className="flex flex-col items-center gap-6">
            <LoaderCircle className="animate-spin w-8 h-8 text-zinc-800" />
            <p className="text-zinc-600 font-bold uppercase tracking-[0.2em] text-xs">
                <SignallingAnimation words={[status || 'Synchronizing', ...DEFAULT_WORDS]} />
            </p>
        </div>
    );
}
