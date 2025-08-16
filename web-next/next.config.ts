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
};

export default nextConfig;
