import type { Metadata } from "next";
import { generateMetadataWithCanonical } from "@/lib/canonical";

export const metadata: Metadata = generateMetadataWithCanonical('/oauth', {
    title: "OAuth Authentication - Bytover",
    description: "Authenticating your account with Bytover.",
});

export default function OAuthLayout({
    children,
}: {
    children: React.ReactNode;
}) {
    return <>{children}</>;
}

