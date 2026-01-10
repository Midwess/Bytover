'use client';

import Image from 'next/image';

interface BentoCardData {
    id: string;
    heading: string;
    description: string;
    video?: string;
    image?: string;
    variant?: 'big' | 'small';
    height?: string;
}

interface BentoProps {
    cards: BentoCardData[];
}

function BentoCard({ card }: { card: BentoCardData }) {
    const isBig = card.variant === 'big';

    return (
        <div
            className={`flex flex-col overflow-hidden bg-black ${isBig ? 'h-full' : ''}`}
            style={card.height ? { height: card.height } : undefined}
        >
            <div className="relative flex-1 min-h-0">
                {card.video ? (
                    <video
                        src={card.video}
                        autoPlay
                        loop
                        muted
                        playsInline
                        className="w-full h-full object-contain"
                    />
                ) : card.image ? (
                    <Image
                        src={card.image}
                        alt={card.heading}
                        fill
                        className="object-cover"
                    />
                ) : null}
            </div>
            <div className="bg-muted p-4 md:p-6">
                <h3 className="text-lg md:text-xl font-bold text-white mb-2">
                    {card.heading}
                </h3>
                <p className="text-sm md:text-base text-white/70 line-clamp-3">
                    {card.description}
                </p>
            </div>
        </div>
    );
}

export function Bento({ cards }: BentoProps) {
    const bigCard = cards.find(c => c.variant === 'big');
    const smallCards = cards.filter(c => c.variant !== 'big');

    return (
        <div className="w-full max-w-5xl mx-auto px-4">
            <div className="flex flex-col md:flex-row gap-6">
                {bigCard && (
                    <div className="flex-1 flex">
                        <div className="flex-1">
                            <BentoCard card={bigCard} />
                        </div>
                    </div>
                )}
                <div className="hidden md:block w-px bg-transparent relative">
                    <div className="absolute inset-0 border-l border-dashed border-white/20" />
                </div>
                <div className="flex-1 flex flex-col gap-6">
                    {smallCards.map((card, index) => (
                        <div key={card.id} className="relative">
                            {index > 0 && (
                                <div className="absolute -top-3 left-0 right-0 h-px">
                                    <div className="w-full h-full border-t border-dashed border-white/20" />
                                </div>
                            )}
                            <BentoCard card={card} />
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}
