'use client';

import { Bento } from "@/components/bento";
import { DownloadPlatforms } from "@/components/download-platforms";
import { getAssetUrl } from "@/utils/asset-url";

export function DesktopSection() {
    const features = [
        {
            id: "shelf",
            heading: "Organize Your Files",
            description: "Keep all your files organized in a beautiful shelf interface. Easy drag-and-drop, quick access.",
            video: getAssetUrl("/demo/desktop-shelf.mp4"),
            variant: 'big' as const,
        },
        {
            id: "public-share",
            heading: "Public file transfer",
            description: "Share files with anyone using a simple link. Optional Password protected keeps your content secure while making sharing effortless.",
            video: getAssetUrl("/demo/desktop-share-public.mp4"),
            variant: 'small' as const,
            height: '400px',
        },
        {
            id: "nearby-share",
            heading: "Instant Transfer",
            description: "Share files instantly with anyone using a simple link. No upload needed - files transfer directly to the receiver.",
            video: getAssetUrl("/demo/desktop-share-nearby.mp4"),
            variant: 'small' as const,
        },
    ];

    return (
        <section className="w-full bg-transparent">
            <div className="w-full flex flex-col items-center px-0">
                <div className="mb-16 flex flex-col items-center text-center max-w-2xl px-6">
                    <span className="inline-flex items-center px-4 py-1.5 rounded-full text-sm font-semibold bg-bluePrimary/20 text-bluePrimary border border-bluePrimary/30 mb-6">
                        Desktop App
                    </span>
                    <h2 className="mb-4 text-3xl font-bold text-white md:text-4xl lg:text-5xl">
                        Even better on desktop
                    </h2>
                    <p className="text-primaryText/60 text-base md:text-lg mb-8">
                        Get the full experience with our native desktop app. Faster transfers, system integration, and seamless file management.
                    </p>
                    <DownloadPlatforms />
                </div>
            </div>
            <div className="py-8 bg-black">
                <Bento cards={features} />
            </div>
        </section>
    );
}
