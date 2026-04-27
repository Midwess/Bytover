import type { Metadata } from 'next';
import Link from 'next/link';
import { getCanonicalUrl } from '@/lib/canonical';
import { LegalShell, type TocEntry } from '../_components/legal-shell';
import { PRIVACY_EFFECTIVE_DATE } from '../_components/effective-date';

export const metadata: Metadata = {
    title: 'Privacy Policy — Bytover',
    description:
        'How Bytover handles your data: end-to-end encryption, no third-party data sharing for analysis, and clear data-subject rights.',
    alternates: {
        canonical: getCanonicalUrl('/policy/privacy'),
    },
};

const TOC: TocEntry[] = [
    { id: 'summary', label: 'Summary' },
    { id: 'data-we-collect', label: 'Data we collect' },
    { id: 'how-we-use-data', label: 'How we use data' },
    { id: 'no-third-party-sharing', label: 'No third-party sharing' },
    { id: 'sub-processors', label: 'Operational sub-processors' },
    { id: 'transfer-architecture', label: 'How transfers work' },
    { id: 'retention', label: 'Retention' },
    { id: 'international-transfers', label: 'International transfers' },
    { id: 'your-rights', label: 'Your rights' },
    { id: 'account-deletion', label: 'Account deletion' },
    { id: 'children', label: 'Children' },
    { id: 'cookies-and-storage', label: 'Cookies & storage' },
    { id: 'security', label: 'Security' },
    { id: 'changes', label: 'Changes to this policy' },
    { id: 'contact', label: 'Contact' },
];

