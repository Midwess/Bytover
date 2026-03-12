"use client";

import { Check, Sparkles, Plus } from "lucide-react";
import { motion } from "motion/react";
import { Button } from "./ui/button";

export function Pricing2() {
  return (
    <section className="w-full py-24 md:py-40 bg-black relative">
      <div className="container mx-auto px-4 md:px-6 relative z-10">
        <div className="flex flex-col items-center text-center max-w-3xl mx-auto mb-20 md:mb-32 space-y-6">
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="px-3 py-1 rounded-full text-[10px] font-bold tracking-[0.2em] uppercase bg-zinc-900 text-zinc-500 border border-zinc-800"
          >
            Pricing
          </motion.div>
          <motion.h2 
             initial={{ opacity: 0, y: 20 }}
             whileInView={{ opacity: 1, y: 0 }}
             viewport={{ once: true }}
             className="text-4xl md:text-5xl font-bold text-white tracking-tight"
          >
            Get Bytover. <br />
            <span className="text-zinc-600">Free forever.</span>
          </motion.h2>
          <motion.p 
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ delay: 0.1 }}
            className="text-lg text-zinc-400 max-w-xl font-medium"
          >
            Start with our powerful basic version, or unlock professional features with a one-time purchase.
          </motion.p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-8 max-w-5xl mx-auto">
          {/* Free Plan */}
          <motion.div
            initial={{ opacity: 0, y: 30 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
            className="relative p-8 md:p-10 rounded-[2rem] bg-zinc-950 border border-white/5 flex flex-col justify-between group hover:border-white/10 transition-colors"
          >
            <div className="space-y-8">
              <div className="space-y-3">
                <h3 className="text-xl font-bold text-white tracking-tight">Basic</h3>
                <div className="flex items-baseline gap-1">
                  <span className="text-5xl font-bold text-white">$0</span>
                  <span className="text-zinc-600 font-bold uppercase tracking-widest text-[9px]">USD</span>
                </div>
                <p className="text-xs font-medium text-zinc-500">The essential tool for everyday sharing.</p>
              </div>
              
              <ul className="space-y-3 border-t border-white/5 pt-8">
                {[
                  "Unlimited P2P transfers",
                  "Direct device-to-device",
                  "End-to-end encryption",
                  "Native Shelf UI",
                  "Mobile companion app"
                ].map((feature, i) => (
                  <li key={i} className="flex items-center gap-3 text-zinc-300">
                    <Check className="w-4 h-4 text-zinc-600" />
                    <span className="text-xs font-medium">{feature}</span>
                  </li>
                ))}
              </ul>
            </div>

            <Button className="mt-10 w-full h-12 rounded-xl bg-white text-black hover:bg-zinc-200 transition-all font-bold text-sm active:scale-[0.98]">
              Download Now
            </Button>
          </motion.div>

          {/* Pro Plan (Coming Soon) */}
          <motion.div
            initial={{ opacity: 0, y: 30 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6, delay: 0.2 }}
            className="relative p-8 md:p-10 rounded-[2rem] bg-gradient-to-br from-bluePrimary/10 via-zinc-950 to-zinc-950 border border-bluePrimary/20 flex flex-col justify-between overflow-hidden group"
          >
            <div className="space-y-8">
              <div className="space-y-3">
                <div className="flex items-center gap-2">
                    <h3 className="text-xl font-bold text-white tracking-tight">Pro</h3>
                    <span className="px-1.5 py-0.5 rounded-full text-[8px] font-bold bg-bluePrimary text-white uppercase tracking-widest">Early Access</span>
                </div>
                <div className="flex items-baseline gap-1">
                  <span className="text-5xl font-bold text-white">$15</span>
                  <span className="text-zinc-600 font-bold uppercase tracking-widest text-[9px]">USD</span>
                </div>
                <p className="text-xs font-medium text-zinc-400">One-time purchase. Lifetime updates.</p>
              </div>
              
              <ul className="space-y-3 border-t border-white/5 pt-8">
                {[
                    "Permanent Cloud Storage",
                    "Password Protected Links",
                    "Custom Share Expiration",
                    "Priority Transfer Speeds",
                    "Advanced File Management"
                ].map((feature, i) => (
                  <li key={i} className="flex items-center gap-3 text-zinc-200">
                    <Plus className="w-4 h-4 text-bluePrimary" />
                    <span className="text-xs font-bold">{feature}</span>
                  </li>
                ))}
              </ul>
            </div>

            <div className="mt-12 p-6 rounded-2xl bg-bluePrimary/5 border border-bluePrimary/10 flex flex-col gap-4">
                <div className="flex items-center gap-3">
                    <Sparkles className="w-5 h-5 text-bluePrimary" />
                    <p className="text-sm font-bold text-white">Join the Waitlist</p>
                </div>
                <p className="text-xs font-medium text-zinc-400 leading-relaxed">
                    Be the first to know when Bytover Pro launches. Early adopters get a special discount.
                </p>
            </div>
          </motion.div>
        </div>
      </div>
    </section>
  );
}
