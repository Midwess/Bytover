import Header from "@/components/web/header";
import Footer from "@/components/web/footer";

import { Suspense } from "react";
import MagicBento from "@/components/MagicBento";
import Introduction from "@/app/home/introduction.tsx";
import { JoinWaitList } from "@/components/join-waitlist";
import { AdditionalFeatures } from "@/components/additional-features";
import { DownloadPlatforms } from "@/components/download-platforms";
import { getAssetUrl } from "@/utils/asset-url";

function DesktopSection() {
    const features = [
        {
            id: "shelf",
            heading: "Organize Your Files",
            description: "Keep all your files organized in a beautiful shelf interface. Easy drag-and-drop, quick access.",
            video: getAssetUrl("/demo/desktop-shelf.mp4"),
            color: '#060010',
        },
        {
            id: "public-share",
            heading: "Public file transfer",
            description: "Share files with anyone using a simple link. Optional Password protected keeps your content secure while making sharing effortless.",
            video: getAssetUrl("/demo/desktop-share-public.mp4"),
            color: '#060010',
        },
        {
            id: "nearby-share",
            heading: "Instant Transfer",
            description: "Share files instantly with anyone using a simple link. No upload needed - files transfer directly to the receiver.",
            video: getAssetUrl("/demo/desktop-share-nearby.mp4"),
            color: '#060010',
        },
        {
            id: "all-platform",
            heading: "Available on All Platforms",
            description: "Coming soon this year! Native apps for Windows, macOS, iOS, and Android. Experience Bytover seamlessly across all your devices with full feature parity and consistent performance.",
            image: getAssetUrl("/demo/mobile_mockup_1.png"),
            color: '#060010',
        },
    ];

    return (
        <section className="py-20 md:py-32">
            <div className="w-full lg:h-[1400px] flex flex-col items-center px-0">
                {/* Desktop Introduction */}
                <div className="mb-16 flex flex-col items-center text-center max-w-2xl px-6">
                    <span className="inline-flex items-center px-4 py-1.5 rounded-full text-sm font-semibold bg-bluePrimary/20 text-blue border border-bluePrimary/30 mb-6">
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

                {/* Feature Cards */}
                <MagicBento
                    textAutoHide={true}
                    enableStars={true}
                    enableSpotlight={true}
                    enableBorderGlow={true}
                    enableTilt={true}
                    enableMagnetism={true}
                    clickEffect={true}
                    spotlightRadius={300}
                    particleCount={12}
                    glowColor="59, 130, 246"
                    cardData={features}
                />
            </div>
        </section>
    );
}

function PricingPlans() {
    return (
        <section className="py-16 text-center container">
            <h2 className="text-4xl md:text-5xl lg:text-6xl font-bold text-primaryText mb-4">Pricing</h2>
            <p className="text-primaryText/70 text-lg mb-2">Free for now</p>
            <p className="text-muted-foreground">We&apos;re working on pricing plans.</p>
        </section>
    );
}

export default function Home() {
    return <div className="flex flex-col w-full h-full items-center bg-black">
        {/* Fixed Header */}
        <Suspense fallback={null}>
            <Header className="px-6 sm:px-4 container" />
        </Suspense>

        {/* Hero Section */}
        <div id="intro" className={"w-screen h-screen bg-black"}>
            <Introduction />
        </div>

        {/* Desktop Section */}
        <div id="desktop" className={"w-full bg-zinc-900"}>
            <div className="w-full container">
                <DesktopSection />
            </div>
        </div>

        {/* Pricing Section */}
        <div id="pricing" className={"w-full bg-black sm:pt-30 flex items-center justify-center lg:py-25"}>
            <PricingPlans />
        </div>

        {/* Additional Features Section */}
        <div id="more-features" className={"w-full bg-blue-800/10"}>
            <AdditionalFeatures />
        </div>

        {/* Join Waitlist Section */}
        <div id="waitlist" className={"w-full bg-zinc-900 h-[60vh] py-5 min-h-fit items-center flex"}>
            <JoinWaitList />
        </div>

        {/* Footer */}
        <Footer />
    </div>
}
