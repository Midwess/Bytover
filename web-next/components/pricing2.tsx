"use client";

import { Check, Plus, Loader2 } from "lucide-react";
import { motion } from "motion/react";
import { Button } from "./ui/button";
import { useState, useEffect } from "react";
import core from '@/wasm/wasm_core';
import { AppEventVariantAuthentication, AuthenticationEventVariantFeedback } from "shared_types/types/shared_types";
import { Input } from "./ui/input";

export function Pricing2() {
  const authState = core.useAuthenticationState();
  const [email, setEmail] = useState<string>('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isJoined, setIsJoined] = useState(false);

  useEffect(() => {
    if (authState?.user?.email) {
      setEmail(authState.user.email);
    }
  }, [authState?.user?.email]);

  const handleJoinWaitlist = async () => {
    if (!email || isSubmitting) return;
    
    setIsSubmitting(true);
    try {
      core.update(new AppEventVariantAuthentication(
        new AuthenticationEventVariantFeedback('user joined wait list', email)
      ));
      setIsJoined(true);
    } catch (error) {
      console.error("Failed to join waitlist:", error);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <section className="w-full py-24 md:py-40 bg-black relative" id="pricing">
      <div className="container mx-auto px-4 md:px-6 relative z-10">
        <div className="flex flex-col items-center text-center max-w-3xl mx-auto mb-20 md:mb-32 space-y-6">
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="px-3 py-1 rounded-full text-xs font-bold tracking-[0.2em] uppercase bg-zinc-900 text-zinc-500 border border-zinc-800"
          >
            Pricing
          </motion.div>
          <motion.h2 
             initial={{ opacity: 0, y: 20 }}
             whileInView={{ opacity: 1, y: 0 }}
             viewport={{ once: true }}
             className="text-4xl md:text-5xl font-bold text-white tracking-tight"
          >
            One-time purchase. <br />
            <span className="text-zinc-600">Lifetime access.</span>
          </motion.h2>
          <motion.p 
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ delay: 0.1 }}
            className="text-xl text-zinc-400 max-w-xl font-medium"
          >
            Experience the full power of Bytover with a single payment. No subscriptions, just seamless file orchestration.
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
                <h3 className="text-xl font-bold text-white tracking-tight">Free</h3>
                <div className="flex items-baseline gap-1">
                  <span className="text-5xl font-bold text-white">$0</span>
                  <span className="text-zinc-600 font-bold uppercase tracking-widest text-[9px]">USD</span>
                </div>
                <p className="text-sm font-medium text-zinc-500">Essential features for everyday sharing.</p>
              </div>
              
              <ul className="space-y-3 border-t border-white/5 pt-8">
                {[
                  "Cloud-only transfers",
                  "Capped network speeds",
                  "End-to-end encryption",
                  "Advanced shelf management",
                  "Manage multiple concurrent shelves"
                ].map((feature, i) => (
                  <li key={i} className="flex items-center gap-3 text-zinc-400">
                    <Check className="w-4 h-4 text-zinc-800" />
                    <span className="text-sm font-medium">{feature}</span>
                  </li>
                ))}
              </ul>
            </div>

            <Button className="mt-10 w-full h-12 rounded-xl bg-zinc-900 text-zinc-400 hover:bg-zinc-800 transition-all font-bold text-sm active:scale-[0.98]">
              Get Started
            </Button>
          </motion.div>

          {/* Full Plan */}
          <motion.div
            initial={{ opacity: 0, y: 30 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6, delay: 0.2 }}
            className="relative p-8 md:p-10 rounded-[2rem] bg-gradient-to-br from-blue-600/10 via-zinc-950 to-zinc-950 border border-blue-600/30 flex flex-col justify-between overflow-hidden group shadow-[0_0_50px_-12px_rgba(37,99,235,0.3)]"
          >
            <div className="space-y-8">
              <div className="space-y-3">
                <div className="flex items-center gap-2">
                    <h3 className="text-xl font-bold text-white tracking-tight">Full</h3>
                    <span className="px-1.5 py-0.5 rounded-full text-xs font-bold bg-blue-600 text-white uppercase tracking-widest">Recommended</span>
                </div>
                <div className="flex items-baseline gap-1">
                  <span className="text-5xl font-bold text-white">$12.99</span>
                  <span className="text-zinc-600 font-bold uppercase tracking-widest text-[9px]">USD</span>
                </div>
                <p className="text-sm font-medium text-blue-600/60">One-time payment. Own it forever.</p>
              </div>
              
              <ul className="space-y-3 border-t border-white/5 pt-8">
                {[
                    "Everything in Free, plus:",
                    "Direct P2P: Files stay on your local machine",
                    "Zero upload time: Shareable URLs ready instantly",
                    "Native folder sharing without zipping",
                    "Uncapped network transfer speeds"
                ].map((feature, i) => (
                  <li key={i} className="flex items-center gap-3 text-zinc-100">
                    <Plus className="w-4 h-4 text-blue-600" />
                    <span className="text-sm font-bold">{feature}</span>
                  </li>
                ))}
              </ul>
            </div>

            <div className="mt-12 p-6 rounded-2xl bg-blue-600/5 border border-blue-600/10 flex flex-col gap-4">
                <div className="flex items-center gap-3">
                    <p className="text-sm font-bold text-white">Join the Waitlist</p>
                </div>
                {isJoined ? (
                    <div className="text-xs font-bold text-emerald-500 bg-emerald-500/10 p-3 rounded-lg border border-emerald-500/20 text-center">
                        You&apos;re on the list! We&apos;ll be in touch.
                    </div>
                ) : (
                    <div className="flex flex-col gap-3">
                        <Input 
                            type="email"
                            placeholder="your@email.com"
                            className="bg-zinc-950 border-white/10 h-10 text-xs focus-visible:ring-blue-600"
                            value={email}
                            onChange={(e) => setEmail(e.target.value)}
                        />
                        <Button 
                            onClick={handleJoinWaitlist}
                            disabled={isSubmitting || !email}
                            className="w-full h-10 rounded-xl bg-blue-600 text-white hover:bg-blue-600/90 transition-all font-bold text-xs shadow-[0_0_20px_-5px_rgba(37,99,235,0.3)]"
                        >
                            {isSubmitting ? <Loader2 className="w-4 h-4 animate-spin" /> : "Join the Waitlist"}
                        </Button>
                    </div>
                )}
            </div>
          </motion.div>
        </div>
      </div>
    </section>
  );
}

