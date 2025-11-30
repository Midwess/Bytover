'use client';

import { MotionEffect } from '@/components/animate-ui/effects/motion-effect';

interface Feature {
    title: string;
    description: string;
}

const features: Feature[] = [
    {
        title: "Peer to peer with a relay server",
        description: "All transfer are truely peer to peer unless the sender or receiver is behind a NAT or firewall.",
    },
    {
        title: "No file size limits.",
        description: "Transfer files of any size without restrictions or compression.",
    },
    {
        title: "Cross-platform support.",
        description: "Works seamlessly across Windows, macOS, Linux, iOS, and Android.",
    },
    {
        title: "Folder transfer.",
        description: "Transfer entire folders without any zip processing.",
    },
    {
        title: "Nearby transfers.",
        description: "Auto detect and transfer files to nearby devices.",
    },
    {
        title: "To email inbox.",
        description: "Send files to multiple people simultaneously with one link.",
    },
    {
        title: "Public transfer with password protected link.",
        description: "Share files with anyone using a simple link. Optional password protection keeps your content secure while making sharing effortless.",
    },
    {
        title: "Public url is ready right after transfer.",
        description: "No need to wait for the transfer to finish before sharing. The public url is ready right after the transfer is completed.",
    }
];

export function AdditionalFeatures() {
    return (
        <section className="w-full py-20 md:py-32">
            <div className="container mx-auto px-4 md:px-6">
                {/* Heading */}
                <MotionEffect
                    slide={{ direction: 'up', offset: 30 }}
                    fade
                    delay={0.1}
                    inView
                    inViewOnce
                >
                    <h2 className="text-4xl md:text-5xl lg:text-6xl font-bold text-center mb-16 md:mb-20 text-primaryText">
                        There's more
                    </h2>
                </MotionEffect>

                {/* Features Grid */}
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8 md:gap-x-12 md:gap-y-12 max-w-7xl mx-auto">
                    {features.map((feature, index) => (
                        <MotionEffect
                            key={index}
                            slide={{ direction: 'up', offset: 20 }}
                            fade
                            delay={0.2 + Math.min(index, 8) * 0.05}
                            inView
                            inViewOnce
                        >
                            <div className="space-y-2">
                                <h3 className="text-lg md:text-xl font-bold text-primaryText">
                                    {feature.title}
                                </h3>
                                <p className="text-primaryText/70 text-base leading-relaxed">
                                    {feature.description}
                                </p>
                            </div>
                        </MotionEffect>
                    ))}
                </div>
            </div>
        </section>
    );
}
