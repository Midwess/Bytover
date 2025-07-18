import type { NextConfig } from "next";

const nextConfig: NextConfig = {
    transpilePackages: ["shared_types"],
    experimental: {
        clientInstrumentationHook: true
    }
};

export default nextConfig;
