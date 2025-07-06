use actix_web::{get, Responder};
use core_services::logger;

#[cfg(feature = "ssr")]

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_files::Files;
    use actix_web::*;
    use leptos::prelude::*;
    use leptos::config::get_configuration;
    use leptos_meta::MetaTags;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use web_leptos::app::*;

    logger::setup();

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;

    HttpServer::new(move || {
        // Generate the list of routes in your Leptos App
        let routes = generate_route_list(App);
        let leptos_options = &conf.leptos_options;
        let site_root = leptos_options.site_root.clone().to_string();

        println!("listening on http://{}", &addr);

        App::new()
            // serve JS/WASM/CSS from `pkg`
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            // serve other assets from the `assets` directory
            .service(Files::new("/assets", &site_root))
            // serve the favicon from /favicon.ico
            .service(favicon)
            .service(style)
            .service(images)
            .service(fonts)
            .leptos_routes(routes, {
                let leptos_options = leptos_options.clone();
                move || {
                    view! {
                        <!DOCTYPE html>
                        <html lang="en">
                            <head>
                                <link rel="stylesheet" href="style.css" />
                                <meta charset="utf-8"/>
                                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                                <AutoReload options=leptos_options.clone() />
                                <HydrationScripts options=leptos_options.clone()/>
                                <MetaTags/>
                            </head>
                            <body>
                                <App/>
                            </body>
                        </html>
                    }
                }
            })
            .app_data(web::Data::new(leptos_options.to_owned()))
        //.wrap(middleware::Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}


#[cfg(feature = "ssr")]
#[get("style.css")]
async fn style(
    leptos_options: actix_web::web::Data<leptos::config::LeptosOptions>,
) -> impl Responder {
    log::info!("Serving styling.css");
    let leptos_options = leptos_options.into_inner();
    let site_root = &leptos_options.site_root;
    actix_files::NamedFile::open_async(format!("{site_root}/style.css")).await
}

#[cfg(feature = "ssr")]
#[get("favicon.ico")]
async fn favicon(
    leptos_options: actix_web::web::Data<leptos::config::LeptosOptions>,
) -> actix_web::Result<actix_files::NamedFile> {
    log::info!("Serving favicon.ico");
    let leptos_options = leptos_options.into_inner();
    let site_root = &leptos_options.site_root;
    Ok(actix_files::NamedFile::open(format!(
        "{site_root}/favicon.ico"
    ))?)
}

#[get("images/{file}")]
async fn images(
    leptos_options: actix_web::web::Data<leptos::config::LeptosOptions>,
    file: actix_web::web::Path<String>,
) -> actix_web::Result<actix_files::NamedFile> {
    log::info!("Serving images: {}", file);

    let leptos_options = leptos_options.into_inner();
    let site_root = &leptos_options.site_root;

    let filepath = format!("{site_root}/images/{file}");

    Ok(actix_files::NamedFile::open(filepath)?)
}

#[get("fonts/{file}")]
async fn fonts(
    leptos_options: actix_web::web::Data<leptos::config::LeptosOptions>,
    file: actix_web::web::Path<String>,
) -> actix_web::Result<actix_files::NamedFile> {
    log::info!("Serving fonts: {}", file);
    let leptos_options = leptos_options.into_inner();
    let site_root = &leptos_options.site_root;
    Ok(actix_files::NamedFile::open(format!("{site_root}/fonts/{file}"))?)
}

#[cfg(not(any(feature = "ssr", feature = "csr")))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
    // see optional feature `csr` instead
}

#[cfg(all(not(feature = "ssr"), feature = "csr"))]
pub fn main() {
    // a client-side main function is required for using `trunk serve`
    // prefer using `cargo leptos serve` instead
    // to run: `trunk serve --open --features csr`
    use web_leptos::app::*;

    logger::setup();

    console_error_panic_hook::set_once();

    leptos::mount_to_body(App);
}
