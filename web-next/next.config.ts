import type { NextConfig } from "next";

const VERSION = process.env.VERSION || process.env.RAILWAY_GIT_COMMIT_SHA
const nextConfig: NextConfig = {
    transpilePackages: ["shared_types"],
    assetPrefix: process.env.S3_CDN_PREFIX && VERSION ? `${process.env.S3_CDN_PREFIX}/commit-${VERSION}` : undefined,
    env: {
        NEXT_PUBLIC_S3_CDN_PREFIX: process.env.S3_CDN_PREFIX || '',
        NEXT_PUBLIC_VERSION: process.env.VERSION || '',
        NEXT_PUBLIC_RAILWAY_GIT_COMMIT_SHA: process.env.RAILWAY_GIT_COMMIT_SHA || '',
    },
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
            {
                protocol: "https",
                hostname: "s3.ap-southeast-1.wasabisys.com",
                pathname: "/**"
            },
            {
                protocol: "https",
                hostname: "s3.wasabisys.com",
                pathname: "/**"
            },
            {
                protocol: "https",
                hostname: "s3.us-east-2.wasabisys.com",
                pathname: "/**"
            },
            {
                protocol: "https",
                hostname: "s3.us-west-1.wasabisys.com",
                pathname: "/**"
            },
            {
                protocol: "https",
                hostname: "s3.ap-southeast-2.wasabisys.com",
                pathname: "/**"
            },
            {
                protocol: "https",
                hostname: "s3.eu-central-2.wasabisys.com",
                pathname: "/**"
            },
        ],
    },
    output: "standalone",
    poweredByHeader: false,
    compress: true,
};

export default nextConfig;
