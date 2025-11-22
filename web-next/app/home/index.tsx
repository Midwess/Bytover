import {TypingText} from "@/components/animate-ui/text/typing";
import Header from "@/components/web/header";
import {LiquidButton} from '@/components/animate-ui/buttons/liquid'
import Android from '@/public/android.svg'
import apple from '@/public/apple.svg'
import windows from '@/public/windows.svg'
import Image from 'next/image'
import TransferBoard from "@/app/transfer";
import { Suspense } from "react";
import { Pricing2 } from "@/components/pricing2";
import { Feature72 } from "@/components/feature72";

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
            description: "Coming soon this year! Native apps for Windows, macOS, iOS, and Android. Experience BitBridge seamlessly across all your devices with full feature parity and consistent performance.",
            image: "/demo/bitbridge_mockup_1.png",
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
                { text: "Peer-to-peer transfer with limited bandwidth", included: true },
                { text: "Public sharing", included: false },
            ],
            button: {
                text: "Get Started",
                url: "/transfer",
            },
        },
        {
            id: "pro",
            name: "Pro",
            description: "Advanced features for peer-to-peer transfers across the internet and public sharing",
            price: "Coming soon",
            features: [
                { text: "Peer-to-peer transfer with unlimited bandwidth", included: true },
                { text: "Public sharing with password protection", included: true },
                { text: "Send files via email", included: true },
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
        <div className={'relative flex flex-col w-full items-center gap-10 pb-16 pt-32'}>

            <div
                className="absolute top-0 h-full w-screen bg-black bg-[radial-gradient(ellipse_80%_80%_at_50%_-20%,rgba(124,255,121,0.2),rgba(255,255,255,0))]">
            </div>
            <div className={'flex flex-col items-center gap-4 container z-2'}>
                <h2 className="text-lg tracking-widest text-greenSecondary text-center">
                    Powering your productivity
                </h2>
                <TypingText
                    delay={200}
                    duration={15}
                    className="text-5xl font-bold text-center h1"
                    text="Seamless file transfer that you can trust"
                    cursor
                    cursorClassName="h-9"
                />
            </div>
            <div className={"flex flex-col items-center gap-4"}>
                <h2 className={"font-bold text-lg text-primaryText/80"}>Available on all platforms</h2>
                <div className={"flex flex-row gap-2"}>
                    <LiquidButton className={"flex flex-row gap-3"}>
                        <Image src={Android} alt="Android" width={20} height={20}/>
                        Android
                    </LiquidButton>
                    <LiquidButton className={"flex flex-row gap-3"}>
                        <Image src={apple} alt="iOS" width={20} height={20}/>
                        iOS
                    </LiquidButton>
                    <LiquidButton className={"flex flex-row gap-3"}>
                        <Image src={windows} alt="Windows" width={20} height={20}/>
                        Windows
                    </LiquidButton>
                    <LiquidButton className={"flex flex-row gap-3"}>
                        <Image src={apple} alt="Mac" width={20} height={20}/>
                        Mac OS
                    </LiquidButton>
                </div>
            </div>
        </div>

        {/* Transfer Board Section */}
        <div id="transfer" className={"container flex flex-col py-16 scroll-mt-20"}>
            <Suspense fallback={null}>
                <TransferBoard/>
            </Suspense>
        </div>

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
