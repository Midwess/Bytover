'use client';

import { useEffect, useState, Suspense } from 'react';
import { useSearchParams } from 'next/navigation';
import { Copy, Check } from 'lucide-react';

function OAuthCallback() {
    const searchParams = useSearchParams();
    const [status, setStatus] = useState<'loading' | 'success'>('loading');
    const [accessToken, setAccessToken] = useState<string | null>(null);
    const [isCopied, setIsCopied] = useState(false);

    useEffect(() => {
        const token = searchParams.get('access_token');
        const redirectUrl = searchParams.get('redirect_url');

        if (token) {
            requestAnimationFrame(() => setAccessToken(token));
        }

        if (window.opener) {
            window.opener.postMessage({
                type: 'OAUTH_CALLBACK',
                payload: {
                    url: window.location.href
                }
            }, '*');

            window.close();
        } else if (redirectUrl) {
            const params = new URLSearchParams(searchParams.toString());
            params.delete('redirect_url');
            window.location.href = `${redirectUrl}?${params.toString()}`;
            requestAnimationFrame(() => setStatus('success'));
        } else if (token) {
            requestAnimationFrame(() => setStatus('success'));
        } else {
            window.location.href = '/';
        }
    }, [searchParams]);

    const handleCopy = async () => {
        if (!accessToken) return;
        try {
            await navigator.clipboard.writeText(accessToken);
            setIsCopied(true);
            setTimeout(() => setIsCopied(false), 2000);
        } catch (err) {
            console.error('Failed to copy token:', err);
        }
    };

    if (status === 'success' && accessToken) {
        return (
            <div className="flex items-center justify-center min-h-screen bg-black text-white">
                <div className="text-center max-w-md px-4">
                    <div className="mb-4 text-green-500">
                        <svg className="w-16 h-16 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                        </svg>
                    </div>
                    <h1 className="text-xl font-bold mb-2">Sign-in successful!</h1>
                    <p className="text-gray-400 mb-6">Copy this token and paste it in the desktop app if authentication doesn&apos;t complete automatically.</p>
                    <div className="flex items-center gap-2 bg-zinc-800 rounded-lg p-3">
                        <input
                            type="text"
                            readOnly
                            value={accessToken}
                            className="flex-1 bg-transparent text-sm text-zinc-300 outline-none font-mono"
                        />
                        <button
                            onClick={handleCopy}
                            className="p-2 hover:bg-zinc-700 rounded-md transition-colors"
                            title="Copy token"
                        >
                            {isCopied ? (
                                <Check className="w-4 h-4 text-green-500" />
                            ) : (
                                <Copy className="w-4 h-4 text-zinc-400" />
                            )}
                        </button>
                    </div>
                </div>
            </div>
        );
    }

    if (status === 'success') {
        return (
            <div className="flex items-center justify-center min-h-screen bg-black text-white">
                <div className="text-center">
                    <div className="mb-4 text-green-500">
                        <svg className="w-16 h-16 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                        </svg>
                    </div>
                    <h1 className="text-xl font-bold mb-2">Sign-in successful!</h1>
                    <p className="text-gray-400">You can close this tab and return to the app.</p>
                </div>
            </div>
        );
    }

    return (
        <div className="flex items-center justify-center min-h-screen bg-black text-white">
            <div className="text-center">
                <h1 className="text-xl font-bold mb-2">Authenticating...</h1>
                <p className="text-gray-400">Please wait while we complete the sign-in process.</p>
            </div>
        </div>
    );
}

export default function Page() {
    return (
        <Suspense fallback={<div className="flex items-center justify-center min-h-screen bg-black text-white">Loading...</div>}>
            <OAuthCallback />
        </Suspense>
    );
}
