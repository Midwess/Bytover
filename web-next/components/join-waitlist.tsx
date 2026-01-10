'use client'

import core from '@/wasm/wasm_core'
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { AppEventVariantAuthentication, AuthenticationEventVariantFeedback } from "shared_types/types/shared_types"
import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "motion/react";

export function JoinWaitList() {
  const authState = core.useAuthenticationState()
  const isSubmitted = authState?.is_already_feedback;
  const [email, setEmail] = useState<string>('')
  const [message, setMessage] = useState<string>('')
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false)

  useEffect(() => {
    setEmail(authState?.user?.email || '')
  }, [authState?.user?.email, setEmail]);

  const handleSubmit = () => {
    if (isSubmitted || isSubmitting) return;

    setIsSubmitting(true);

    // Send feedback
    core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantFeedback(message, email)));

    // Simulate a brief delay for animation
    setTimeout(() => {
      setIsSubmitting(false);
    }, 800);
  };

  return (
    <section className="relative flex w-full items-center justify-center overflow-hidden py-16">
      <AnimatePresence mode="wait">
        {!isSubmitted ? (
          <motion.div
            key="form"
            initial={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.3 }}
            className="relative z-20 w-full max-w-md flex flex-col items-center"
          >
            <h2 className="relative z-20 text-center font-sans text-4xl font-semibold tracking-tighter md:py-5 lg:text-6xl">
              Feature Request
            </h2>
            <p className="text-md text-muted-foreground mx-auto max-w-xl text-center lg:text-lg px-8">
              Have a feature in mind? Let us know what you&apos;d like to see in Bytover.
            </p>
            <div className="relative z-20 mt-10 flex w-full flex-col gap-3">
              <input
                type="email"
                className="bg-muted-foreground/20 h-10 w-full rounded-xl border border-input px-3 shadow-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] disabled:opacity-50 disabled:cursor-not-allowed"
                placeholder="Enter your email"
                onChange={(e) => setEmail(e.target.value)}
                value={email}
                disabled={isSubmitting || isSubmitted}
                required
              />
              <textarea
                className={cn(
                  "bg-muted text-muted-foreground placeholder:text-muted-foreground/70 min-h-[100px] w-full rounded-xl border border-input p-3 text-base shadow-none resize-none",
                  "focus-visible:outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
                  "md:text-sm disabled:opacity-50 disabled:cursor-not-allowed"
                )}
                onChange={(e) => setMessage(e.target.value)}
                value={message}
                disabled={isSubmitting || isSubmitted}
                placeholder="Tell us what you think or do you have any feature requests? (Optional)"
              />
              <Button
                onClick={handleSubmit}
                disabled={isSubmitting || !email}
                className="bg-bluePrimary text-white h-10 rounded-xl w-full disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSubmitting ? (
                  <span className="flex items-center gap-2">
                    <motion.span
                      animate={{ rotate: 360 }}
                      transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
                      className="w-4 h-4 border-2 border-white border-t-transparent rounded-full"
                    />
                    Sending...
                  </span>
                ) : (
                  "Submit Request"
                )}
              </Button>
            </div>
          </motion.div>
        ) : (
          <motion.div
            key="thankyou"
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.5, type: "spring", stiffness: 200, damping: 20 }}
            className="relative z-20 w-full max-w-md flex flex-col items-center justify-center"
          >
            <motion.div
              initial={{ scale: 0 }}
              animate={{ scale: 1 }}
              transition={{ delay: 0.2, type: "spring", stiffness: 200, damping: 15 }}
              className="mb-6"
            >
              <svg
                className="w-16 h-16 md:w-20 md:h-20 text-green-500"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <motion.path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M5 13l4 4L19 7"
                  initial={{ pathLength: 0 }}
                  animate={{ pathLength: 1 }}
                  transition={{ duration: 0.5, delay: 0.3 }}
                />
              </svg>
            </motion.div>
            <motion.h2
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.4, duration: 0.5 }}
              className="text-center font-sans text-4xl md:text-5xl lg:text-6xl font-semibold tracking-tighter text-primaryText mb-4"
            >
              Thank You!
            </motion.h2>
            <motion.p
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.5, duration: 0.5 }}
              className="text-md text-muted-foreground mx-auto max-w-xl text-center lg:text-lg px-8"
            >
              We&apos;ve received your feedback and will be in touch soon.
            </motion.p>
          </motion.div>
        )}
      </AnimatePresence>
    </section>
  );
};
