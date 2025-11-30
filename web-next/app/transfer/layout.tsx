import type { Metadata } from "next";
import { generateMetadataWithCanonical } from "@/lib/canonical";

export const metadata: Metadata = generateMetadataWithCanonical('/transfer', {
    title: "Transfer Files - Bytover",
    description: "Transfer files between all your devices. Send and receive files securely with nearby transfer or public cloud sharing.",
});

export default function TransferLayout({
    children,
}: {
    children: React.ReactNode;
}) {
    return <>{children}</>;
}

