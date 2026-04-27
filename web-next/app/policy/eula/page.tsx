import type { Metadata } from 'next';
import Link from 'next/link';
import { getCanonicalUrl } from '@/lib/canonical';
import { LegalShell, type TocEntry } from '../_components/legal-shell';
import { EULA_EFFECTIVE_DATE } from '../_components/effective-date';

export const metadata: Metadata = {
    title: 'End User License Agreement — Bytover',
    description:
        'The license agreement for the Bytover client software, including Paid Plan one-time purchase terms.',
    alternates: {
        canonical: getCanonicalUrl('/policy/eula'),
    },
};

const TOC: TocEntry[] = [
    { id: 'scope', label: 'Scope of this EULA' },
    { id: 'license-grant', label: 'License grant' },
    { id: 'restrictions', label: 'Restrictions' },
    { id: 'paid-plan', label: 'Paid Plan (one-time)' },
    { id: 'no-subscriptions', label: 'No auto-renewal' },
    { id: 'apple-eula', label: 'Apple App Store distribution' },
    { id: 'updates', label: 'Updates' },
    { id: 'termination', label: 'Termination' },
    { id: 'export', label: 'Export control' },
    { id: 'governing-law', label: 'Governing law' },
    { id: 'contact', label: 'Contact' },
];

export default function EulaPage() {
    return (
        <LegalShell
            title="End User License Agreement"
            effectiveDate={EULA_EFFECTIVE_DATE}
            toc={TOC}
            intro={
                <p>
                    This End User License Agreement (&quot;EULA&quot;) governs your installation and use
                    of the Bytover client software, including any Paid Plan purchase you make.
                </p>
            }
        >
            <section id="scope" className="mb-12 scroll-mt-24">
                <h2>Scope of this EULA</h2>
                <p>
                    This EULA applies to the Bytover client software on Web, Desktop, iOS, and Android.
                    It is in addition to our{' '}
                    <Link href="/policy/terms">Terms of Service</Link> and{' '}
                    <Link href="/policy/privacy">Privacy Policy</Link>. For Bytover apps distributed
                    through the Apple App Store, the additional terms in{' '}
                    <a href="#apple-eula">Apple App Store distribution</a> also apply.
                </p>
            </section>

            <section id="license-grant" className="mb-12 scroll-mt-24">
                <h2>License grant</h2>
                <p>
                    Subject to your compliance with this EULA, Bytover grants you a limited,
                    non-exclusive, revocable, non-transferable, non-sublicensable license to install and
                    run the Bytover client software on devices you own or control, for your personal or
                    internal business use.
                </p>
            </section>

            <section id="restrictions" className="mb-12 scroll-mt-24">
                <h2>Restrictions</h2>
                <p>You may not:</p>
                <ul>
                    <li>Sell, rent, lease, or sublicense the software.</li>
                    <li>
                        Reverse-engineer, decompile, or disassemble the software, except to the extent
                        such restriction is prohibited by applicable law.
                    </li>
                    <li>Remove, obscure, or alter any proprietary notices in the software.</li>
                    <li>
                        Use the software to build a competing product or to evade the limits of the Free
                        plan.
                    </li>
                </ul>
            </section>

            <section id="paid-plan" className="mb-12 scroll-mt-24">
                <h2>Paid Plan — one-time purchase</h2>
                <p>The Bytover Paid Plan is a single, one-time purchase.</p>
                <ul>
                    <li>
                        <strong>Price:</strong> US$20, charged once at the time of purchase. Local
                        currency conversion and applicable taxes are determined by the store you purchase
                        through.
                    </li>
                    <li>
                        <strong>Billing model:</strong> One-time only. There is no recurring charge, no
                        auto-renewal, and no subscription period.
                    </li>
                    <li>
                        <strong>What you get:</strong> The capabilities listed on our pricing page,
                        including removal of the Free-plan transfer caps and access to password-protected
                        transfers.
                    </li>
                    <li>
                        <strong>Refunds:</strong> Purchases made through the Apple App Store are subject
                        to Apple&apos;s refund policies; refund requests are handled by Apple. Purchases
                        made through other stores follow that store&apos;s refund policy. For
                        direct-to-Bytover purchases, contact{' '}
                        <a href="mailto:team@bytover.com">team@bytover.com</a> within a reasonable
                        period.
                    </li>
                </ul>
            </section>

            <section id="no-subscriptions" className="mb-12 scroll-mt-24">
                <h2>No auto-renewing subscriptions</h2>
                <p>
                    Bytover does not currently offer auto-renewable subscriptions. The Paid Plan
                    described above is the only paid product. You will not be charged any recurring fee
                    for using Bytover. If we ever introduce a subscription product, this EULA will be
                    updated and you will be notified before any subscription is offered.
                </p>
            </section>

            <section id="apple-eula" className="mb-12 scroll-mt-24">
                <h2>Apple App Store distribution</h2>
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
                        The license granted is limited to use on Apple-branded products that you own or
                        control, as permitted by the Apple Media Services Terms.
                    </li>
                    <li>
                        Apple has no obligation to provide maintenance or support services for the app.
                    </li>
                    <li>
                        In the event of any failure of the app to conform to any applicable warranty,
                        you may notify Apple, and Apple will refund the purchase price; to the maximum
                        extent permitted by law, Apple has no other warranty obligation with respect to
                        the app.
                    </li>
                    <li>
                        Bytover, not Apple, is responsible for addressing any claims by you or any third
                        party relating to the app or your use of it (including product-liability,
                        legal-compliance, and intellectual-property claims).
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

            <section id="updates" className="mb-12 scroll-mt-24">
                <h2>Updates</h2>
                <p>
                    Bytover may release updates to fix issues, improve performance, or add features.
                    Updates are governed by this EULA. We have no obligation to support unsupported
                    operating-system versions or end-of-life devices.
                </p>
            </section>

            <section id="termination" className="mb-12 scroll-mt-24">
                <h2>Termination</h2>
                <p>
                    This EULA is effective until terminated. It terminates automatically if you breach
                    any of its terms. On termination you must stop using the software and remove it from
                    your devices. The provisions on Restrictions, the Apple third-party-beneficiary
                    clause, governing law, and any limitations of liability survive termination.
                </p>
            </section>

            <section id="export" className="mb-12 scroll-mt-24">
                <h2>Export control</h2>
                <p>
                    You may not use, export, or re-export the Bytover software in violation of any
                    applicable export-control laws, including those of the United States. By using the
                    software you represent that you are not located in any country subject to a US
                    Government embargo and are not on any US Government list of prohibited or restricted
                    parties.
                </p>
            </section>

            <section id="governing-law" className="mb-12 scroll-mt-24">
                <h2>Governing law</h2>
                <p>
                    This EULA is governed by the same law that governs our{' '}
                    <Link href="/policy/terms#governing-law">Terms of Service</Link>, except that
                    distribution through the Apple App Store is additionally subject to the choice-of-law
                    provisions in Apple&apos;s standard licensed-application EULA.
                </p>
            </section>

            <section id="contact" className="mb-12 scroll-mt-24">
                <h2>Contact</h2>
                <p>
                    Questions about this EULA can be sent to{' '}
                    <a href="mailto:team@bytover.com">team@bytover.com</a>.
                </p>
            </section>
        </LegalShell>
    );
}
