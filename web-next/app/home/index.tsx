import {TypingText} from "@/components/animate-ui/text/typing";
import Header from "@/components/web/header";

import {Suspense} from "react";
import {Pricing2} from "@/components/pricing2";
import {Feature72} from "@/components/feature72";
import Introduction from "@/app/home/introduction.tsx";

function FeaturesSection() {
    const features = [
        {
            id: "shelf",
            heading: "Organize Your Files",
            description: "Keep all your files organized in a beautiful shelf interface. Easy drag-and-drop, quick access, and seamless file management.",
            video: "/demo/desktop-shelf.mp4",
        },
        {
            id: "public-share",
            heading: "Public Sharing",
            description: "Share files with anyone using a simple link. Optional password protection keeps your content secure while making sharing effortless.",
            video: "/demo/desktop-share-public.mp4",
        },
        {
            id: "nearby-share",
            heading: "Nearby and P2P Transfer",
            description: "Transfer files directly to any device instantly with Peer to Peer connection. And automatically matching nearby users.",
            video: "/demo/desktop-share-nearby.mp4",
        },
        {
            id: "all-platform",
            heading: "Available on All Platforms",
            description: "Coming soon this year! Native apps for Windows, macOS, iOS, and Android. Experience Bytover seamlessly across all your devices with full feature parity and consistent performance.",
            image: "/demo/mobile_mockup_1.png",
        },
    ];

    return (
        <Feature72
            title="Powerful Features"
            description="Experience seamless file transfer with our intuitive interface. Share files publicly, transfer peer to peer, organize everything in your personal shelf."
            features={features}
        />
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
                text: "Buy Now",
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
       <div id="intro" className={"w-screen h-screen"}>
            <Introduction/>
        </div>

        {/* Transfer Board Section */}
        {/*<div id="transfer" className={"container flex flex-col py-16 scroll-mt-20"}>*/}
        {/*    <Suspense fallback={null}>*/}
        {/*        <TransferBoard/>*/}
        {/*    </Suspense>*/}
        {/*</div>*/}

        {/* Features Section */}
        <div id="features" className={"w-full bg-black py-16 scroll-mt-20"}>
            <div className="container">
                <FeaturesSection/>
            </div>
        </div>

        {/* Pricing Section */}
        <div id="pricing" className={"w-full bg-black scroll-mt-20"}>
            <PricingPlans/>
        </div>

        {/* Footer Spacing */}
        <div className={"h-24 w-full"}></div>
    </div>
}
