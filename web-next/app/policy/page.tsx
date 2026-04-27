import type { Metadata } from 'next';
import Link from 'next/link';
import { getCanonicalUrl } from '@/lib/canonical';
import Header from '@/components/web/header';
import Footer from '@/components/web/footer';
import {
    PRIVACY_EFFECTIVE_DATE,
    TERMS_EFFECTIVE_DATE,
    EULA_EFFECTIVE_DATE,
    formatEffectiveDate,
} from './_components/effective-date';

export const metadata: Metadata = {
    title: 'Legal — Bytover',
    description:
        'Bytover Privacy Policy, Terms of Service, and End User License Agreement. We do not share your data with third parties for analysis.',
    alternates: {
        canonical: getCanonicalUrl('/policy'),
    },
};

type TocGroup = {
    id: string;
    label: string;
    items: { id: string; label: string }[];
};

const TOC: TocGroup[] = [
    {
        id: 'privacy',
        label: 'Privacy Policy',
        items: [
            { id: 'privacy-summary', label: 'Summary' },
            { id: 'privacy-data-we-collect', label: 'Data we collect' },
            { id: 'privacy-how-we-use-data', label: 'How we use data' },
            { id: 'privacy-no-third-party-sharing', label: 'No third-party sharing' },
            { id: 'privacy-sub-processors', label: 'Operational sub-processors' },
            { id: 'privacy-transfer-architecture', label: 'How transfers work' },
            { id: 'privacy-retention', label: 'Retention' },
            { id: 'privacy-international-transfers', label: 'International transfers' },
            { id: 'privacy-your-rights', label: 'Your rights' },
            { id: 'privacy-account-deletion', label: 'Account deletion' },
            { id: 'privacy-children', label: 'Children' },
            { id: 'privacy-cookies-and-storage', label: 'Cookies & storage' },
            { id: 'privacy-security', label: 'Security' },
            { id: 'privacy-changes', label: 'Changes' },
        ],
    },
    {
        id: 'terms',
        label: 'Terms of Service',
        items: [
            { id: 'terms-agreement', label: 'The agreement' },
            { id: 'terms-eligibility', label: 'Eligibility' },
            { id: 'terms-accounts', label: 'Your account' },
            { id: 'terms-license', label: 'License to use Bytover' },
            { id: 'terms-acceptable-use', label: 'Acceptable use' },
            { id: 'terms-user-content', label: 'Your files & content' },
            { id: 'terms-dmca', label: 'DMCA & copyright' },
            { id: 'terms-suspension', label: 'Suspension & termination' },
            { id: 'terms-paid-plan', label: 'Paid Plan' },
            { id: 'terms-third-party-services', label: 'Third-party services' },
            { id: 'terms-warranty', label: 'No warranty' },
            { id: 'terms-liability', label: 'Limitation of liability' },
            { id: 'terms-indemnification', label: 'Indemnification' },
            { id: 'terms-governing-law', label: 'Governing law' },
            { id: 'terms-changes', label: 'Changes' },
        ],
    },
    {
        id: 'eula',
        label: 'End User License Agreement',
        items: [
            { id: 'eula-scope', label: 'Scope' },
            { id: 'eula-license-grant', label: 'License grant' },
            { id: 'eula-restrictions', label: 'Restrictions' },
            { id: 'eula-paid-plan', label: 'Paid Plan (one-time)' },
            { id: 'eula-no-subscriptions', label: 'No auto-renewal' },
            { id: 'eula-apple', label: 'Apple App Store distribution' },
            { id: 'eula-updates', label: 'Updates' },
            { id: 'eula-termination', label: 'Termination' },
            { id: 'eula-export', label: 'Export control' },
            { id: 'eula-governing-law', label: 'Governing law' },
        ],
    },
    {
        id: 'contact',
        label: 'Contact',
        items: [],
    },
];

