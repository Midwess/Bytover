use leptos::*;
use leptos::prelude::*;

#[component]
pub fn PrimaryButton(
    #[prop()]
    title: &'static str
) -> impl IntoView {
    view! {
        <button>
            <div class="p-2">
                <p class="font-sfbold">{title}</p>
            </div>
        </button>
    }
}