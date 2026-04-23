'use client'

import { getAssetUrl } from '@/utils/asset-url';
import { FeedbackForm, useIsFeedbackSubmitted } from '@/components/feedback-form';

export function JoinWaitList() {
    const isSubmitted = useIsFeedbackSubmitted();

    return (
        <section id="waitlist" className="w-full py-12 md:py-24 bg-black overflow-hidden px-4 md:px-6 flex justify-center">
            <div className="w-full max-w-4xl relative rounded-xl md:rounded-[2.5rem] overflow-hidden border border-white/10 bg-[#080410]">
                <div className="absolute inset-0 z-0">
                    <img
                        src={getAssetUrl('/background6.jpg')}
                        alt=""
                        className="w-full h-full object-cover opacity-25"
                    />
                    <div className="absolute inset-0 bg-gradient-to-b from-[#080410]/50 to-[#080410]" />
                    <div className="absolute inset-0 opacity-[0.2] mix-blend-overlay pointer-events-none" style={{ backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/svg%3E#noiseFilter")` }} />
                    <div className="absolute inset-0 pointer-events-none overflow-hidden mix-blend-overlay hidden dark:block bg-purple-500/5 backdrop-blur-[2px]" />
                </div>

                <div className="relative z-10 px-8 md:px-24 py-20 md:py-32">
                    <div className="max-w-md mx-auto flex flex-col items-center">
                        <div className="flex flex-col items-center text-center space-y-16 w-full">
                            {!isSubmitted && (
                                <div className="space-y-4">
                                    <span className="text-xs font-bold tracking-[0.3em] uppercase text-blue-600">
                                        Give Feedback
                                    </span>
                                    <h2 className="text-4xl md:text-5xl font-bold text-white tracking-tight leading-[1.1]">
                                        Shape Bytover.
                                    </h2>
                                    <p className="text-sm md:text-base text-zinc-400 font-medium">
                                        Have a feature request or feedback? We&apos;re listening.
                                    </p>
                                </div>
                            )}

                            <FeedbackForm />
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
}