export default function PolicyPage() {
    return (
        <div className="min-h-screen bg-black text-primaryText flex flex-col">
            <Header />
            <main className="flex-1 py-16 px-4">
                <div className="container mx-auto max-w-5xl">
                    <header className="mb-10">
                        <h1 className="text-4xl font-bold mb-4">Legal</h1>
                        <p className="text-muted-foreground leading-relaxed max-w-3xl mb-3">
                            Bytover is a file-transfer product designed so your files travel directly
                            between devices whenever possible. We do not share your data with any third
                            party for analysis, advertising, or profiling. The sections below describe
                            the details — Privacy Policy, Terms of Service, and the End User License
                            Agreement.
                        </p>
                        <p className="text-sm text-muted-foreground">
                            Questions: <a href="mailto:team@bytover.com" className="underline hover:text-white">team@bytover.com</a>
                        </p>
                    </header>

                    <div className="grid grid-cols-1 md:grid-cols-[240px_1fr] gap-10">
                        <aside className="hidden md:block">
                            <nav
                                aria-label="Table of contents"
                                className="sticky top-24 border-l border-zinc-800 pl-4"
                            >
                                <p className="text-xs uppercase tracking-[0.2em] text-zinc-500 mb-3">
                                    Contents
                                </p>
                                <ul className="flex flex-col gap-4">
                                    {TOC.map((group) => (
                                        <li key={group.id}>
                                            <a
                                                href={`#${group.id}`}
                                                className="text-sm font-semibold text-zinc-300 hover:text-white transition-colors"
                                            >
                                                {group.label}
                                            </a>
                                            {group.items.length > 0 ? (
                                                <ul className="mt-2 ml-3 flex flex-col gap-1.5 border-l border-zinc-800 pl-3">
                                                    {group.items.map((item) => (
                                                        <li key={item.id}>
                                                            <a
                                                                href={`#${item.id}`}
                                                                className="text-xs text-zinc-500 hover:text-white transition-colors"
                                                            >
                                                                {item.label}
                                                            </a>
                                                        </li>
                                                    ))}
                                                </ul>
                                            ) : null}
                                        </li>
                                    ))}
                                </ul>
                            </nav>
                        </aside>

                        <div>
                            <details className="md:hidden mb-8 border border-zinc-800 rounded-lg">
                                <summary className="cursor-pointer px-4 py-3 text-sm uppercase tracking-[0.2em] text-zinc-400">
                                    Contents
                                </summary>
                                <ul className="flex flex-col gap-2 px-4 pb-4">
                                    {TOC.map((group) => (
                                        <li key={group.id}>
                                            <a
                                                href={`#${group.id}`}
                                                className="text-sm text-zinc-300 hover:text-white transition-colors"
                                            >
                                                {group.label}
                                            </a>
                                        </li>
                                    ))}
                                </ul>
                            </details>

                            <article className="prose prose-invert prose-lg max-w-none prose-headings:text-primaryText prose-strong:text-primaryText">
                                <PrivacySection />
                                <Divider />
                                <TermsSection />
                                <Divider />
                                <EulaSection />
                                <Divider />
                                <ContactSection />
                            </article>
                        </div>
                    </div>
                </div>
            </main>
            <Footer />
        </div>
    );
}

function Divider() {
    return <hr className="my-16 border-zinc-800" />;
}

function EffectiveLine({ iso }: { iso: string }) {
    return (
        <p className="text-xs uppercase tracking-[0.2em] text-zinc-500 not-prose mb-6">
            Effective: {formatEffectiveDate(iso)}
        </p>
    );
}

