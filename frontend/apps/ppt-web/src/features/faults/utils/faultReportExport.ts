/**
 * Client-side export helpers for the Fault Reports page (Story 4.7 / BIT-186).
 *
 * The fault statistics endpoint has no server-side export, so CSV is assembled
 * in the browser and offered as a Blob download. PDF export reuses the browser
 * print pipeline (`window.print()`) with a print stylesheet on the page, which
 * keeps the bundle dependency-free (no jsPDF/pdfmake) and lets the user "Save
 * as PDF" from the print dialog.
 */

import type { FaultStatistics } from '@ppt/api-client';

/** Escape a single CSV field per RFC 4180 (quote when it contains , " or newline). */
function csvField(value: string | number | null | undefined): string {
  const s = value == null ? '' : String(value);
  return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

export interface FaultReportMeta {
  buildingLabel: string;
  dateFrom?: string;
  dateTo?: string;
}

/**
 * Build a CSV document summarising the fault statistics. Sections are separated
 * by blank lines: a header/meta block, top-line KPIs, then each breakdown.
 */
export function buildStatisticsCsv(stats: FaultStatistics, meta: FaultReportMeta): string {
  const rows: string[] = [];
  const push = (...cells: Array<string | number | null | undefined>) =>
    rows.push(cells.map(csvField).join(','));

  push('Fault Report');
  push('Building', meta.buildingLabel);
  push('Date from', meta.dateFrom || 'All time');
  push('Date to', meta.dateTo || 'All time');
  push();

  push('Metric', 'Value');
  push('Total faults', stats.total_count);
  push('Open faults', stats.open_count);
  push('Closed faults', stats.closed_count);
  push(
    'Average resolution time (hours)',
    stats.average_resolution_time_hours == null
      ? 'N/A'
      : stats.average_resolution_time_hours.toFixed(1)
  );
  push(
    'Average rating (1-5)',
    stats.average_rating == null ? 'N/A' : stats.average_rating.toFixed(2)
  );
  push();

  push('Status', 'Count');
  for (const s of stats.by_status) push(s.status, s.count);
  push();

  push('Category', 'Count');
  for (const c of stats.by_category) push(c.category, c.count);
  push();

  push('Priority', 'Count');
  for (const p of stats.by_priority) push(p.priority, p.count);

  return rows.join('\n');
}

/** Trigger a browser download of `content` as a UTF-8 CSV file. */
export function downloadCsv(filename: string, content: string): void {
  // Prepend a BOM so Excel opens UTF-8 correctly.
  const blob = new Blob([`﻿${content}`], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
