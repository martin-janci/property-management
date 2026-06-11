/** Quorum + participation tile shared by the list and detail views (Epic 5). */
import { participationPercent } from '../types';

interface QuorumTileProps {
  participation: number;
  eligible: number;
  quorumMet: boolean;
  compact?: boolean;
}

export function QuorumTile({ participation, eligible, quorumMet, compact }: QuorumTileProps) {
  const pct = participationPercent(participation, eligible);
  const barColor = quorumMet ? 'bg-green-500' : 'bg-amber-500';

  return (
    <div className={compact ? 'w-40' : 'w-44'}>
      <div className="flex items-baseline justify-between">
        <span className="text-lg font-semibold text-gray-900">
          {participation}/{eligible}
        </span>
        <span className={`text-xs font-medium ${quorumMet ? 'text-green-700' : 'text-amber-700'}`}>
          {quorumMet ? 'Quorum met' : `${pct}%`}
        </span>
      </div>
      <div className="mt-1 h-2 w-full overflow-hidden rounded-full bg-gray-200">
        <div className={`h-full ${barColor}`} style={{ width: `${pct}%` }} />
      </div>
      <p className="mt-1 text-xs text-gray-500">
        {participation} of {eligible} eligible voted
      </p>
    </div>
  );
}
