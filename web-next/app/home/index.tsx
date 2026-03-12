import Header from "@/components/web/header";
import Footer from "@/components/web/footer";

import { Suspense } from "react";
import Introduction from "@/app/home/introduction.tsx";
import { JoinWaitList } from "@/components/join-waitlist";
import { AdditionalFeatures } from "@/components/additional-features";
import { BentoFeatures } from "@/components/bento-features";
import { BitBridgeFlow } from "@/components/bit-bridge-flow";
import { Pricing2 } from "@/components/pricing2";

export default function Home() {
    return (
        <div className="min-h-screen w-screen bg-black relative overflow-x-hidden selection:bg-blue-500 selection:text-white font-inter">
            <Suspense fallback={null}>
                <Header className="px-3" />
            </Suspense>

            <main>
                <section id="intro">
                    <Introduction />
                </section>

                <div className="space-y-0">
                    <BentoFeatures />

                    <div id="pricing">
                        <Pricing2 />
                    </div>

                    <div id="more-features">
                        <AdditionalFeatures />
                    </div>

                    <div id="waitlist" className="w-full">
                        <JoinWaitList />
                    </div>
                </div>
            </main>

            <Footer className="bg-black border-t border-white/5" />
        </div>
    );
}
