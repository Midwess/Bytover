import type { Metadata } from 'next';
import Link from 'next/link';
import { getCanonicalUrl } from '@/lib/canonical';
import { LegalShell, type TocEntry } from '../_components/legal-shell';
import { TERMS_EFFECTIVE_DATE } from '../_components/effective-date';

export const metadata: Metadata = {
    title: 'Terms of Service — Bytover',
    description:
        'The agreement governing your use of Bytover, including acceptable use, DMCA, and account responsibilities.',
    alternates: {
        canonical: getCanonicalUrl('/policy/terms'),
    },
};

const TOC: TocEntry[] = [
    { id: 'agreement', label: 'The agreement' },
    { id: 'eligibility', label: 'Eligibility' },
    { id: 'accounts', label: 'Your account' },
    { id: 'license', label: 'License to use Bytover' },
    { id: 'acceptable-use', label: 'Acceptable use' },
    { id: 'user-content', label: 'Your files & content' },
    { id: 'dmca', label: 'DMCA & copyright' },
    { id: 'suspension', label: 'Suspension & termination' },
    { id: 'paid-plan', label: 'Paid Plan' },
    { id: 'third-party-services', label: 'Third-party services' },
    { id: 'warranty', label: 'No warranty' },
    { id: 'liability', label: 'Limitation of liability' },
    { id: 'indemnification', label: 'Indemnification' },
    { id: 'governing-law', label: 'Governing law' },
    { id: 'changes', label: 'Changes' },
    { id: 'contact', label: 'Contact' },
];

