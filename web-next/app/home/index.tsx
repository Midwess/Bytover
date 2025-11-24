import Header from "@/components/web/header";
import Footer from "@/components/web/footer";

import {Suspense} from "react";
import {Pricing2} from "@/components/pricing2";
import MagicBento from "@/components/MagicBento";
import Introduction from "@/app/home/introduction.tsx";
import { JoinWaitList } from "@/components/join-waitlist";

function FeaturesSection() {
    const features = [
        {
            id: "shelf",
            heading: "Organize Your Files",
            description: "Keep all your files organized in a beautiful shelf interface. Easy drag-and-drop, quick access, and seamless file management.",
            video: "/demo/desktop-shelf.mp4",
            color: '#060010',
        },
        {
            id: "public-share",
            heading: "Public Sharing",
            description: "Share files with anyone using a simple link. Optional password protection keeps your content secure while making sharing effortless.",
            video: "/demo/desktop-share-public.mp4",
            color: '#060010',
        },
        {
            id: "nearby-share",
            heading: "Nearby and P2P Transfer",
            description: "Transfer files directly to any device instantly with Peer to Peer connection. And automatically matching nearby users.",
            video: "/demo/desktop-share-nearby.mp4",
            color: '#060010',
        },
        {
            id: "all-platform",
            heading: "Available on All Platforms",
            description: "Coming soon this year! Native apps for Windows, macOS, iOS, and Android. Experience Bytover seamlessly across all your devices with full feature parity and consistent performance.",
            image: "/demo/mobile_mockup_1.png",
            color: '#060010',
        },
    ];

    return (
        <section className="py-32">
            <div className="w-full container lg:h-[1400px] flex flex-col items-center">
                <div className="mb-12 flex flex-col items-center text-center max-w-3xl">
                    <h2 className="mb-4 text-4xl font-bold text-primaryText md:text-5xl lg:text-6xl">
                        Powerful Features
                    </h2>
                    <p className="text-primaryText/70 text-lg lg:text-xl">
                        Experience seamless file transfer with our intuitive interface. Share files publicly, transfer peer to peer, organize everything in your personal shelf.
                    </p>
                </div>
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
    const plans = [
        {
            id: "free",
            name: "Free",
            description: "Perfect for basic users who need simple peer-to-peer file transfers",
            price: "Free",
            features: [
                {text: "Peer-to-peer transfers with limited bandwidth", included: true},
                {text: "Transfer to your own devices", included: true},
                {text: "Public sharing", included: false},
                {text: "Email sharing", included: false},
            ],
            button: {
                text: "Get Started",
                url: "/transfer",
            },
        },
        {
            id: "pro",
            name: "Pro",
            description: "Advanced features for peer-to-peer transfers across the internet and public sharing.",
            price: "Coming soon",
            features: [
                {text: "Peer-to-peer transfers with unlimited bandwidth", included: true},
                {text: "Transfer to your own devices", included: true},
                {text: "Public sharing with password protection", included: true},
                {text: "Public cloud storage up to 500GB / month", included: true},
                {text: "Send files via email", included: true},
            ],
            button: {
                text: "Join waiting list",
                url: "/transfer",
            },
        },
    ];

    return (
        <Pricing2
            heading="Simple, Transparent Pricing"
            description="Choose the plan that fits your needs. Free for basic transfers, Pro for advanced features."
            showOneTime={false}
            plans={plans}
        />
    );
}

export default function Home() {
    return <div className="flex flex-col w-full h-full items-center bg-black">
        {/* Fixed Header */}
        <Suspense fallback={null}>
            <Header/>
        </Suspense>

        {/* Hero Section */}
       <div id="intro" className={"w-screen h-screen bg-black"}>
            <Introduction/>
        </div>

        {/* Features Section */}
        <div id="features" className={"w-full bg-zinc-900"}>
            <div className="w-full container">
                <FeaturesSection/>
            </div>
        </div>

        {/* Pricing Section */}
        <div id="pricing" className={"w-full bg-black sm:pt-30 lg:pt-2 scroll-mt-20 flex items-center justify-center lg:my-25"}>
            <PricingPlans/>
        </div>

        {/* Join Waitlist Section */}
        <div id="waitlist" className={"w-full bg-zinc-900"}>
            <JoinWaitList/>
        </div>

        {/* Footer */}
        <Footer/>
    </div>
}