function PrivacySection() {
    return (
        <section id="privacy" className="scroll-mt-24">
            <h2>Privacy Policy</h2>
            <EffectiveLine iso={PRIVACY_EFFECTIVE_DATE} />

            <section id="privacy-summary" className="scroll-mt-24">
                <h3>Summary</h3>
                <ul>
                    <li>Most transfers are peer-to-peer. Your files do not touch Bytover servers.</li>
                    <li>
                        We do <strong>not</strong> share, sell, or hand your data to any third party for
                        analysis, advertising, or profiling.
                    </li>
                    <li>
                        We only process the minimum data required to operate the service: account
                        identity, a device identifier bound to your token, transfer metadata for plan
                        limits, and Apple-issued purchase receipts for the Paid Plan.
                    </li>
                    <li>
                        You can delete your account at any time from inside the app or by emailing us.
                    </li>
                </ul>
            </section>

            <section id="privacy-data-we-collect" className="scroll-mt-24">
                <h3>Data we collect</h3>
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
                        and per-transfer file counts, used solely to enforce Free-plan limits and to
                        show your usage in the app. We do not store the contents of your transfers.
                    </li>
                    <li>
                        <strong>Payment receipt data.</strong> For the Paid Plan, Apple sends us a
                        purchase receipt confirming a transaction occurred. We never receive your card
                        number, billing address, or any other payment-instrument details.
                    </li>
                    <li>
                        <strong>Operational logs.</strong> Short-lived server logs (request timing,
                        error traces) that may include IP addresses for abuse prevention. These logs do
                        not include the content of your transfers.
                    </li>
                </ul>
            </section>

            <section id="privacy-how-we-use-data" className="scroll-mt-24">
                <h3>How we use data</h3>
                <ul>
                    <li>To authenticate you and keep your session secure.</li>
                    <li>To enforce Free-plan limits and surface your remaining quota.</li>
                    <li>To reconcile Paid Plan purchases with Apple.</li>
                    <li>To detect and respond to abuse of the service or relay infrastructure.</li>
                    <li>To meet legal obligations when required by valid legal process.</li>
                </ul>
                <p>
                    We do not use your data to build advertising profiles, train models, sell access,
                    or any other secondary purpose.
                </p>
            </section>

            <section id="privacy-no-third-party-sharing" className="scroll-mt-24">
                <h3>No third-party sharing for analysis</h3>
                <p>
                    <strong>
                        Bytover does not share, sell, rent, or otherwise provide your data to any third
                        party for analysis, advertising, profiling, audience-building, or any other
                        commercial purpose.
                    </strong>
                </p>
                <p>
                    The companies listed in{' '}
                    <a href="#privacy-sub-processors">Operational sub-processors</a> are strictly the
                    infrastructure providers that make the product work — they receive only the
                    minimum identifiers necessary for that operational role and are contractually
                    bound to use that data only on our instructions.
                </p>
            </section>

            <section id="privacy-sub-processors" className="scroll-mt-24">
                <h3>Operational sub-processors</h3>
                <p>The following providers process limited data on our behalf:</p>
                <ul>
                    <li>
                        <strong>Apple (App Store / StoreKit).</strong> Receives your Paid Plan purchase
                        and returns a receipt to us. Apple does not receive any transfer content or
                        transfer metadata from Bytover.
                    </li>
                    <li>
                        <strong>OAuth identity providers (Google, Apple Sign In).</strong>{' '}
                        Authenticate your sign-in and return an identifier to Bytover. They do not
                        receive any transfer content or transfer metadata.
                    </li>
                    <li>
                        <strong>Cloud hosting and TURN-relay infrastructure.</strong> Encrypted packets
                        traverse this infrastructure when used. The provider cannot read packet
                        contents, and we do not retain the packets after they are forwarded.
                    </li>
                </ul>
                <p>
                    None of these providers receive your data for analysis, profiling, or advertising.
                </p>
            </section>

            <section id="privacy-transfer-architecture" className="scroll-mt-24">
                <h3>How transfers work (and why we see so little)</h3>
                <div className="not-prose grid grid-cols-1 gap-4 my-6">
                    <div className="border border-zinc-800 rounded-lg p-4">
                        <h4 className="font-semibold mb-2">Peer-to-peer (P2P)</h4>
                        <p className="text-muted-foreground leading-relaxed">
                            On Desktop and Web, Bytover uses WebRTC to send files directly from your
                            device to the recipient&apos;s device. The vast majority of transfers happen
                            this way. Your files do not touch our servers.
                        </p>
                    </div>
                    <div className="border border-zinc-800 rounded-lg p-4">
                        <h4 className="font-semibold mb-2">Cloud transfer</h4>
                        <p className="text-muted-foreground leading-relaxed">
                            When P2P is unavailable, we offer cloud-based transfer. Files are encrypted
                            in transit and automatically deleted after the transfer completes.
                        </p>
                    </div>
                    <div className="border border-zinc-800 rounded-lg p-4">
                        <h4 className="font-semibold mb-2">TURN relay (last resort)</h4>
                        <p className="text-muted-foreground leading-relaxed">
                            When direct P2P cannot be established (for example, on networks with
                            symmetric NAT or restrictive firewalls), encrypted packets are forwarded
                            through a TURN relay. The relay forwards in real time and immediately
                            discards each packet — nothing is stored.
                        </p>
                    </div>
                </div>
            </section>

            <section id="privacy-retention" className="scroll-mt-24">
                <h3>Retention</h3>
                <ul>
                    <li>Account data and transfer metadata: retained while your account is active.</li>
                    <li>
                        Cloud-transfer file payloads: deleted automatically after transfer completion
                        or within a short retention window if pickup never occurs.
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

            <section id="privacy-international-transfers" className="scroll-mt-24">
                <h3>International transfers</h3>
                <p>
                    Bytover operates globally. Where your data is processed in a country other than
                    where you reside, we rely on appropriate transfer mechanisms (such as the Standard
                    Contractual Clauses for transfers from the European Economic Area or the United
                    Kingdom) to provide an equivalent level of protection.
                </p>
            </section>

            <section id="privacy-your-rights" className="scroll-mt-24">
                <h3>Your rights</h3>
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
                        California (CCPA) — know what we collect, request deletion, and confirm we do
                        not sell your data. We do not sell your data.
                    </li>
                </ul>
                <p>
                    To exercise any of these rights, email{' '}
                    <a href="mailto:team@bytover.com">team@bytover.com</a>. We respond within 30 days.
                </p>
            </section>

            <section id="privacy-account-deletion" className="scroll-mt-24">
                <h3>Account deletion</h3>
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
                    Account data and transfer metadata are removed within 30 days of a confirmed
                    deletion request. Active P2P sessions terminate immediately.
                </p>
            </section>

            <section id="privacy-children" className="scroll-mt-24">
                <h3>Children</h3>
                <p>
                    Bytover is not directed to children under 13, and our App Store age rating reflects
                    the intended audience. We do not knowingly collect personal data from children. If
                    you believe a child has provided us with personal data, contact{' '}
                    <a href="mailto:team@bytover.com">team@bytover.com</a> and we will delete it.
                </p>
            </section>

            <section id="privacy-cookies-and-storage" className="scroll-mt-24">
                <h3>Cookies and local storage</h3>
                <p>
                    Bytover does not set advertising or tracking cookies. The web app uses strictly
                    necessary storage on your device:
                </p>
                <ul>
                    <li>An authentication cookie to keep you signed in.</li>
                    <li>
                        OPFS (Origin Private File System) storage to manage in-progress transfers.
                    </li>
                    <li>LocalStorage for UI preferences such as theme.</li>
                </ul>
                <p>None of this storage is shared with third parties.</p>
            </section>

            <section id="privacy-security" className="scroll-mt-24">
                <h3>Security</h3>
                <ul>
                    <li>End-to-end encryption (DTLS) on every WebRTC transfer.</li>
                    <li>TLS for all client–server traffic.</li>
                    <li>No server-side storage of P2P transfer contents.</li>
                    <li>Tokens are bound to a device fingerprint to limit replay risk.</li>
                </ul>
            </section>

            <section id="privacy-changes" className="scroll-mt-24">
                <h3>Changes to this policy</h3>
                <p>
                    When we update this policy we change the effective date at the top of this section.
                    For material changes affecting your rights or how we process your data, we will
                    also notify you via email or an in-app banner before the change takes effect.
                </p>
            </section>
        </section>
    );
}

