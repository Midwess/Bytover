import type { NextConfig } from "next";

const VERSION = process.env.VERSION || process.env.GIT_COMMIT_SHA
const nextConfig: NextConfig = {
    transpilePackages: ["shared_types"],
    assetPrefix: process.env.S3_CDN_PREFIX && VERSION ? `${process.env.S3_CDN_PREFIX}/commit-${VERSION}` : undefined,
    env: {
        NEXT_PUBLIC_S3_CDN_PREFIX: process.env.S3_CDN_PREFIX || '',
        NEXT_PUBLIC_VERSION: process.env.VERSION || '',
        NEXT_PUBLIC_GIT_COMMIT_SHA: process.env.GIT_COMMIT_SHA || '',
    },
    allowedDevOrigins: [
        'https://premises-bedrooms-democrat-philadelphia.trycloudflare.com',
    ],
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
            {
                protocol: "https",
                hostname: "midwess.sgp1.digitaloceanspaces.com",
                pathname: "/**"
            },
            {
                protocol: "https",
                hostname: "midwess.sgp1.cdn.digitaloceanspaces.com",
                pathname: "/**"
            }
        ],
    },
    output: "standalone",
    poweredByHeader: false,
    compress: true,
    async redirects() {
        return [
            {
                source: '/policy.html',
                destination: '/policy',
                permanent: true,
            },
            {
                source: '/policy/privacy',
                destination: '/policy#privacy',
                permanent: true,
            },
            {
                source: '/policy/terms',
                destination: '/policy#terms',
                permanent: true,
            },
            {
                source: '/policy/eula',
                destination: '/policy#eula',
                permanent: true,
            },
        ];
    },
};

export default nextConfig;
