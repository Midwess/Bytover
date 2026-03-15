import type { Metadata } from "next";
import { generateMetadataWithCanonical } from "@/lib/canonical";

export const metadata: Metadata = generateMetadataWithCanonical('/transfer', {
    title: "Transfer Files - Bytover",
    description: "Transfer files to anyone instantly. Send files securely with direct peer-to-peer transfer.",
});

export default function TransferLayout({
    children,
}: {
    children: React.ReactNode;
}) {
    return <>{children}</>;
}

