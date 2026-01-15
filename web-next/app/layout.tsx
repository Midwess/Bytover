import { Inter } from 'next/font/google';
import type { Metadata, Viewport } from "next";
import "./globals.css";
import CoreStart from "@/app/core_start";
import AppToaster from "@/components/ui/toaster";
import { getCanonicalUrl } from "@/lib/canonical";
import { SEOSchemas } from "@/components/seo-schemas";

const inter = Inter({
  subsets: ['latin'],
  variable: '--font-inter',
});

export const metadata: Metadata = {
  title: "Bytover – Instant, Public & Email File Transfer",
  description: "Transfer files effortlessly. Share instantly via link with no upload needed, create public links with password protection, or send via email.",
  icons: {
    icon: [
      { url: "/favicon-96x96.png", sizes: "96x96", type: "image/png", media: "(prefers-color-scheme: dark)" },
      { url: "/favicon-light.png", sizes: "96x96", type: "image/png", media: "(prefers-color-scheme: light)" },
    ],
    shortcut: "/favicon.ico",
    apple: "/apple-touch-icon.png",
  },
  manifest: "/site.webmanifest",
  appleWebApp: {
    title: "Bytover",
  },
  alternates: {
    canonical: getCanonicalUrl('/'),
  },
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  maximumScale: 1,
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={"w-full h-full"}>
      <head>
        <SEOSchemas />
      </head>
      <body
        className={`w-full h-full min-h-screen ${inter.variable} antialiased dark`}>
        {children}
        <CoreStart />
        <AppToaster />
      </body>
    </html>
  );
}
