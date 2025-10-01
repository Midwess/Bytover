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
            {
                protocol: "https",
                hostname: "pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev",
                pathname: "/**",
            },
        ],
    },
    output: "standalone",
    poweredByHeader: false,
    compress: true,
};

export default nextConfig;
