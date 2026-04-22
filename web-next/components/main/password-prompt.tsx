'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/button.tsx';
import { Input } from "@/components/ui/input.tsx";

interface PasswordPromptProps {
    errorMessage?: string;
    onSubmit: (password: string) => void;
}

export function PasswordPrompt({ errorMessage, onSubmit, theme = 'dark' }: PasswordPromptProps & { theme?: 'light' | 'dark' }) {
    const [password, setPassword] = useState('');
    const isLight = theme === 'light';

    const handleSubmit = () => {
        onSubmit(password);
    };

    return (
        <div className={`w-full max-w-sm p-10 rounded-[40px] shadow-2xl space-y-8 ${isLight ? 'bg-white border border-zinc-200' : 'bg-[#1A1A1A] border border-white/5'}`}>
            <div className="text-center space-y-2">
                <h2 className={`text-xl font-bold ${isLight ? 'text-zinc-900' : 'text-white'}`}>Locked</h2>
                <p className="text-sm text-zinc-500 font-medium">Provide the encryption key to decrypt metadata.</p>
            </div>
            <div className="space-y-4">
                <Input
                    type="password"
                    placeholder="Enter password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleSubmit()}
                    className={`${isLight ? 'bg-zinc-50 border-zinc-200 text-zinc-900 placeholder:text-zinc-400' : 'bg-black/40 border-white/5 text-white'} h-14 rounded-2xl text-center text-lg focus:border-bluePrimary/50 transition-all`}
                />
                <Button onClick={handleSubmit} className="w-full bg-blue-600 text-white hover:bg-blue-700 h-14 rounded-2xl font-bold text-base transition-transform active:scale-95 shadow-lg shadow-blue-600/20">
                    Decrypt & Open
                </Button>
                {errorMessage && <p className="text-red-500 text-xs text-center font-medium">{errorMessage}</p>}
            </div>
        </div>
    );
}