export default function TermsOfServicePage() {
    return (
        <LegalShell
            title="Terms of Service"
            effectiveDate={TERMS_EFFECTIVE_DATE}
            toc={TOC}
            intro={
                <p>
                    These Terms of Service form the agreement between you and Bytover when you use our
                    apps and services. Please read them carefully.
                </p>
            }
        >
            <section id="agreement" className="mb-12 scroll-mt-24">
                <h2>The agreement</h2>
                <p>
                    By creating a Bytover account, installing a Bytover client, or using the Bytover web
                    app, you agree to these Terms of Service and to our{' '}
                    <Link href="/policy/privacy">Privacy Policy</Link>. If you do not agree, you must not
                    use Bytover.
                </p>
            </section>

            <section id="eligibility" className="mb-12 scroll-mt-24">
                <h2>Eligibility</h2>
                <p>
                    You must be at least the age of majority in your jurisdiction (and in any case at
                    least 13) to use Bytover. If you accept these Terms on behalf of an organization, you
                    represent that you have authority to bind that organization.
                </p>
            </section>

            <section id="accounts" className="mb-12 scroll-mt-24">
                <h2>Your account</h2>
                <ul>
                    <li>You are responsible for the accuracy of the information you provide.</li>
                    <li>
                        You are responsible for keeping your sign-in credentials and devices secure.
                    </li>
                    <li>
                        You may not create accounts by automated means or maintain more than one personal
                        account.
                    </li>
                    <li>
                        You must notify us at <a href="mailto:team@bytover.com">team@bytover.com</a> if
                        you suspect unauthorized access to your account.
                    </li>
                </ul>
            </section>

            <section id="license" className="mb-12 scroll-mt-24">
                <h2>License to use Bytover</h2>
                <p>
                    Subject to these Terms and the{' '}
                    <Link href="/policy/eula">End User License Agreement</Link>, Bytover grants you a
                    limited, non-exclusive, revocable, non-transferable license to install and use the
                    Bytover client software on devices you own or control, and to use the Bytover web
                    application, for personal or internal business purposes.
                </p>
            </section>

            <section id="acceptable-use" className="mb-12 scroll-mt-24">
                <h2>Acceptable use</h2>
                <p>You agree not to use Bytover to:</p>
                <ul>
                    <li>Transmit content that is illegal in your jurisdiction or the recipient&apos;s.</li>
                    <li>Distribute malware, ransomware, or other harmful code.</li>
                    <li>
                        Abuse our relay or signalling infrastructure as an open proxy, an anonymization
                        layer, or a traffic-amplification vector.
                    </li>
                    <li>Infringe any third party&apos;s intellectual property or privacy rights.</li>
                    <li>Harass, threaten, or send unsolicited bulk content to other users.</li>
                    <li>Reverse-engineer, decompile, or attempt to extract source code, except where this restriction is prohibited by applicable law.</li>
                    <li>
                        Probe, scan, or test the vulnerability of any Bytover system without prior written
                        authorization.
                    </li>
                    <li>Create accounts by automated means or to circumvent plan limits.</li>
                </ul>
            </section>

            <section id="user-content" className="mb-12 scroll-mt-24">
                <h2>Your files and content</h2>
                <p>
                    You own the files you transfer through Bytover. We claim no ownership and do not use
                    your files to train models, target advertising, or any other secondary purpose.
                </p>
                <p>
                    To operate the service, you grant Bytover a strictly limited license to host
                    encrypted file payloads briefly when cloud transfer is used, and to forward encrypted
                    packets when relay is used. This license exists only to make the transfer work and
                    ends when the transfer ends.
                </p>
            </section>

            <section id="dmca" className="mb-12 scroll-mt-24">
                <h2>DMCA and copyright</h2>
                <p>
                    Bytover responds to notices of alleged copyright infringement consistent with the
                    Digital Millennium Copyright Act, 17 U.S.C. § 512.
                </p>
                <p>To submit a takedown notice, send the following to{' '}
                    <a href="mailto:team@bytover.com">team@bytover.com</a>:
                </p>
                <ul>
                    <li>An identification of the copyrighted work claimed to have been infringed.</li>
                    <li>An identification of the allegedly infringing material sufficient for us to locate it.</li>
                    <li>Your contact information (name, address, telephone, email).</li>
                    <li>
                        A statement that you have a good-faith belief that the use is not authorized by
                        the copyright owner, its agent, or the law.
                    </li>
                    <li>
                        A statement, made under penalty of perjury, that the information in the notice is
                        accurate and that you are authorized to act on the copyright owner&apos;s behalf.
                    </li>
                    <li>Your physical or electronic signature.</li>
                </ul>
                <p>
                    Counter-notices may be submitted to the same address with the elements required by
                    17 U.S.C. § 512(g)(3). Bytover terminates the accounts of repeat infringers.
                </p>
            </section>

            <section id="suspension" className="mb-12 scroll-mt-24">
                <h2>Suspension and termination</h2>
                <p>
                    We may suspend or terminate your access if we reasonably believe you have breached
                    these Terms, if required by law, or to protect the security of the service or other
                    users. Where practical we will give notice and an opportunity to cure.
                </p>
                <p>
                    You may stop using Bytover at any time by deleting your account (see{' '}
                    <Link href="/policy/privacy#account-deletion">Account deletion</Link>).
                </p>
            </section>

            <section id="paid-plan" className="mb-12 scroll-mt-24">
                <h2>Paid Plan</h2>
                <p>
                    Bytover offers a Paid Plan as a one-time purchase of US$20. There is no recurring
                    billing. The Paid Plan unlocks the capabilities described on our pricing page and is
                    further governed by the{' '}
                    <Link href="/policy/eula">End User License Agreement</Link>.
                </p>
            </section>

            <section id="third-party-services" className="mb-12 scroll-mt-24">
                <h2>Third-party services</h2>
                <p>
                    Bytover integrates with third-party services to operate, including Apple App Store /
                    StoreKit and OAuth identity providers (such as Google and Apple Sign In). Your use of
                    those services is governed by their own terms and policies.
                </p>
            </section>

            <section id="warranty" className="mb-12 scroll-mt-24">
                <h2>No warranty</h2>
                <p>
                    Bytover is provided &quot;as is&quot; and &quot;as available&quot;. To the maximum
                    extent permitted by law, we disclaim all warranties, express or implied, including
                    merchantability, fitness for a particular purpose, and non-infringement. Some
                    consumer protections cannot be waived under your local law and continue to apply.
                </p>
            </section>

            <section id="liability" className="mb-12 scroll-mt-24">
                <h2>Limitation of liability</h2>
                <p>
                    To the maximum extent permitted by law, Bytover&apos;s aggregate liability for any
                    claim arising out of or relating to these Terms or your use of Bytover will not
                    exceed the greater of (a) the amount you paid Bytover in the twelve months before the
                    event giving rise to the claim, or (b) US$20. We are not liable for indirect,
                    incidental, special, consequential, or punitive damages, or for lost profits or
                    revenue. Nothing in these Terms limits liability that cannot be limited under
                    applicable law (including liability for gross negligence or fraud).
                </p>
            </section>

            <section id="indemnification" className="mb-12 scroll-mt-24">
                <h2>Indemnification</h2>
                <p>
                    You agree to indemnify and hold Bytover harmless from any claim, loss, or expense
                    (including reasonable legal fees) arising from your breach of these Terms, your
                    misuse of the service, or your violation of any law or third-party right.
                </p>
            </section>

            <section id="governing-law" className="mb-12 scroll-mt-24">
                <h2>Governing law</h2>
                <p>
                    These Terms are governed by the laws of the jurisdiction in which Bytover is
                    incorporated, without regard to conflict-of-laws principles, except that mandatory
                    consumer-protection laws of your country of residence continue to apply where
                    required. Disputes will be resolved in the courts of that jurisdiction unless
                    applicable law gives you the right to bring a claim where you live.
                </p>
            </section>

            <section id="changes" className="mb-12 scroll-mt-24">
                <h2>Changes to these Terms</h2>
                <p>
                    We may update these Terms from time to time. The effective date at the top of this
                    page reflects the latest version. For material changes affecting your rights, we will
                    notify you in advance via email or an in-app banner.
                </p>
            </section>

            <section id="contact" className="mb-12 scroll-mt-24">
                <h2>Contact</h2>
                <p>
                    Questions about these Terms can be sent to{' '}
                    <a href="mailto:team@bytover.com">team@bytover.com</a>.
                </p>
            </section>
        </LegalShell>
    );
}
