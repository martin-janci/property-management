/**
 * FaultBreakdownChart — accessible horizontal bar chart for fault statistic
 * breakdowns (by status / category / priority).
 *
 * Story 4.7 (BIT-186). Each bucket is rendered as a labelled, proportional bar.
 * When `onSelect` is provided the bars become buttons that drill down into the
 * faults list filtered by that bucket. Dependency-free (Tailwind + SVG-less
 * div bars) to match the existing reports charts which avoid a charting lib.
 */

export interface FaultBreakdownDatum {
  /** Machine value forwarded to `onSelect` (e.g. the raw status/category key). */
  key: string;
  /** Human-readable label shown next to the bar. */
  label: string;
  count: number;
}

interface FaultBreakdownChartProps {
  title: string;
  data: FaultBreakdownDatum[];
  /** Tailwind background class for the bars (e.g. `bg-blue-500`). */
  barClassName?: string;
  /** Drill-down handler; when set, bars are keyboard-focusable buttons. */
  onSelect?: (key: string) => void;
  /** Localised "no data" message. */
  emptyLabel?: string;
}

export function FaultBreakdownChart({
  title,
  data,
  barClassName = 'bg-blue-500',
  onSelect,
  emptyLabel = 'No data available',
}: FaultBreakdownChartProps) {
  const total = data.reduce((sum, d) => sum + d.count, 0);
  const max = data.reduce((m, d) => (d.count > m ? d.count : m), 0);

  return (
    <section className="bg-white rounded-lg shadow p-6" aria-label={title}>
      <h3 className="text-lg font-medium text-gray-900 mb-4">{title}</h3>

      {data.length === 0 || total === 0 ? (
        <p className="text-sm text-gray-500 py-4">{emptyLabel}</p>
      ) : (
        <ul className="space-y-3">
          {data.map((d) => {
            const widthPct = max > 0 ? Math.max((d.count / max) * 100, 2) : 0;
            const sharePct = total > 0 ? (d.count / total) * 100 : 0;
            const rowLabel = `${d.label}: ${d.count} (${sharePct.toFixed(0)}%)`;

            const bar = (
              <div className="flex items-center gap-3">
                <span className="w-32 shrink-0 text-sm text-gray-700 truncate" title={d.label}>
                  {d.label}
                </span>
                <span className="flex-1 h-5 bg-gray-100 rounded overflow-hidden">
                  <span
                    className={`block h-full ${barClassName} transition-all duration-300`}
                    style={{ width: `${widthPct}%` }}
                  />
                </span>
                <span className="w-20 shrink-0 text-right text-sm tabular-nums text-gray-900">
                  {d.count}
                  <span className="text-gray-400"> ({sharePct.toFixed(0)}%)</span>
                </span>
              </div>
            );

            return (
              <li key={d.key}>
                {onSelect ? (
                  <button
                    type="button"
                    onClick={() => onSelect(d.key)}
                    aria-label={`${rowLabel}. View these faults`}
                    className="w-full text-left rounded px-1 py-0.5 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  >
                    {bar}
                  </button>
                ) : (
                  <div aria-label={rowLabel}>{bar}</div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
