'use client';

import { DownloadPlatforms } from "@/components/download-platforms";
import { getAssetUrl } from "@/utils/asset-url";
import { motion } from "motion/react";
import { Monitor, Smartphone, Layout, Globe, Command, Check } from "lucide-react";

export function DesktopSection() {
    const mainFeatures = [
        {
            icon: Layout,
            title: "The Shelf",
            description: "A temporary storage for your files. Shake your cursor to reveal, drop to save. It's that simple.",
            video: getAssetUrl("/demo/desktop-shelf.mp4"),
        },
        {
            icon: Globe,
            title: "Cloud Sharing",
            description: "Instant public links for any file or folder. No ZIP compression needed, structure is preserved.",
            video: getAssetUrl("/demo/desktop-share-public.mp4"),
        }
    ];

    return (
        <section className="w-full relative py-24 md:py-40 bg-black">
            <div className="container mx-auto px-4 md:px-6">
                {/* Section Header */}
                <div className="flex flex-col items-center text-center max-w-3xl mx-auto mb-24 md:mb-32 space-y-8">
                    <motion.div
                        initial={{ opacity: 0, y: 10 }}
                        whileInView={{ opacity: 1, y: 0 }}
                        viewport={{ once: true }}
                        className="px-3 py-1 rounded-full text-[10px] font-bold tracking-[0.2em] uppercase bg-bluePrimary/10 text-bluePrimary border border-bluePrimary/20"
                    >
                        Desktop Experience
                    </motion.div>
                    
                    <motion.h2
                        initial={{ opacity: 0, y: 20 }}
                        whileInView={{ opacity: 1, y: 0 }}
                        viewport={{ once: true }}
                        transition={{ delay: 0.1 }}
                        className="text-4xl md:text-5xl font-bold text-white tracking-tight leading-[1.1]"
                    >
                        Native. Powerful. <br />
                        <span className="text-zinc-600">Effortless.</span>
                    </motion.h2>
                    
                    <motion.p
                        initial={{ opacity: 0, y: 20 }}
                        whileInView={{ opacity: 1, y: 0 }}
                        viewport={{ once: true }}
                        transition={{ delay: 0.2 }}
                        className="text-base md:text-lg text-zinc-400 max-w-xl font-medium"
                    >
                        Bytover Desktop is designed to feel like a part of your operating system. 
                        No bloated UI, just the tools you need when you need them.
                    </motion.p>
                </div>

                {/* Main Feature Showcases - Alternating/Large Layout */}
                <div className="space-y-32 md:space-y-48">
                    {mainFeatures.map((feature, index) => (
                        <motion.div
                            key={index}
                            initial={{ opacity: 0, y: 40 }}
                            whileInView={{ opacity: 1, y: 0 }}
                            viewport={{ once: true }}
                            transition={{ duration: 0.8 }}
                            className={`flex flex-col ${index % 2 === 0 ? 'lg:flex-row' : 'lg:flex-row-reverse'} items-center gap-16 lg:gap-24`}
                        >
                            {/* Visual Component */}
                            <div className="flex-1 w-full relative overflow-visible">
                                <div className="relative group flex items-center justify-center overflow-visible">
                                    <video
                                        src={feature.video}
                                        autoPlay
                                        loop
                                        muted
                                        playsInline
                                        className="w-[120%] max-w-none h-auto object-contain opacity-90 group-hover:opacity-100 transition-all duration-700 group-hover:scale-105 border-0 outline-none bg-transparent"
                                    />
                                </div>
                            </div>

                            {/* Text Component */}
                            <div className="flex-1 max-w-md space-y-4">
                                <div className="w-10 h-10 rounded-xl bg-zinc-900 border border-white/10 flex items-center justify-center">
                                    <feature.icon className="w-5 h-5 text-bluePrimary" />
                                </div>
                                <h3 className="text-2xl md:text-3xl font-bold text-white tracking-tight">
                                    {feature.title}
                                </h3>
                                <p className="text-base text-zinc-400 leading-relaxed font-medium">
                                    {feature.description}
                                </p>
                                <ul className="space-y-3 pt-4">
                                    {["Fast", "Secure", "Reliable"].map((item) => (
                                        <li key={item} className="flex items-center gap-3 text-sm font-bold text-zinc-500 uppercase tracking-widest">
                                            <Check className="w-4 h-4 text-bluePrimary" />
                                            {item}
                                        </li>
                                    ))}
                                </ul>
                            </div>
                        </motion.div>
                    ))}
                </div>

                {/* Secondary Features Grid - "There's More" Section */}
                <div className="mt-48 pt-24 border-t border-white/5">
                    <div className="text-center mb-16">
                        <h3 className="text-xs font-bold tracking-[0.3em] uppercase text-zinc-600">Built for Professionals</h3>
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-12">
                         {[
                            { icon: Command, title: "Global Hotkeys", text: "Control your transfers with customizable system-wide shortcuts." },
                            { icon: Monitor, title: "Multi-Monitor", text: "Seamlessly move files across multiple displays and workspaces." },
                            { icon: Smartphone, title: "Mobile Link", text: "Instantly pair with your phone for direct device-to-device transfer." }
                         ].map((item, i) => (
                            <motion.div 
                                key={i}
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                transition={{ delay: i * 0.1 }}
                                className="space-y-4 text-center md:text-left"
                            >
                                <item.icon className="w-6 h-6 text-zinc-400 mx-auto md:mx-0" />
                                <h4 className="text-lg font-bold text-white">{item.title}</h4>
                                <p className="text-zinc-500 text-sm leading-relaxed">{item.text}</p>
                            </motion.div>
                         ))}
                    </div>
                </div>

                {/* Final Desktop CTA */}
                <motion.div
                    initial={{ opacity: 0, scale: 0.95 }}
                    whileInView={{ opacity: 1, scale: 1 }}
                    viewport={{ once: true }}
                    className="mt-40 text-center space-y-8"
                >
                    <h3 className="text-3xl font-bold text-white">Experience it today.</h3>
                    <DownloadPlatforms />
                </motion.div>
            </div>
        </section>
    );
}
