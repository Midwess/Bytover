import path from "path";
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
    transpilePackages: ["shared_types"],
    images: {
        remotePatterns: [
            {
                protocol: "https",
                hostname: "cbe9ef0f806f8e7c2ed195f658a0c88b.r2.cloudflarestorage.com",
                pathname: "/**",
            },
        ],
    },
    async headers() {
        return [
            {
                source: "/(.*)",
                headers: [
                    { key: "Cross-Origin-Embedder-Policy", value: "require-corp" },
                    { key: "Cross-Origin-Opener-Policy", value: "same-origin" },
                ],
            },
        ];
    },
    webpack: (config, { isServer }) => {
        if (!isServer) {
            config.resolve.fallback = {
                ...config.resolve.fallback,
                fs: false,
                net: false,
                tls: false,
            };
        }

        config.resolve.alias = {
            ...config.resolve.alias,
            "/worker.js$": path.resolve(__dirname, "node_modules/worker/worker.js"),
        };

        return config;
    },
};

export default nextConfig;
