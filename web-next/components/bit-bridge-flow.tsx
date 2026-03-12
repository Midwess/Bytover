'use client';

import { motion } from 'motion/react';
import {
    ArrowRight,
    ShieldCheck,
    Zap,
    Link as LinkIcon,
    Monitor,
    Globe,
    Lock,
    Cpu,
    CloudOff,
} from 'lucide-react';

const subFeatures = [
    {
        icon: CloudOff,
        title: "No cloud storage middleman",
        description: "Your files are never uploaded to a server. They stay on your device until they reach the recipient."
    },
    {
        icon: ShieldCheck,
        title: "Private, fast connections by default",
        description: "End-to-end encryption ensures your data is only visible to the recipient."
    },
    {
        icon: Cpu,
        title: "Peer-to-peer streaming",
        description: "Data flows directly from disk to browser. Real-time transfer without the wait."
    }
];

export function BitBridgeFlow() {
    return (
        <section className="w-full py-24 md:py-40 bg-black overflow-hidden">
            <div className="container mx-auto px-4 md:px-6">
                <div className="flex flex-col lg:flex-row items-center gap-16 lg:gap-24">
                    {/* Left Side: Content */}
                    <div className="flex-1 space-y-8 max-w-2xl">
                        <motion.div
                            initial={{ opacity: 0, y: 10 }}
                            whileInView={{ opacity: 1, y: 0 }}
                            viewport={{ once: true }}
                            className="inline-flex items-center px-3 py-1 rounded-full text-[10px] font-bold tracking-[0.2em] uppercase bg-emerald-500/10 text-emerald-500 border border-emerald-500/20"
                        >
                            Direct Peer-to-Peer
                        </motion.div>
                        
                        <div className="space-y-4">
                            <motion.h2 
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                className="text-4xl md:text-6xl font-bold text-white tracking-tight leading-[1.1]"
                            >
                                Direct streaming.<br />
                                <span className="text-zinc-500">No middleman.</span>
                            </motion.h2>
                            
                            <motion.p 
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true }}
                                transition={{ delay: 0.1 }}
                                className="text-lg text-zinc-400 font-medium max-w-xl"
                            >
                                Skip the cloud upload. Bit Bridge establishes a secure tunnel directly between your machine and the receiver's browser.
                            </motion.p>
                        </div>

                        <motion.a 
                            href="#waitlist"
                            initial={{ opacity: 0, y: 20 }}
                            whileInView={{ opacity: 1, y: 0 }}
                            viewport={{ once: true }}
                            transition={{ delay: 0.2 }}
                            className="inline-flex items-center text-white font-bold group"
                        >
                            Learn more 
                            <ArrowRight className="ml-2 w-4 h-4 transition-transform group-hover:translate-x-1" />
                        </motion.a>

                        <div className="pt-8 space-y-8 border-t border-white/5">
                            {subFeatures.map((feature, index) => (
                                <motion.div
                                    key={index}
                                    initial={{ opacity: 0, x: -20 }}
                                    whileInView={{ opacity: 1, x: 0 }}
                                    viewport={{ once: true }}
                                    transition={{ delay: 0.3 + index * 0.1 }}
                                    className="flex items-start gap-4"
                                >
                                    <div className="mt-1">
                                        <feature.icon className="w-5 h-5 text-zinc-400" />
                                    </div>
                                    <div className="space-y-1">
                                        <h3 className="text-white font-bold">
                                            {feature.title}
                                        </h3>
                                        <p className="text-zinc-500 text-sm font-medium leading-relaxed">
                                            {feature.description}
                                        </p>
                                    </div>
                                </motion.div>
                            ))}
                        </div>
                    </div>

                    {/* Right Side: Visual */}
                    <div className="flex-1 w-full relative">
                        <motion.div
                            initial={{ opacity: 0, scale: 0.95 }}
                            whileInView={{ opacity: 1, scale: 1 }}
                            viewport={{ once: true }}
                            className="relative aspect-square max-w-[500px] mx-auto bg-zinc-900/50 rounded-3xl border border-white/5 overflow-hidden p-8 flex flex-col justify-between"
                        >
                            {/* Background decoration */}
                            <div className="absolute inset-0 bg-gradient-to-b from-emerald-500/5 to-transparent pointer-events-none" />
                            
                            {/* Sender Node */}
                            <div className="relative z-10 bg-zinc-950/80 border border-white/10 rounded-2xl p-5 shadow-2xl">
                                <div className="flex items-center gap-4">
                                    <div className="w-10 h-10 rounded-full bg-emerald-500/20 flex items-center justify-center">
                                        <Monitor className="w-5 h-5 text-emerald-500" />
                                    </div>
                                    <div className="flex-1">
                                        <div className="text-white font-bold text-sm">MacBook Pro (Sender)</div>
                                        <div className="text-zinc-500 text-xs font-mono">bit-bridge-app-v2.1</div>
                                    </div>
                                    <div className="flex items-center gap-1.5">
                                        <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
                                        <span className="text-[10px] text-emerald-500 font-bold uppercase tracking-wider">Live</span>
                                    </div>
                                </div>
                            </div>

                            {/* Connection Flow Area */}
                            <div className="flex-1 relative flex flex-col items-center justify-center py-4">
                                
                                {/* Bypassed Cloud - Floating Left */}
                                <motion.div 
                                    animate={{ y: [0, -12, 0] }}
                                    transition={{ duration: 4, repeat: Infinity, ease: "easeInOut" }}
                                    className="absolute left-[5%] top-[10%] z-20"
                                >
                                    <div className="flex flex-col items-center gap-2 opacity-40 hover:opacity-80 transition-opacity">
                                        <div className="p-4 rounded-2xl bg-zinc-800/80 border border-white/20 relative backdrop-blur-sm">
                                            <CloudOff className="w-10 h-10 text-zinc-300" />
                                            {/* Diagonal Strike */}
                                            <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-full h-0.5 bg-red-500/60 rotate-45" />
                                        </div>
                                        <span className="text-[8px] font-bold text-zinc-400 uppercase tracking-[0.2em] whitespace-nowrap">Bypassed Cloud</span>
                                    </div>
                                </motion.div>

                                {/* Flow Path (The Secure Tunnel) */}
                                <div className="w-2 h-full bg-zinc-800/30 relative rounded-full overflow-hidden border-x border-white/5">
                                    <motion.div 
                                        animate={{ y: [0, 300] }}
                                        transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
                                        className="absolute top-[-100px] left-0 w-full h-32 bg-gradient-to-b from-transparent via-emerald-400 to-transparent shadow-[0_0_15px_rgba(52,211,153,0.5)]"
                                    />
                                </div>

                                {/* Flow Stats Badge - Floating Right */}
                                <motion.div 
                                    animate={{ y: [0, 10, 0] }}
                                    transition={{ duration: 5, repeat: Infinity, ease: "easeInOut", delay: 0.5 }}
                                    className="absolute right-[2%] top-[15%] z-30"
                                >
                                    <div className="bg-zinc-900/95 border border-emerald-500/40 rounded-2xl p-4 flex flex-col gap-3 backdrop-blur-md shadow-2xl max-w-[160px]">
                                        <div className="flex items-center gap-2">
                                            <div className="w-6 h-6 rounded-full bg-emerald-500 flex items-center justify-center flex-shrink-0">
                                                <Zap className="w-3 h-3 text-black fill-current" />
                                            </div>
                                            <span className="text-[10px] font-black text-white uppercase tracking-wider">Direct P2P</span>
                                        </div>
                                        <div className="space-y-2 pt-2 border-t border-white/5">
                                            <div className="flex justify-between items-center gap-4">
                                                <span className="text-[8px] font-bold text-zinc-500 uppercase">Speed</span>
                                                <span className="text-[10px] font-mono text-emerald-400">1.2 Gbps</span>
                                            </div>
                                            <div className="flex justify-between items-center gap-4">
                                                <span className="text-[8px] font-bold text-zinc-500 uppercase">Latency</span>
                                                <span className="text-[10px] font-mono text-emerald-400">&lt;2ms</span>
                                            </div>
                                        </div>
                                        <div className="flex items-center gap-1.5 pt-1">
                                            <ShieldCheck className="w-3 h-3 text-emerald-500" />
                                            <span className="text-[8px] font-bold text-emerald-500/80 uppercase">E2E Secure</span>
                                        </div>
                                    </div>
                                </motion.div>

                                {/* Connecting Line Visuals (Subtle glow/wires) */}
                                <svg className="absolute inset-0 w-full h-full pointer-events-none opacity-20" preserveAspectRatio="none">
                                    <motion.path 
                                        d="M 120,80 Q 180,100 250,110" 
                                        stroke="white" strokeWidth="1" fill="none" strokeDasharray="4 4"
                                        initial={{ pathLength: 0 }}
                                        whileInView={{ pathLength: 1 }}
                                    />
                                    <motion.path 
                                        d="M 380,140 Q 320,130 250,120" 
                                        stroke="emerald" strokeWidth="1" fill="none" strokeDasharray="4 4"
                                        initial={{ pathLength: 0 }}
                                        whileInView={{ pathLength: 1 }}
                                    />
                                </svg>

                                {/* Particles */}
                                <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
                                    {[...Array(8)].map((_, i) => (
                                        <motion.div
                                            key={i}
                                            animate={{ 
                                                y: [20 * (i - 4), 60 * (i - 2)],
                                                opacity: [0, 0.6, 0],
                                                scale: [0.5, 1, 0.5]
                                            }}
                                            transition={{ 
                                                duration: 1 + Math.random(), 
                                                repeat: Infinity,
                                                delay: i * 0.15 
                                            }}
                                            className="absolute w-1 h-1 bg-emerald-400/40 rounded-full blur-[1px]"
                                            style={{ 
                                                left: `${48 + Math.random() * 4}%`,
                                                top: '30%' 
                                            }}
                                        />
                                    ))}
                                </div>
                            </div>

                            {/* Receiver Node */}
                            <div className="relative z-10 bg-zinc-950/80 border border-white/10 rounded-2xl p-5 shadow-2xl">
                                <div className="flex items-center gap-4">
                                    <div className="w-10 h-10 rounded-full bg-blue-500/20 flex items-center justify-center">
                                        <Globe className="w-5 h-5 text-blue-400" />
                                    </div>
                                    <div className="flex-1">
                                        <div className="text-white font-bold text-sm">Chrome Browser (Receiver)</div>
                                        <div className="text-zinc-500 text-xs font-mono truncate max-w-[200px]">https://bytover.com/s/7x92-k2m...</div>
                                    </div>
                                    <div className="flex items-center gap-2">
                                        <Lock className="w-3.5 h-3.5 text-zinc-600" />
                                    </div>
                                </div>
                            </div>
                        </motion.div>
                    </div>
                </div>
            </div>
        </section>
    );
}
