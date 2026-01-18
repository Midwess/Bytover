'use client';

import { useEffect, useState, Suspense } from 'react';
import { useSearchParams } from 'next/navigation';

function OAuthCallback() {
    const searchParams = useSearchParams();
    const [status, setStatus] = useState<'loading' | 'success'>('loading');

    useEffect(() => {
        const redirectUrl = searchParams.get('redirect_url');

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
            setStatus('success');
        } else {
            window.location.href = '/';
        }
    }, [searchParams]);

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
