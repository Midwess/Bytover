'use client'

import core from '@/wasm/wasm_core'
import { Button } from "@/components/ui/button";
import { AppEventVariantAuthentication, AuthenticationEventVariantFeedback } from "shared_types/types/shared_types"
import { useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { Check } from "lucide-react";
import { cn } from "@/lib/utils";

export interface FeedbackFormProps {
    className?: string;
}

export function useIsFeedbackSubmitted(): boolean {
    const authState = core.useAuthenticationState();
    return !!authState?.is_already_feedback;
}

export function FeedbackForm({ className }: FeedbackFormProps) {
    const isSubmitted = useIsFeedbackSubmitted();
    const [email, setEmail] = useState<string>('');
    const [message, setMessage] = useState<string>('');
    const [isSubmitting, setIsSubmitting] = useState<boolean>(false);

    const handleSubmit = () => {
        if (isSubmitted || isSubmitting || !email) return;

        setIsSubmitting(true);
        core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantFeedback(message, email)));

        setTimeout(() => {
            setIsSubmitting(false);
        }, 800);
    };

    return (
        <div className={cn("w-full", className)}>
            <AnimatePresence mode="wait">
                {!isSubmitted ? (
                    <motion.div
                        key="form"
                        initial={{ opacity: 0, y: 20 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, y: -20 }}
                        transition={{ duration: 0.5 }}
                        className="w-full space-y-10"
                    >
                        <div className="space-y-1 text-left">
                            <label className="text-xs font-bold tracking-[0.1em] uppercase text-zinc-500 ml-0.5">Email address</label>
                            <input
                                type="email"
                                className="w-full h-10 bg-transparent border-b border-zinc-800 focus:border-white transition-all text-white placeholder:text-zinc-700 focus:outline-none font-medium text-base px-0.5"
                                placeholder="alex@example.com"
                                onChange={(e) => setEmail(e.target.value)}
                                value={email}
                                disabled={isSubmitting}
                                required
                            />
                        </div>

                        <div className="space-y-1 text-left">
                            <label className="text-xs font-bold tracking-[0.1em] uppercase text-zinc-500 ml-0.5">Message</label>
                            <textarea
                                className="w-full min-h-[100px] bg-transparent border-b border-zinc-800 focus:border-white transition-all text-white placeholder:text-zinc-700 focus:outline-none resize-none font-medium text-base py-1 px-0.5"
                                placeholder="Tell us what you&apos;d like to see next..."
                                onChange={(e) => setMessage(e.target.value)}
                                value={message}
                                disabled={isSubmitting}
                            />
                        </div>

                        <div className="pt-4">
                            <Button
                                onClick={handleSubmit}
                                disabled={isSubmitting || !email}
                                className="group w-full h-14 bg-white text-black hover:bg-zinc-200 rounded-full transition-all flex items-center justify-center gap-3 text-base font-bold active:scale-[0.98] border-none shadow-none"
                            >
                                {isSubmitting ? (
                                    <span className="flex items-center gap-2">
                                        <motion.span
                                            animate={{ rotate: 360 }}
                                            transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
                                            className="w-5 h-5 border-2 border-black border-t-transparent rounded-full"
                                        />
                                        Sending...
                                    </span>
                                ) : (
                                    "Submit Feedback"
                                )}
                            </Button>
                            <p className="text-xs text-center text-zinc-600 mt-8 font-bold tracking-widest uppercase">We read every single message.</p>
                        </div>
                    </motion.div>
                ) : (
                    <motion.div
                        key="thankyou"
                        initial={{ opacity: 0, scale: 0.95 }}
                        animate={{ opacity: 1, scale: 1 }}
                        className="flex flex-col items-center justify-center py-20 space-y-6"
                    >
                        <div className="flex items-center justify-center">
                            <Check className="w-12 h-12 text-green-500" />
                        </div>
                        <div className="text-center space-y-2">
                            <h2 className="text-3xl font-bold text-white tracking-tight">Thank You.</h2>
                            <p className="text-base text-zinc-500 font-medium">
                                Your input helps us build a better Bytover.
                            </p>
                        </div>
                    </motion.div>
                )}
            </AnimatePresence>
        </div>
    );
}
