export const PRIVACY_EFFECTIVE_DATE = '2026-04-27';
export const TERMS_EFFECTIVE_DATE = '2026-04-27';
export const EULA_EFFECTIVE_DATE = '2026-04-27';

const MONTHS = [
    'January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December',
] as const;

export function formatEffectiveDate(iso: string): string {
    const [yearStr, monthStr, dayStr] = iso.split('-');
    const year = Number(yearStr);
    const monthIndex = Number(monthStr) - 1;
    const day = Number(dayStr);

    if (
        !Number.isInteger(year) ||
        !Number.isInteger(monthIndex) ||
        !Number.isInteger(day) ||
        monthIndex < 0 ||
        monthIndex > 11
    ) {
        throw new Error(`Invalid effective date: ${iso}`);
    }

    return `${MONTHS[monthIndex]} ${day}, ${year}`;
}
