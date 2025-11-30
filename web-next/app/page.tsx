import Home from "@/app/home";
import type { Metadata } from "next";
import { generateMetadataWithCanonical } from "@/lib/canonical";

export const metadata: Metadata = generateMetadataWithCanonical('/', {
    title: "Bytover - Free File Transfer Between All Your Devices",
    description: "Free nearby and public files transfer on all platforms. Transfer files securely between your devices with Bytover.",
});

export default function Page() {
    return (
        <>
            <Home></Home>
        </>
    );
}
