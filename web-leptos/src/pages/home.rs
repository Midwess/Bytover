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
    view! {
        <div class="absolute top-0 z-[-1] h-screen w-screen bg-blackBase bg-[radial-gradient(ellipse_80%_80%_at_50%_-20%,rgba(124,255,121,0.2),rgba(255,255,255,0))]"></div>
        <div class="relative h-screen w-screen flex items-center justify-center">
            <div class="container flex flex-col items-center justify-center h-full w-screen">
                <Header/>
            </div>
        </div>
    }
}