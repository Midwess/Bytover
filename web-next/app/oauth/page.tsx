'use client';

import { useEffect, Suspense } from 'react';
import { useSearchParams } from 'next/navigation';

function OAuthCallback() {
    const searchParams = useSearchParams();

    useEffect(() => {
        if (window.opener) {
            window.opener.postMessage({
                type: 'OAUTH_CALLBACK',
                payload: {
                    url: window.location.href
                }
            }, '*');

            window.close();
        }
        else {
            window.location.href = '/';
        }
    }, [searchParams]);

    return (
        <div className="flex items-center justify-center min-h-screen bg-black text-white">
            <div className="text-center">
                <h1 className="text-xl font-bold mb-2">Authenticating...</h1>
                <p className="text-gray-400">Please wait while we complete the sign-in process.</p>
                <p className="text-sm text-gray-600 mt-4">You can close this window if it doesn't close automatically.</p>
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
