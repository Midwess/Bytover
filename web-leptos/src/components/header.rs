use leptos::component;
use leptos::prelude::*;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <div class="w-screen container flex justify-between text-primaryText text-opacity-30 my-5">
            <div class="flex flex-row items-center gap-2">
                <img src="/images/earth.png" class="h-[60px] w-[60px]"/>
                <p class="text-3xl font-sfbold">Bit bridge</p>
            </div>
            <div class="flex flex-row rounded-xl px-6 py-4 gap-x-[60px]">
                <Navigation title={"Pricing"}/>
                <Navigation title={"Account"}/>
            </div>
        </div>
    }
}

#[component]
pub fn Navigation(
    #[prop()]
    title: &'static str,
) -> impl IntoView {
    view! {
       <p class="text-md text-primaryText opacity-80 font-sfbold">{title}</p>
    }
}
