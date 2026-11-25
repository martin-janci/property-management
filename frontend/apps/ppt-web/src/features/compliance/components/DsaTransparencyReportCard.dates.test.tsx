/// <reference types="vitest/globals" />
/**
 * DsaTransparencyReportCard date-localization regression test (#3791).
 *
 * History: after PR #3783 localized every string of the card into all six
 * shipped locales (`dsa.card.*`), the date fields were still formatted through
 * a hardcoded `toLocaleDateString('en-GB', …)` helper. The period line
 * (`dsa.card.period` → `{{start}} - {{end}}`) and the `dsa.card.publishedOn`
 * line therefore rendered English month abbreviations ("Oct", "Nov") inside an
 * otherwise fully-translated card for Slovak/Czech/German/Polish/Hungarian
 * managers.
 *
 * The helper now formats off the active i18n language. These tests render the
 * card under a non-English locale (German) and assert the month segment is the
 * localized abbreviation ("Okt.") rather than the English one ("Oct"). They
 * fail on `main` (dates are always `en-GB`) and pass once the helper reads
 * `i18n.language`.
 */
import { render, screen } from '@testing-library/react';
import i18n from 'i18next';
import { afterEach, describe, expect, it } from 'vitest';
import {
  type DsaTransparencyReport,
  DsaTransparencyReportCard,
} from './DsaTransparencyReportCard';

const summary: DsaTransparencyReport['summary'] = {
  total_moderation_actions: 0,
  content_removed: 0,
  content_restricted: 0,
  warnings_issued: 0,
  user_reports_received: 0,
  user_reports_resolved: 0,
  automated_decisions: 0,
  automated_decisions_overturned: 0,
  appeals_received: 0,
  appeals_upheld: 0,
  appeals_rejected: 0,
};

// October is a month whose short form differs between en-GB ("Oct") and
// German ("Okt."), making it a clean discriminator for the active locale.
const report: DsaTransparencyReport = {
  id: 'r-1',
  period_start: '2024-10-01',
  period_end: '2024-10-31',
  status: 'published',
  summary,
  content_type_breakdown: [],
  violation_type_breakdown: [],
  published_at: '2024-10-15',
};

afterEach(async () => {
  // Restore the default test language so locale state can't leak into other
  // tests sharing the global i18n instance (see src/test/setup.ts).
  await i18n.changeLanguage('en');
});

describe('DsaTransparencyReportCard date localization', () => {
  it('formats the reporting-period dates using the active locale', async () => {
    await i18n.changeLanguage('de');
    render(<DsaTransparencyReportCard report={report} showActions={false} />);

    // The period line is the only one interpolating a `{{start}} - {{end}}`
    // range, so it is the single node containing the " - " separator.
    const period = screen.getByText((content) => content.includes(' - ')).textContent ?? '';
    expect(period).toContain('Okt');
    expect(period).not.toContain('Oct');
  });

  it('formats the published-on date using the active locale', async () => {
    await i18n.changeLanguage('de');
    render(<DsaTransparencyReportCard report={report} showActions={false} />);

    // "15." is unique to the published_at date (period dates are the 1st/31st).
    const published = screen.getByText(/15\. Okt\. 2024/);
    expect(published).toBeInTheDocument();
  });

  it('still formats dates in English month abbreviations under en', async () => {
    await i18n.changeLanguage('en');
    render(<DsaTransparencyReportCard report={report} showActions={false} />);

    // Both the period and published-on lines carry the English "Oct" form.
    expect(screen.getAllByText(/Oct/).length).toBeGreaterThan(0);
  });
});
