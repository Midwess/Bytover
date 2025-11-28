'use client';

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { AnimatePresence, motion } from "motion/react";
import Image from "next/image";

interface DownloadPlatformsProps {
  className?: string;
}

const PLATFORMS = [
  { id: "android", label: "Android", icon: "/android.svg" },
  { id: "ios", label: "iOS", icon: "/apple.svg" },
  { id: "windows", label: "Windows", icon: "/windows.svg" },
  { id: "macos", label: "Mac OS", icon: "/apple.svg" },
];

export function DownloadPlatforms({ className }: DownloadPlatformsProps) {
  const [expandedPlatform, setExpandedPlatform] = useState<string | null>(null);

  const handlePlatformClick = (platformId: string) => {
    setExpandedPlatform(expandedPlatform === platformId ? null : platformId);
  };

  const scrollToWaitlist = () => {
    const element = document.querySelector("#waitlist");
    if (element) {
      element.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  };

  return (
    <div
      className={
        className ??
        "inline-flex flex-col items-stretch gap-0 bg-white rounded-lg md:rounded-xl border border-gray-200 px-1 md:px-2 py-0.5 md:py-1 shadow-sm overflow-hidden"
      }
    >
      <div className="flex flex-row items-center justify-center shrink-0">
        {PLATFORMS.map((platform, index) => (
          <div key={platform.id} className="flex flex-row items-center shrink-0">
            <Button
              onClick={() => handlePlatformClick(platform.id)}
              className={`flex flex-row items-center gap-1 md:gap-2 bg-transparent hover:bg-black/10 px-2 md:px-4 py-1.5 md:py-2 rounded-lg transition-colors shrink-0 ${expandedPlatform === platform.id ? "bg-black/10" : ""
                }`}
            >
              <Image
                src={platform.icon}
                alt={platform.id}
                width={20}
                height={20}
                className="hidden md:block w-4 h-4 md:w-5 md:h-5 shrink-0"
              />
              <span className="text-xs md:text-sm font-medium whitespace-nowrap">
                {platform.label}
              </span>
            </Button>
            {index < PLATFORMS.length - 1 && (
              <div className="h-5 md:h-6 w-px bg-gray-200 mx-0.5 md:mx-1 shrink-0" />
            )}
          </div>
        ))}
      </div>
      <AnimatePresence>
        {expandedPlatform && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{
              duration: 0.5,
              ease: [0.4, 0, 0.2, 1],
              height: { duration: 0.5, ease: [0.4, 0, 0.2, 1] },
              opacity: { duration: 0.4, ease: "easeInOut" },
            }}
            className="overflow-hidden w-full"
            style={{ width: "100%", maxWidth: "100%" }}
          >
            <div className="pt-2 md:pt-3 px-2 md:px-4 pb-1 md:pb-2 text-center">
              <p className="text-xs md:text-sm text-gray-600 mb-1">
                We&apos;re currently developing native versions
              </p>
              <p className="text-xs md:text-sm text-gray-600 mb-2 md:mb-3">
                and will release soon this year.{" "}
                <button
                  onClick={scrollToWaitlist}
                  className="text-bluePrimary hover:text-blue-600 underline transition-colors"
                >
                  Join the waitlist
                </button>
              </p>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}


