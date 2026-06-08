/** Per-question results bars for the vote-detail / results view (Epic 5). */
import type { QuestionResult } from '@ppt/api-client';

export function QuestionResults({ result }: { result: QuestionResult }) {
  return (
    <div className="rounded-lg border border-gray-200 p-4">
      <h4 className="text-sm font-semibold text-gray-900">{result.question_text}</h4>
      <p className="mb-3 mt-0.5 text-xs text-gray-500">
        {result.total_responses} response{result.total_responses === 1 ? '' : 's'}
      </p>
      <div className="space-y-3">
        {result.options.map((opt) => {
          const isWinner = result.winner === opt.option_id;
          return (
            <div key={opt.option_id}>
              <div className="flex justify-between text-sm">
                <span className={isWinner ? 'font-semibold text-gray-900' : 'text-gray-700'}>
                  {opt.option_text}
                  {isWinner ? <span className="ml-1 text-green-600">✓</span> : null}
                </span>
                <span className="text-gray-500">
                  {opt.count} · {Math.round(opt.percentage)}%
                </span>
              </div>
              <div className="mt-1 h-2 w-full overflow-hidden rounded-full bg-gray-200">
                <div
                  className={`h-full ${isWinner ? 'bg-green-500' : 'bg-blue-400'}`}
                  style={{ width: `${Math.min(100, Math.round(opt.percentage))}%` }}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
