use leptos::{component, view, IntoView};
use leptos::prelude::*;
use crate::components::header::Header;

/// Renders the home page of your application.
#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <HomeBackground />
    }
}

#[component]
pub fn HomeBackground() -> impl IntoView {
    struct PlatformItem {
        name: &'static str,
        logo: &'static str,
    }

    let available_platforms = vec![
        PlatformItem {
            name: "Android",
            logo: "images/android.svg",
        },
        PlatformItem {
            name: "iOS",
            logo: "images/apple.svg",
        },
        PlatformItem {
            name: "Windows",
            logo: "images/windows.svg",
        },
        PlatformItem {
            name: "Mac OS",
            logo: "images/apple.svg",
        }
    ];

    view! {
        <div class="absolute top-0 z-[-1] h-screen w-screen bg-blackBase bg-[radial-gradient(ellipse_80%_80%_at_50%_-20%,rgba(124,255,121,0.2),rgba(255,255,255,0))]"></div>
        <div class="relative h-screen w-screen flex items-center justify-center">
            <div class="container flex flex-col items-center h-full w-screen gap-16">
                <Header/>
                <style>{r#"
                    .word {
                        display: inline-block;
                        opacity: 0;
                        transform: translateY(1em);
                        animation: fadeUp 0.6s cubic-bezier(0.19, 1, 0.22, 1) forwards;
                    }

                    /* stagger delays for each word */
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
                "#}</style>
                <p class="text-5xl font-sfbold text-primaryText text-center leading-14 max-width-50vw">
                    <span class="word p-1">Seamless</span>
                    <span class="word p-1">File</span>
                    <span class="word p-1">Transfers</span>
                    <span class="word p-1">You</span>
                    <span class="word p-1">Can</span>
                    <span class="word p-1">Trust</span>
                </p>
                <div class="flex flex-col w-full justify-center gap-2">
                    <p class="text-xl font-sf text-primaryText/80 text-center">Available on all platforms</p>
                    <div class="flex flex-row w-full justify-center gap-3">
                        {
                            available_platforms.iter().map(|item| {
                                view! {
                                    <div class="flex flex-col items-center rounded-xl bg-primaryText/5 hover:bg-primaryBlue/10 border-solid w-[70px] h-[75px]">
                                        <img src={item.logo} class="px-2 h-[45px] w-[45px] opacity-90" />
                                        <p class="text-md font-sf text-primaryText/80 text-center">{item.name}</p>
                                    </div>
                                }
                            }).collect_view()
                        }
                    </div>
                </div>
            </div>
        </div>
    }
}