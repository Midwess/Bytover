import type { Metadata } from "next";
import { generateMetadataWithCanonical } from "@/lib/canonical";

export const metadata: Metadata = generateMetadataWithCanonical('/authentication', {
    title: "Authentication - Bytover",
    description: "Sign in to your Bytover account to access file transfer features.",
});

export default function AuthenticationLayout({
    children,
}: {
    children: React.ReactNode;
}) {
    return <>{children}</>;
}

