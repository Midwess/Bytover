import type { Metadata } from "next";
import { generateMetadataWithCanonical } from "@/lib/canonical";

export const metadata: Metadata = generateMetadataWithCanonical('/transfer', {
    title: "Transfer Files - Bytover",
    description: "Transfer files to anyone instantly. Send and receive files securely with direct transfer or public cloud sharing.",
});

export default function TransferLayout({
    children,
}: {
    children: React.ReactNode;
}) {
    return <>{children}</>;
}