export default function PrivacyPolicyPage() {
    return (
        <LegalShell
            title="Privacy Policy"
            effectiveDate={PRIVACY_EFFECTIVE_DATE}
            toc={TOC}
            intro={
                <p>
                    Bytover is a file-transfer product designed so that your files do not pass through
                    our servers in the common case. This policy explains exactly what we do collect,
                    why, and the rights you have over that data.
                </p>
            }
        >
            <section id="summary" className="mb-12 scroll-mt-24">
                <h2>Summary</h2>
                <ul>
                    <li>Most transfers are peer-to-peer. Your files do not touch Bytover servers.</li>
                    <li>
                        We do <strong>not</strong> share, sell, or hand your data to any third party for
                        analysis, advertising, or profiling.
                    </li>
                    <li>
                        We only process the minimum data required to operate the service: account identity,
                        a device identifier bound to your token, transfer metadata for plan limits, and
                        Apple-issued purchase receipts for the Paid Plan.
                    </li>
                    <li>You can delete your account at any time from inside the app or by emailing us.</li>
                </ul>
            </section>

            <section id="data-we-collect" className="mb-12 scroll-mt-24">
                <h2>Data we collect</h2>
                <p>We collect only what is necessary to operate Bytover:</p>
                <ul>
                    <li>
                        <strong>Account data.</strong> The identifier returned by your sign-in provider
                        (such as an email address from Google or Apple Sign In). We do not see your
                        provider password.
                    </li>
                    <li>
                        <strong>Device data.</strong> A device identifier bound to your authentication
                        token so a stolen token cannot be replayed from a different device.
                    </li>
                    <li>
                        <strong>Transfer metadata.</strong> Counters such as lifetime bytes transferred
                        and per-transfer file counts, used solely to enforce Free-plan limits and to show
                        your usage in the app. We do not store the contents of your transfers.
                    </li>
                    <li>
                        <strong>Payment receipt data.</strong> For the Paid Plan, Apple sends us a
                        purchase receipt confirming a transaction occurred. We never receive your card
                        number, billing address, or any other payment-instrument details.
                    </li>
                    <li>
                        <strong>Operational logs.</strong> Short-lived server logs (request timing, error
                        traces) that may include IP addresses for abuse prevention. These logs do not
                        include the content of your transfers.
                    </li>
                </ul>
            </section>

            <section id="how-we-use-data" className="mb-12 scroll-mt-24">
                <h2>How we use data</h2>
                <ul>
                    <li>To authenticate you and keep your session secure.</li>
                    <li>To enforce Free-plan limits and surface your remaining quota.</li>
                    <li>To reconcile Paid Plan purchases with Apple.</li>
                    <li>To detect and respond to abuse of the service or relay infrastructure.</li>
                    <li>To meet legal obligations when required by valid legal process.</li>
                </ul>
                <p>
                    We do not use your data to build advertising profiles, train models, sell access, or
                    any other secondary purpose.
                </p>
            </section>

            <section id="no-third-party-sharing" className="mb-12 scroll-mt-24">
                <h2>No third-party sharing for analysis</h2>
                <p>
                    <strong>
                        Bytover does not share, sell, rent, or otherwise provide your data to any third
                        party for analysis, advertising, profiling, audience-building, or any other
                        commercial purpose.
                    </strong>
                </p>
                <p>
                    The companies listed in <a href="#sub-processors">Operational sub-processors</a> are
                    strictly the infrastructure providers that make the product work — they receive only
                    the minimum identifiers necessary for that operational role and are contractually
                    bound to use that data only on our instructions.
                </p>
            </section>

            <section id="sub-processors" className="mb-12 scroll-mt-24">
                <h2>Operational sub-processors</h2>
                <p>The following providers process limited data on our behalf:</p>
                <ul>
                    <li>
                        <strong>Apple (App Store / StoreKit).</strong> Receives your Paid Plan purchase
                        and returns a receipt to us. Apple does not receive any transfer content or
                        transfer metadata from Bytover.
                    </li>
                    <li>
                        <strong>OAuth identity providers (Google, Apple Sign In).</strong> Authenticate
                        your sign-in and return an identifier to Bytover. They do not receive any transfer
                        content or transfer metadata.
                    </li>
                    <li>
                        <strong>Cloud hosting and TURN-relay infrastructure.</strong> Encrypted packets
                        traverse this infrastructure when used. The provider cannot read packet contents,
                        and we do not retain the packets after they are forwarded.
                    </li>
                </ul>
                <p>
                    None of these providers receive your data for analysis, profiling, or advertising.
                </p>
            </section>

            <section id="transfer-architecture" className="mb-12 scroll-mt-24">
                <h2>How transfers work (and why we see so little)</h2>
                <div className="border border-zinc-800 rounded-lg p-4 mb-4">
                    <h3>Peer-to-peer (P2P)</h3>
                    <p>
                        On Desktop and Web, Bytover uses WebRTC to send files directly from your device to
                        the recipient&apos;s device. The vast majority of transfers happen this way. Your
                        files do not touch our servers.
                    </p>
                </div>
                <div className="border border-zinc-800 rounded-lg p-4 mb-4">
                    <h3>Cloud transfer</h3>
                    <p>
                        When P2P is unavailable, we offer cloud-based transfer. Files are encrypted in
                        transit and automatically deleted after the transfer completes.
                    </p>
                </div>
                <div className="border border-zinc-800 rounded-lg p-4">
                    <h3>TURN relay (last resort)</h3>
                    <p>
                        When direct P2P cannot be established (for example, on networks with symmetric NAT
                        or restrictive firewalls), encrypted packets are forwarded through a TURN relay.
                        The relay forwards in real time and immediately discards each packet — nothing is
                        stored.
                    </p>
                </div>
            </section>

            <section id="retention" className="mb-12 scroll-mt-24">
                <h2>Retention</h2>
                <ul>
                    <li>Account data and transfer metadata: retained while your account is active.</li>
                    <li>
                        Cloud-transfer file payloads: deleted automatically after transfer completion or
                        within a short retention window if pickup never occurs.
                    </li>
                    <li>Relay packets: never persisted.</li>
                    <li>Server logs: rotated within a short operational window.</li>
                    <li>
                        On account deletion: account data and transfer metadata are deleted within 30
                        days. Apple-issued purchase records may be retained where required for tax,
                        accounting, or anti-fraud purposes.
                    </li>
                </ul>
            </section>

            <section id="international-transfers" className="mb-12 scroll-mt-24">
                <h2>International transfers</h2>
                <p>
                    Bytover operates globally. Where your data is processed in a country other than where
                    you reside, we rely on appropriate transfer mechanisms (such as the Standard
                    Contractual Clauses for transfers from the European Economic Area or the United
                    Kingdom) to provide an equivalent level of protection.
                </p>
            </section>

            <section id="your-rights" className="mb-12 scroll-mt-24">
                <h2>Your rights</h2>
                <p>
                    Depending on where you live, you may have the following rights over your personal
                    data:
                </p>
                <ul>
                    <li>Access — request a copy of the data we hold about you.</li>
                    <li>Rectification — correct inaccurate data.</li>
                    <li>Erasure — request deletion of your account and associated data.</li>
                    <li>Portability — receive your data in a machine-readable format.</li>
                    <li>Objection / restriction — object to or restrict certain processing.</li>
                    <li>
                        California (CCPA) — know what we collect, request deletion, and confirm we do not
                        sell your data. We do not sell your data.
                    </li>
                </ul>
                <p>
                    To exercise any of these rights, email{' '}
                    <a href="mailto:team@bytover.com">team@bytover.com</a>. We respond within 30 days.
                </p>
            </section>

            <section id="account-deletion" className="mb-12 scroll-mt-24">
                <h2>Account deletion</h2>
                <p>You can delete your Bytover account at any time:</p>
                <ul>
                    <li>
                        <strong>In-app:</strong> open Settings, choose Account, then Delete Account, and
                        confirm.
                    </li>
                    <li>
                        <strong>By email:</strong> send a deletion request from the address associated
                        with your account to{' '}
                        <a href="mailto:team@bytover.com">team@bytover.com</a>.
                    </li>
                </ul>
                <p>
                    Account data and transfer metadata are removed within 30 days of a confirmed deletion
                    request. Active P2P sessions terminate immediately.
                </p>
            </section>

            <section id="children" className="mb-12 scroll-mt-24">
                <h2>Children</h2>
                <p>
                    Bytover is not directed to children under 13, and our App Store age rating reflects
                    the intended audience. We do not knowingly collect personal data from children. If
                    you believe a child has provided us with personal data, contact{' '}
                    <a href="mailto:team@bytover.com">team@bytover.com</a> and we will delete it.
                </p>
            </section>

            <section id="cookies-and-storage" className="mb-12 scroll-mt-24">
                <h2>Cookies and local storage</h2>
                <p>
                    Bytover does not set advertising or tracking cookies. The web app uses strictly
                    necessary storage on your device:
                </p>
                <ul>
                    <li>An authentication cookie to keep you signed in.</li>
                    <li>OPFS (Origin Private File System) storage to manage in-progress transfers.</li>
                    <li>LocalStorage for UI preferences such as theme.</li>
                </ul>
                <p>None of this storage is shared with third parties.</p>
            </section>

            <section id="security" className="mb-12 scroll-mt-24">
                <h2>Security</h2>
                <ul>
                    <li>End-to-end encryption (DTLS) on every WebRTC transfer.</li>
                    <li>TLS for all client–server traffic.</li>
                    <li>No server-side storage of P2P transfer contents.</li>
                    <li>Tokens are bound to a device fingerprint to limit replay risk.</li>
                </ul>
            </section>

            <section id="changes" className="mb-12 scroll-mt-24">
                <h2>Changes to this policy</h2>
                <p>
                    When we update this policy we change the effective date at the top of the page. For
                    material changes affecting your rights or how we process your data, we will also
                    notify you via email or an in-app banner before the change takes effect.
                </p>
            </section>

            <section id="contact" className="mb-12 scroll-mt-24">
                <h2>Contact</h2>
                <p>
                    Questions, requests, or complaints about this policy can be sent to{' '}
                    <a href="mailto:team@bytover.com">team@bytover.com</a>. You can also visit our{' '}
                    <Link href="/contact">contact page</Link>.
                </p>
            </section>
        </LegalShell>
    );
}
