export default function Home() {
    const availablePlatforms = [
        {
            name: "Android",
            logo: "images/android.svg",
        },
        {
            name: "iOS",
            logo: "images/apple.svg",
        },
        {
            name: "Windows",
            logo: "images/windows.svg",
        },
        {
            name: "Mac OS",
            logo: "images/apple.svg",
        }
    ];

    return (
        <>
            <div
                className="absolute top-0 z-[-1] h-screen w-screen bg-blackBase bg-[radial-gradient(ellipse_80%_80%_at_50%_-20%,rgba(124,255,121,0.2),rgba(255,255,255,0))]"></div>
            <div className="relative h-screen w-screen flex items-center justify-center">
                <div className="container flex flex-col items-center h-full w-screen gap-16">
                    <style>{`
                        .word {
                            display: inline-block;
                            opacity: 0;
                            transform: translateY(1em);
                            animation: fadeUp 0.6s cubic-bezier(0.19, 1, 0.22, 1) forwards;
                        }

                        .word:nth-child(1) { animation-delay: 0s; }
                        .word:nth-child(2) { animation-delay: 0.1s; }
                        .word:nth-child(3) { animation-delay: 0.2s; }
                        .word:nth-child(4) { animation-delay: 0.3s; }
                        .word:nth-child(5) { animation-delay: 0.4s; }
                        .word:nth-child(6) { animation-delay: 0.5s; }

                        @keyframes fadeUp {
                            from {
                                opacity: 0;
                                transform: translateY(1em);
                            }
                            to {
                                opacity: 1;
                                transform: translateY(0);
                            }
                        }
                    `}</style>
                    <p className="text-5xl font-sfbold text-primaryText text-center leading-14 max-width-50vw">
                        <span className="word p-1">Seamless</span>
                        <span className="word p-1">File</span>
                        <span className="word p-1">Transfers</span>
                        <span className="word p-1">You</span>
                        <span className="word p-1">Can</span>
                        <span className="word p-1">Trust</span>
                    </p>
                    <div className="flex flex-col w-full justify-center gap-2">
                        <p className="text-xl font-sf text-primaryText/80 text-center">Available on all platforms</p>
                        <div className="flex flex-row w-full justify-center gap-3">
                            {availablePlatforms.map((item, index) => (
                                <div key={index}
                                     className="flex flex-col items-center rounded-xl bg-primaryText/5 hover:bg-primaryBlue/10 border-solid w-[70px] h-[75px]">
                                    <img src={item.logo} className="px-2 h-[45px] w-[45px] opacity-90" alt={item.name}/>
                                    <p className="text-md font-sf text-primaryText/80 text-center">{item.name}</p>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </>
    );
}