function TermsSection() {
    return (
        <section id="terms" className="scroll-mt-24">
            <h2>Terms of Service</h2>
            <EffectiveLine iso={TERMS_EFFECTIVE_DATE} />

            <section id="terms-agreement" className="scroll-mt-24">
                <h3>The agreement</h3>
                <p>
                    By creating a Bytover account, installing a Bytover client, or using the Bytover
                    web app, you agree to these Terms of Service and to the{' '}
                    <a href="#privacy">Privacy Policy</a> above. If you do not agree, you must not use
                    Bytover.
                </p>
            </section>

            <section id="terms-eligibility" className="scroll-mt-24">
                <h3>Eligibility</h3>
                <p>
                    You must be at least the age of majority in your jurisdiction (and in any case at
                    least 13) to use Bytover. If you accept these Terms on behalf of an organization,
                    you represent that you have authority to bind that organization.
                </p>
            </section>

            <section id="terms-accounts" className="scroll-mt-24">
                <h3>Your account</h3>
                <ul>
                    <li>You are responsible for the accuracy of the information you provide.</li>
                    <li>You are responsible for keeping your sign-in credentials and devices secure.</li>
                    <li>
                        You may not create accounts by automated means or maintain more than one
                        personal account.
                    </li>
                    <li>
                        You must notify us at{' '}
                        <a href="mailto:team@bytover.com">team@bytover.com</a> if you suspect
                        unauthorized access to your account.
                    </li>
                </ul>
            </section>

            <section id="terms-license" className="scroll-mt-24">
                <h3>License to use Bytover</h3>
                <p>
                    Subject to these Terms and the <a href="#eula">End User License Agreement</a>{' '}
                    below, Bytover grants you a limited, non-exclusive, revocable, non-transferable
                    license to install and use the Bytover client software on devices you own or
                    control, and to use the Bytover web application, for personal or internal business
                    purposes.
                </p>
            </section>

            <section id="terms-acceptable-use" className="scroll-mt-24">
                <h3>Acceptable use</h3>
                <p>You agree not to use Bytover to:</p>
                <ul>
                    <li>Transmit content that is illegal in your jurisdiction or the recipient&apos;s.</li>
                    <li>Distribute malware, ransomware, or other harmful code.</li>
                    <li>
                        Abuse our relay or signalling infrastructure as an open proxy, an
                        anonymization layer, or a traffic-amplification vector.
                    </li>
                    <li>
                        Infringe any third party&apos;s intellectual property or privacy rights.
                    </li>
                    <li>Harass, threaten, or send unsolicited bulk content to other users.</li>
                    <li>
                        Reverse-engineer, decompile, or attempt to extract source code, except where
                        this restriction is prohibited by applicable law.
                    </li>
                    <li>
                        Probe, scan, or test the vulnerability of any Bytover system without prior
                        written authorization.
                    </li>
                    <li>
                        Create accounts by automated means or to circumvent plan limits.
                    </li>
                </ul>
            </section>

            <section id="terms-user-content" className="scroll-mt-24">
                <h3>Your files and content</h3>
                <p>
                    You own the files you transfer through Bytover. We claim no ownership and do not
                    use your files to train models, target advertising, or any other secondary purpose.
                </p>
                <p>
                    To operate the service, you grant Bytover a strictly limited license to host
                    encrypted file payloads briefly when cloud transfer is used, and to forward
                    encrypted packets when relay is used. This license exists only to make the
                    transfer work and ends when the transfer ends.
                </p>
            </section>

            <section id="terms-dmca" className="scroll-mt-24">
                <h3>DMCA and copyright</h3>
                <p>
                    Bytover responds to notices of alleged copyright infringement consistent with the
                    Digital Millennium Copyright Act, 17 U.S.C. § 512.
                </p>
                <p>
                    To submit a takedown notice, send the following to{' '}
                    <a href="mailto:team@bytover.com">team@bytover.com</a>:
                </p>
                <ul>
                    <li>An identification of the copyrighted work claimed to have been infringed.</li>
                    <li>
                        An identification of the allegedly infringing material sufficient for us to
                        locate it.
                    </li>
                    <li>Your contact information (name, address, telephone, email).</li>
                    <li>
                        A statement that you have a good-faith belief that the use is not authorized
                        by the copyright owner, its agent, or the law.
                    </li>
                    <li>
                        A statement, made under penalty of perjury, that the information in the notice
                        is accurate and that you are authorized to act on the copyright owner&apos;s
                        behalf.
                    </li>
                    <li>Your physical or electronic signature.</li>
                </ul>
                <p>
                    Counter-notices may be submitted to the same address with the elements required by
                    17 U.S.C. § 512(g)(3). Bytover terminates the accounts of repeat infringers.
                </p>
            </section>

            <section id="terms-suspension" className="scroll-mt-24">
                <h3>Suspension and termination</h3>
                <p>
                    We may suspend or terminate your access if we reasonably believe you have breached
                    these Terms, if required by law, or to protect the security of the service or
                    other users. Where practical we will give notice and an opportunity to cure.
                </p>
                <p>
                    You may stop using Bytover at any time by deleting your account (see{' '}
                    <a href="#privacy-account-deletion">Account deletion</a>).
                </p>
            </section>

            <section id="terms-paid-plan" className="scroll-mt-24">
                <h3>Paid Plan</h3>
                <p>
                    Bytover offers a Paid Plan as a one-time purchase of US$20. There is no recurring
                    billing. The Paid Plan unlocks the capabilities described on our pricing page and
                    is further governed by the <a href="#eula">End User License Agreement</a>.
                </p>
            </section>

            <section id="terms-third-party-services" className="scroll-mt-24">
                <h3>Third-party services</h3>
                <p>
                    Bytover integrates with third-party services to operate, including Apple App Store
                    / StoreKit and OAuth identity providers (such as Google and Apple Sign In). Your
                    use of those services is governed by their own terms and policies.
                </p>
            </section>

            <section id="terms-warranty" className="scroll-mt-24">
                <h3>No warranty</h3>
                <p>
                    Bytover is provided &quot;as is&quot; and &quot;as available&quot;. To the maximum
                    extent permitted by law, we disclaim all warranties, express or implied, including
                    merchantability, fitness for a particular purpose, and non-infringement. Some
                    consumer protections cannot be waived under your local law and continue to apply.
                </p>
            </section>

            <section id="terms-liability" className="scroll-mt-24">
                <h3>Limitation of liability</h3>
                <p>
                    To the maximum extent permitted by law, Bytover&apos;s aggregate liability for any
                    claim arising out of or relating to these Terms or your use of Bytover will not
                    exceed the greater of (a) the amount you paid Bytover in the twelve months before
                    the event giving rise to the claim, or (b) US$20. We are not liable for indirect,
                    incidental, special, consequential, or punitive damages, or for lost profits or
                    revenue. Nothing in these Terms limits liability that cannot be limited under
                    applicable law (including liability for gross negligence or fraud).
                </p>
            </section>

            <section id="terms-indemnification" className="scroll-mt-24">
                <h3>Indemnification</h3>
                <p>
                    You agree to indemnify and hold Bytover harmless from any claim, loss, or expense
                    (including reasonable legal fees) arising from your breach of these Terms, your
                    misuse of the service, or your violation of any law or third-party right.
                </p>
            </section>

            <section id="terms-governing-law" className="scroll-mt-24">
                <h3>Governing law</h3>
                <p>
                    These Terms are governed by the laws of the jurisdiction in which Bytover is
                    incorporated, without regard to conflict-of-laws principles, except that mandatory
                    consumer-protection laws of your country of residence continue to apply where
                    required. Disputes will be resolved in the courts of that jurisdiction unless
                    applicable law gives you the right to bring a claim where you live.
                </p>
            </section>

            <section id="terms-changes" className="scroll-mt-24">
                <h3>Changes to these Terms</h3>
                <p>
                    We may update these Terms from time to time. The effective date at the top of this
                    section reflects the latest version. For material changes affecting your rights,
                    we will notify you in advance via email or an in-app banner.
                </p>
            </section>
        </section>
    );
}

