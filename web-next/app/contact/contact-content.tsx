'use client'

import { FeedbackForm, useIsFeedbackSubmitted } from '@/components/feedback-form';

export function ContactContent() {
    const isSubmitted = useIsFeedbackSubmitted();

    return (
        <div className="max-w-xl w-full mx-auto flex flex-col items-center">
            <div className="flex flex-col items-center text-center space-y-16 w-full">
                {!isSubmitted && (
                    <div className="space-y-4">
                        <span className="text-xs font-bold tracking-[0.3em] uppercase text-blue-600">
                            Contact Us
                        </span>
                        <h1 className="text-4xl md:text-5xl font-bold text-white tracking-tight leading-[1.1]">
                            Get in Touch.
                        </h1>
                        <p className="text-sm md:text-base text-zinc-400 font-medium">
                            Have a question, feedback, or partnership idea? We read every message.
                        </p>
                    </div>
                )}

                <FeedbackForm />
            </div>
        </div>
    );
}
