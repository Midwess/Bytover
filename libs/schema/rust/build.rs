use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running schema build.rs...");
    println!("cargo:rerun-if-changed=../proto");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(".generated");
    if !out_dir.exists() {
        fs::create_dir_all(&out_dir)?;
        println!("Created .generated folder");
    }

    #[allow(unused_mut)]
    let mut config = tonic_build::configure();

    #[cfg(feature = "client")]
    {
        config = config.build_client(true).client_mod_attribute(".", "#[allow(non_camel_case_types)]");
    }
    #[cfg(feature = "server")]
    {
        config = config
            .build_server(true)
            .build_client(true)
            .server_mod_attribute(".", "#[allow(non_camel_case_types)]");
    }

    #[cfg(feature = "bindgen")]
    {
        config = config
            .enum_attribute(".value", "#[derive(uniffi::Enum)]")
            .message_attribute(".value", "#[derive(uniffi::Record)]")
            .message_attribute(".devlog.bitbridge", "#[derive(uniffi::Record)]")
            .enum_attribute(".devlog.bitbridge", "#[derive(uniffi::Enum)]")
            .type_attribute(
                ".value.Platform",
                r#"#[derive(uniffi::Enum)]
               #[repr(u32)]"#
            );
    }

    config
        .client_mod_attribute(".", "#[cfg(feature = \"client\")]")
        .server_mod_attribute(".", "#[cfg(feature = \"server\")]")
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &[
                "../proto/value/datetime.proto",
                "../proto/value/static_resource.proto",
                "../proto/devlog/app-gateway/rpc/auth.proto",
                "../proto/devlog/app-gateway/rpc/user.proto",
                "../proto/devlog/app-gateway/rpc/storage.proto",
                "../proto/devlog/app-gateway/rpc/application.proto",
                "../proto/devlog/app-gateway/rpc/mail.proto",
                "../proto/devlog/app-gateway/models/application.proto",
                "../proto/devlog/app-gateway/models/user.proto",
                "../proto/devlog/rpc-signalling/server.proto",
                "../proto/crafter/email.proto",
                "../proto/value/static_resource.proto",
                "../proto/value/datetime.proto",
                "../proto/value/auth_method.proto",
                "../proto/value/device.proto",
                "../proto/value/platform.proto",
                "../proto/devlog/bitbridge/peer.proto",
                "../proto/devlog/bitbridge/request.proto",
                "../proto/devlog/bitbridge/resource.proto",
                "../proto/devlog/bitbridge/session.proto",
                "../proto/devlog/bitbridge/relay.proto",
                "../proto/devlog/devblog/code-playground/entities/file.proto",
                "../proto/devlog/devblog/code-playground/rpc/code-playground.proto",
                "../proto/devlog/devblog/entities/author.proto",
                "../proto/devlog/devblog/entities/post.proto",
                "../proto/devlog/devblog/entities/interaction.proto",
                "../proto/devlog/devblog/rpc/post.proto",
                "../proto/devlog/bitbridge/cloud_service.proto",
                "../proto/devlog/bitbridge/p2p.proto",
                "../proto/devlog/app-gateway/models/device.proto",
                "../proto/devlog/app-gateway/rpc/markov.proto",
                "../proto/devlog/app-gateway/rpc/people.proto",
                "../proto/devlog/app-gateway/rpc/feedback.proto",
                "../proto/midwess_ai/builder/rpc/builder.proto",
                "../proto/midwess_ai/api/models/user.proto",
                "../proto/midwess_ai/api/models/workspace.proto",
                "../proto/midwess_ai/api/models/workspace.proto",
                "../proto/midwess_ai/api/public/user.proto",
                "../proto/midwess_ai/api/public/workspace.proto",
                "../proto/midwess_ai/api/models/project.proto",
                "../proto/midwess_ai/api/models/version_control.proto",
                "../proto/midwess_ai/api/models/models/sandbox.proto",
                "../proto/midwess_ai/api/models/models/template.proto",
                "../proto/midwess_ai/api/public/project.proto",
                "../proto/midwess_ai/api/public/version_control.proto",
                "../proto/midwess_ai/api/public/template.proto",
                "../proto/midwess_ai/api/internal/token.proto",
                "../proto/midwess_ai/api/public/chat.proto",
                "../proto/midwess_ai/api/public/sandbox.proto",
                "../proto/midwess_ai/api/public/llm_provider.proto",
                "../proto/midwess_ai/sandbox_manager/rpc/internal/manager.proto",
                "../proto/midwess_ai/sandbox_manager/rpc/public/manager.proto"
            ],
            &["../proto"]
        )?;

    println!("Proto compilation successful!");
    Ok(())
}
