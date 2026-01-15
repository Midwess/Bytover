import type { Metadata } from 'next';
import { getCanonicalUrl } from '@/lib/canonical';
import Header from '@/components/web/header';
import Footer from '@/components/web/footer';

export const metadata: Metadata = {
    title: 'Privacy Policy - Bytover',
    description: 'Learn how Bytover protects your privacy with P2P transfers and no data collection.',
    alternates: {
        canonical: getCanonicalUrl('/policy'),
    },
};

export default function PolicyPage() {
    return (
        <div className="min-h-screen bg-black text-primaryText flex flex-col">
            <Header />
            <main className="flex-1 py-16 px-4">
                <div className="container mx-auto max-w-4xl">
                    <article className="prose prose-invert prose-lg max-w-none">
                        <h1 className="text-4xl font-bold mb-8">Privacy Policy</h1>
                        <p className="text-muted-foreground mb-8">
                            Last updated: {new Date().toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' })}
                        </p>

                        <section className="mb-12">
                            <h2 className="text-2xl font-semibold mb-4">Our Commitment to Your Privacy</h2>
                            <p className="text-muted-foreground leading-relaxed">
                                At Bytover, we believe your data belongs to you. We have built our file transfer service
                                with privacy as a core principle, not an afterthought.
                            </p>
                        </section>

                        <section className="mb-12">
                            <h2 className="text-2xl font-semibold mb-4">Transfer Types</h2>
                            <p className="text-muted-foreground leading-relaxed mb-4">
                                Bytover offers two methods of file transfer, each designed with your privacy in mind:
                            </p>

                            <div className="mb-6 p-4 border border-zinc-800 rounded-lg">
                                <h3 className="text-xl font-semibold mb-3 text-primaryText">1. Peer-to-Peer (P2P) Transfer</h3>
                                <p className="text-muted-foreground leading-relaxed mb-3">
                                    Available on Desktop and Web, P2P is our primary transfer method and is used in
                                    <strong className="text-primaryText"> the vast majority of transfers</strong>.
                                    Using WebRTC technology, your files travel directly from your device to the
                                    recipient&apos;s device without passing through our servers.
                                </p>
                                <p className="text-muted-foreground leading-relaxed">
                                    <strong className="text-primaryText">We do not keep any of your data.</strong>
                                    {' '}Your files never touch our servers - they go straight from sender to receiver.
                                </p>
                            </div>

                            <div className="p-4 border border-zinc-800 rounded-lg">
                                <h3 className="text-xl font-semibold mb-3 text-primaryText">2. Cloud Transfer</h3>
                                <p className="text-muted-foreground leading-relaxed">
                                    For situations where P2P is not available, we offer cloud-based transfer.
                                    <strong className="text-primaryText"> We do not use or sell any of your data.</strong>
                                    {' '}Files uploaded for cloud transfer are encrypted and automatically deleted
                                    after the transfer is complete.
                                </p>
                            </div>
                        </section>

                        <section className="mb-12">
                            <h2 className="text-2xl font-semibold mb-4">Relay Server (Edge Cases Only)</h2>
                            <p className="text-muted-foreground leading-relaxed mb-4">
                                In rare edge cases where direct P2P connections are blocked by network configurations
                                (such as symmetric NAT or restrictive firewalls), we use TURN relay servers to
                                facilitate the transfer.
                            </p>
                            <p className="text-muted-foreground leading-relaxed mb-4">
                                <strong className="text-primaryText">Our relay servers truly relay - they do not store or keep any data.</strong>
                                {' '}The relay simply forwards encrypted packets in real-time between peers. Once the
                                packet is forwarded, it is immediately discarded.
                            </p>
                            <p className="text-muted-foreground leading-relaxed">
                                Relay is only used as a last resort when direct P2P cannot be established. The vast
                                majority of transfers happen via true P2P without any relay involvement.
                            </p>
                        </section>

                        <section className="mb-12">
                            <h2 className="text-2xl font-semibold mb-4">Data Collection</h2>
                            <p className="text-muted-foreground leading-relaxed mb-4">
                                <strong className="text-primaryText">We do not collect, store, or sell your personal data.</strong>
                            </p>
                            <p className="text-muted-foreground leading-relaxed">
                                Regardless of which transfer method you use, we do not analyze, index, or access
                                the content of your transfers. Your files are your business, not ours.
                            </p>
                        </section>

                        <section className="mb-12">
                            <h2 className="text-2xl font-semibold mb-4">Third-Party Data Sharing</h2>
                            <p className="text-muted-foreground leading-relaxed">
                                <strong className="text-primaryText">We never share, sell, or provide your data to any third party.</strong>
                                {' '}Your files and transfer metadata are not accessible to advertisers, data brokers,
                                or any other external entities.
                            </p>
                        </section>

                        <section className="mb-12">
                            <h2 className="text-2xl font-semibold mb-4">Security Measures</h2>
                            <ul className="list-disc list-inside text-muted-foreground space-y-2">
                                <li>End-to-end encryption for all transfers</li>
                                <li>No server-side storage of transferred files</li>
                                <li>Secure WebRTC connections with DTLS encryption</li>
                                <li>No tracking cookies or analytics that identify you</li>
                            </ul>
                        </section>

                        <section className="mb-12">
                            <h2 className="text-2xl font-semibold mb-4">Your Rights</h2>
                            <p className="text-muted-foreground leading-relaxed">
                                Since we do not collect or store your personal data, there is no data to request,
                                modify, or delete. Your privacy is protected by design, not by policy alone.
                            </p>
                        </section>

                        <section className="mb-12">
                            <h2 className="text-2xl font-semibold mb-4">Contact Us</h2>
                            <p className="text-muted-foreground leading-relaxed">
                                If you have any questions about this Privacy Policy or our privacy practices,
                                please contact us through our GitHub repository or support channels.
                            </p>
                        </section>

                        <section className="mb-12">
                            <h2 className="text-2xl font-semibold mb-4">Changes to This Policy</h2>
                            <p className="text-muted-foreground leading-relaxed">
                                We may update this Privacy Policy from time to time. Any changes will be posted
                                on this page with an updated revision date.
                            </p>
                        </section>
                    </article>
                </div>
            </main>
            <Footer />
        </div>
    );
}
