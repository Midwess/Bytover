'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/button.tsx';
import { Input } from "@/components/ui/input.tsx";

interface PasswordPromptProps {
    errorMessage?: string;
    onSubmit: (password: string) => void;
}

export function PasswordPrompt({ errorMessage, onSubmit }: PasswordPromptProps) {
    const [password, setPassword] = useState('');

    const handleSubmit = () => {
        onSubmit(password);
    };

    return (
        <div className="w-full max-w-sm p-10 rounded-[40px] bg-[#1A1A1A] border border-white/5 shadow-2xl space-y-8">
            <div className="text-center space-y-2">
                <h2 className="text-xl font-bold text-white">Locked Drop</h2>
                <p className="text-sm text-zinc-500">Provide the encryption key to decrypt metadata.</p>
            </div>
            <div className="space-y-4">
                <Input
                    type="password"
                    placeholder="Enter password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleSubmit()}
                    className="bg-black/40 border-white/5 h-14 rounded-2xl text-center text-lg focus:border-bluePrimary/50 transition-all text-white"
                />
                <Button onClick={handleSubmit} className="w-full bg-bluePrimary text-white hover:bg-bluePrimary/90 h-14 rounded-2xl font-bold text-base transition-transform active:scale-95 shadow-lg shadow-bluePrimary/20">
                    Decrypt & Open
                </Button>
                {errorMessage && <p className="text-red-500 text-xs text-center font-medium">{errorMessage}</p>}
            </div>
        </div>
    );
}