function EulaSection() {
    return (
        <section id="eula" className="scroll-mt-24">
            <h2>End User License Agreement</h2>
            <EffectiveLine iso={EULA_EFFECTIVE_DATE} />

            <section id="eula-scope" className="scroll-mt-24">
                <h3>Scope</h3>
                <p>
                    This End User License Agreement (&quot;EULA&quot;) governs your installation and
                    use of the Bytover client software on Web, Desktop, iOS, and Android. It is in
                    addition to the <a href="#terms">Terms of Service</a> and{' '}
                    <a href="#privacy">Privacy Policy</a> above. For Bytover apps distributed through
                    the Apple App Store, the additional terms in{' '}
                    <a href="#eula-apple">Apple App Store distribution</a> also apply.
                </p>
            </section>

            <section id="eula-license-grant" className="scroll-mt-24">
                <h3>License grant</h3>
                <p>
                    Subject to your compliance with this EULA, Bytover grants you a limited,
                    non-exclusive, revocable, non-transferable, non-sublicensable license to install
                    and run the Bytover client software on devices you own or control, for your
                    personal or internal business use.
                </p>
            </section>

            <section id="eula-restrictions" className="scroll-mt-24">
                <h3>Restrictions</h3>
                <p>You may not:</p>
                <ul>
                    <li>Sell, rent, lease, or sublicense the software.</li>
                    <li>
                        Reverse-engineer, decompile, or disassemble the software, except to the extent
                        such restriction is prohibited by applicable law.
                    </li>
                    <li>Remove, obscure, or alter any proprietary notices in the software.</li>
                    <li>
                        Use the software to build a competing product or to evade the limits of the
                        Free plan.
                    </li>
                </ul>
            </section>

            <section id="eula-paid-plan" className="scroll-mt-24">
                <h3>Paid Plan — one-time purchase</h3>
                <p>The Bytover Paid Plan is a single, one-time purchase.</p>
                <ul>
                    <li>
                        <strong>Price:</strong> US$20, charged once at the time of purchase. Local
                        currency conversion and applicable taxes are determined by the store you
                        purchase through.
                    </li>
                    <li>
                        <strong>Billing model:</strong> One-time only. There is no recurring charge,
                        no auto-renewal, and no subscription period.
                    </li>
                    <li>
                        <strong>What you get:</strong> The capabilities listed on our pricing page,
                        including removal of the Free-plan transfer caps and access to
                        password-protected transfers.
                    </li>
                    <li>
                        <strong>Refunds:</strong> Purchases made through the Apple App Store are
                        subject to Apple&apos;s refund policies; refund requests are handled by Apple.
                        Purchases made through other stores follow that store&apos;s refund policy.
                        For direct-to-Bytover purchases, contact{' '}
                        <a href="mailto:team@bytover.com">team@bytover.com</a> within a reasonable
                        period.
                    </li>
                </ul>
            </section>

            <section id="eula-no-subscriptions" className="scroll-mt-24">
                <h3>No auto-renewing subscriptions</h3>
                <p>
                    Bytover does not currently offer auto-renewable subscriptions. The Paid Plan
                    described above is the only paid product. You will not be charged any recurring
                    fee for using Bytover. If we ever introduce a subscription product, this EULA will
                    be updated and you will be notified before any subscription is offered.
                </p>
            </section>

            <section id="eula-apple" className="scroll-mt-24">
                <h3>Apple App Store distribution</h3>
                <p>
                    For Bytover apps you obtain through the Apple App Store, the following additional
                    terms apply:
                </p>
                <ul>
                    <li>
                        This EULA is between you and Bytover only, not with Apple. Apple is not
                        responsible for the app or its content.
                    </li>
                    <li>
                        The license granted is limited to use on Apple-branded products that you own
                        or control, as permitted by the Apple Media Services Terms.
                    </li>
                    <li>
                        Apple has no obligation to provide maintenance or support services for the
                        app.
                    </li>
                    <li>
                        In the event of any failure of the app to conform to any applicable warranty,
                        you may notify Apple, and Apple will refund the purchase price; to the
                        maximum extent permitted by law, Apple has no other warranty obligation with
                        respect to the app.
                    </li>
                    <li>
                        Bytover, not Apple, is responsible for addressing any claims by you or any
                        third party relating to the app or your use of it (including
                        product-liability, legal-compliance, and intellectual-property claims).
                    </li>
                    <li>
                        <strong>
                            Apple and Apple&apos;s subsidiaries are third-party beneficiaries of this
                            EULA, and upon your acceptance of this EULA, Apple has the right (and is
                            deemed to have accepted the right) to enforce this EULA against you as a
                            third-party beneficiary.
                        </strong>
                    </li>
                    <li>
                        Apple&apos;s standard licensed-application end-user license agreement is
                        available at{' '}
                        <a
                            href="https://www.apple.com/legal/internet-services/itunes/dev/stdeula/"
                            target="_blank"
                            rel="noreferrer noopener"
                        >
                            apple.com/legal/internet-services/itunes/dev/stdeula
                        </a>
                        .
                    </li>
                </ul>
            </section>

            <section id="eula-updates" className="scroll-mt-24">
                <h3>Updates</h3>
                <p>
                    Bytover may release updates to fix issues, improve performance, or add features.
                    Updates are governed by this EULA. We have no obligation to support unsupported
                    operating-system versions or end-of-life devices.
                </p>
            </section>

            <section id="eula-termination" className="scroll-mt-24">
                <h3>Termination</h3>
                <p>
                    This EULA is effective until terminated. It terminates automatically if you breach
                    any of its terms. On termination you must stop using the software and remove it
                    from your devices. The provisions on Restrictions, the Apple
                    third-party-beneficiary clause, governing law, and any limitations of liability
                    survive termination.
                </p>
            </section>

            <section id="eula-export" className="scroll-mt-24">
                <h3>Export control</h3>
                <p>
                    You may not use, export, or re-export the Bytover software in violation of any
                    applicable export-control laws, including those of the United States. By using
                    the software you represent that you are not located in any country subject to a
                    US Government embargo and are not on any US Government list of prohibited or
                    restricted parties.
                </p>
            </section>

            <section id="eula-governing-law" className="scroll-mt-24">
                <h3>Governing law</h3>
                <p>
                    This EULA is governed by the same law that governs the{' '}
                    <a href="#terms-governing-law">Terms of Service</a> above, except that
                    distribution through the Apple App Store is additionally subject to the
                    choice-of-law provisions in Apple&apos;s standard licensed-application EULA.
                </p>
            </section>
        </section>
    );
}

function ContactSection() {
    return (
        <section id="contact" className="scroll-mt-24">
            <h2>Contact</h2>
            <p>
                Questions about any of the documents above — Privacy, Terms, or EULA — can be sent to{' '}
                <a href="mailto:team@bytover.com">team@bytover.com</a>. You can also visit our{' '}
                <Link href="/contact">contact page</Link>.
            </p>
        </section>
    );
}
