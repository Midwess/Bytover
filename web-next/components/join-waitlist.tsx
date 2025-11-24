'use client'

import { BackgroundLines } from "@/components/aceternity/background-lines";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const JoinWaitList = () => {
  return (
    <section className="relative flex h-[50vh] md:h-[60vh] lg:h-[70vh] w-screen items-center justify-center overflow-hidden py-16">
      <BackgroundLines className="relative container flex w-full flex-col items-center justify-center px-4 h-full overflow-hidden bg-transparent">
        <h2 className="relative z-20 py-2 text-center font-sans text-5xl font-semibold tracking-tighter md:py-10 lg:text-8xl">
          Join the Waitlist
        </h2>
        <p className="text-md text-muted-foreground mx-auto max-w-xl text-center lg:text-lg">
          Get early access to Pro features: unlimited bandwidth, secure sharing, cloud storage, and more.
        </p>
        <div className="relative z-20 mt-10 flex w-full max-w-md flex-col gap-3">
          <input
            type="email"
            className="bg-muted-foreground/20 h-10 w-full rounded-xl border border-input px-3 shadow-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]"
            placeholder="Enter your email"
            required
          />
          <textarea
            className={cn(
              "bg-muted text-muted-foreground placeholder:text-muted-foreground/70 min-h-[100px] w-full rounded-xl border border-input p-3 text-base shadow-none resize-none",
              "focus-visible:outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
              "md:text-sm"
            )}
            placeholder="Tell us what you think or do you have any feature requests? (Optional)"
          />
          <Button className="bg-bluePrimary text-white h-10 rounded-xl w-full">Join the Waitlist</Button>
        </div>
      </BackgroundLines>
    </section>
  );
};

export { JoinWaitList };
