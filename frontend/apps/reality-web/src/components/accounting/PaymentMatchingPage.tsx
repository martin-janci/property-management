import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { format } from 'date-fns';

interface BankStatement {
  id: string;
  source_filename: string;
  imported_at: string;
  account_iban: string;
}

interface BankStatementLine {
  id: string;
  statement_id: string;
  booking_date: string;
  amount: string;
  currency: string;
  counterparty_iban?: string;
  variable_symbol?: string;
  raw_ref?: string;
  match_state: 'unmatched' | 'suggested' | 'matched';
}

interface PaymentMatch {
  id: string;
  statement_line_id: string;
  invoice_id: string;
  confidence: string; // Decimal comes as string
  state: 'suggested' | 'confirmed' | 'rejected';
}

export function PaymentMatchingPage() {
  const [selectedStatementId, setSelectedStatementId] = useState<string | null>(null);
  const [selectedLineId, setSelectedLineId] = useState<string | null>(null);
  const queryClient = useQueryClient();

  const { data: statements, isLoading: statementsLoading } = useQuery({
    queryKey: ['accounting', 'statements'],
    queryFn: async () => {
      const res = await fetch('/api/v1/accounting/statements', {
          headers: { 
            'X-Tenant-ID': localStorage.getItem('current_tenant_id') || '',
            'Authorization': `Bearer ${localStorage.getItem('auth_token')}`
          }
      });
      if (!res.ok) throw new Error('Failed to fetch statements');
      return res.json() as Promise<BankStatement[]>;
    },
  });

  const { data: lines, isLoading: linesLoading } = useQuery({
    queryKey: ['accounting', 'statements', selectedStatementId, 'lines'],
    queryFn: async () => {
      const res = await fetch(`/api/v1/accounting/statements/${selectedStatementId}/lines`, {
          headers: { 
            'X-Tenant-ID': localStorage.getItem('current_tenant_id') || '',
            'Authorization': `Bearer ${localStorage.getItem('auth_token')}`
          }
      });
      if (!res.ok) throw new Error('Failed to fetch lines');
      return res.json() as Promise<BankStatementLine[]>;
    },
    enabled: !!selectedStatementId,
  });

  const { data: matches, isLoading: matchesLoading } = useQuery({
    queryKey: ['accounting', 'lines', selectedLineId, 'matches'],
    queryFn: async () => {
      const res = await fetch(`/api/v1/accounting/lines/${selectedLineId}/matches`, {
          headers: { 
            'X-Tenant-ID': localStorage.getItem('current_tenant_id') || '',
            'Authorization': `Bearer ${localStorage.getItem('auth_token')}`
          }
      });
      if (!res.ok) throw new Error('Failed to fetch matches');
      return res.json() as Promise<PaymentMatch[]>;
    },
    enabled: !!selectedLineId,
  });

  const uploadMutation = useMutation({
    mutationFn: async (file: File) => {
      const formData = new FormData();
      formData.append('file', file);
      const res = await fetch('/api/v1/accounting/statements', {
        method: 'POST',
        body: formData,
        headers: { 
          'X-Tenant-ID': localStorage.getItem('current_tenant_id') || '',
          'Authorization': `Bearer ${localStorage.getItem('auth_token')}`
        }
      });
      if (!res.ok) throw new Error('Upload failed');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounting', 'statements'] });
    },
  });

  const confirmMutation = useMutation({
    mutationFn: async (matchId: string) => {
      const res = await fetch(`/api/v1/accounting/matches/${matchId}/confirm`, {
        method: 'POST',
        headers: { 
          'X-Tenant-ID': localStorage.getItem('current_tenant_id') || '',
          'Authorization': `Bearer ${localStorage.getItem('auth_token')}`
        }
      });
      if (!res.ok) throw new Error('Confirmation failed');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounting'] });
    },
  });

  const rejectMutation = useMutation({
    mutationFn: async (matchId: string) => {
      const res = await fetch(`/api/v1/accounting/matches/${matchId}/reject`, {
        method: 'POST',
        headers: { 
          'X-Tenant-ID': localStorage.getItem('current_tenant_id') || '',
          'Authorization': `Bearer ${localStorage.getItem('auth_token')}`
        }
      });
      if (!res.ok) throw new Error('Rejection failed');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounting'] });
    },
  });

  return (
    <div className="p-8">
      <div className="flex justify-between items-center mb-8">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Payment Matching</h1>
          <p className="text-sm text-gray-500">Review and confirm matches from bank statements</p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-8">
        {/* Left: Statements (3 cols) */}
        <div className="lg:col-span-3 bg-white p-6 rounded-lg shadow border border-gray-100">
          <h2 className="text-lg font-semibold mb-4 text-gray-800">Bank Statements</h2>
          <div className="mb-6">
            <label className="block text-xs font-bold text-gray-500 mb-2 uppercase tracking-wider">Upload CSV</label>
            <div className="relative">
              <input
                type="file"
                accept=".csv"
                id="statement-upload"
                onChange={(e) => e.target.files?.[0] && uploadMutation.mutate(e.target.files[0])}
                className="hidden"
              />
              <label 
                htmlFor="statement-upload"
                className="flex items-center justify-center px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 cursor-pointer transition-colors"
              >
                Choose File
              </label>
            </div>
            {uploadMutation.isPending && <p className="text-xs text-blue-600 mt-2 animate-pulse font-medium">Processing upload...</p>}
            {uploadMutation.isError && <p className="text-xs text-red-600 mt-2 font-medium">Upload failed</p>}
          </div>
          
          <div className="space-y-2 max-h-[500px] overflow-y-auto pr-1">
            {statementsLoading ? (
              <div className="animate-pulse space-y-2">
                {[1,2,3].map(i => <div key={i} className="h-16 bg-gray-50 rounded"></div>)}
              </div>
            ) : (statements?.length === 0) ? (
              <p className="text-xs text-gray-400 italic text-center py-4">No statements uploaded yet</p>
            ) : statements?.map((s) => (
              <div
                key={s.id}
                onClick={() => {
                  setSelectedStatementId(s.id);
                  setSelectedLineId(null);
                }}
                className={`p-3 rounded-md cursor-pointer transition-all border ${
                  selectedStatementId === s.id 
                    ? 'bg-blue-50 border-blue-200 shadow-sm' 
                    : 'hover:bg-gray-50 border-transparent'
                }`}
              >
                <p className={`text-sm font-semibold truncate ${selectedStatementId === s.id ? 'text-blue-700' : 'text-gray-700'}`}>
                  {s.source_filename}
                </p>
                <div className="flex justify-between items-center mt-2">
                  <p className="text-[10px] text-gray-500">{format(new Date(s.imported_at), 'MMM d, yyyy')}</p>
                  <p className="text-[10px] font-mono text-gray-400 truncate max-w-[100px]">{s.account_iban}</p>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Middle: Lines (5 cols) */}
        <div className="lg:col-span-5 bg-white p-6 rounded-lg shadow border border-gray-100">
          <h2 className="text-lg font-semibold mb-4 text-gray-800">Transactions</h2>
          {!selectedStatementId ? (
            <div className="flex flex-col items-center justify-center py-24 border-2 border-dashed border-gray-100 rounded-lg text-gray-400">
              <svg className="w-12 h-12 mb-3 text-gray-200" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              <p className="text-sm">Select a statement to view items</p>
            </div>
          ) : linesLoading ? (
            <div className="animate-pulse space-y-3">
                {[1,2,3,4,5].map(i => <div key={i} className="h-20 bg-gray-50 rounded"></div>)}
            </div>
          ) : (
            <div className="space-y-3 max-h-[600px] overflow-y-auto pr-2 custom-scrollbar">
              {lines?.map((l) => (
                <div
                  key={l.id}
                  onClick={() => setSelectedLineId(l.id)}
                  className={`p-4 rounded-lg cursor-pointer border transition-all relative overflow-hidden ${
                    selectedLineId === l.id 
                      ? 'border-blue-400 bg-blue-50/30 shadow-md' 
                      : 'border-gray-100 hover:border-blue-200 hover:bg-gray-50/50'
                  }`}
                >
                  <div className="flex justify-between items-start relative z-10">
                    <div>
                      <p className="text-base font-bold text-gray-900 leading-none">
                        {l.amount} <span className="text-xs font-medium text-gray-500 ml-0.5">{l.currency}</span>
                      </p>
                      <p className="text-[10px] font-medium text-gray-400 mt-2 tracking-tight uppercase">{format(new Date(l.booking_date), 'MMMM d, yyyy')}</p>
                    </div>
                    <span className={`text-[10px] px-2 py-1 rounded font-bold tracking-wider uppercase ${
                      l.match_state === 'matched' ? 'bg-green-100 text-green-700' :
                      l.match_state === 'suggested' ? 'bg-yellow-100 text-yellow-700 border border-yellow-200' :
                      'bg-gray-50 text-gray-500 border border-gray-100'
                    }`}>
                      {l.match_state}
                    </span>
                  </div>
                  <div className="mt-4 pt-3 border-t border-gray-50 grid grid-cols-2 gap-4 text-[11px]">
                    <div className="flex flex-col">
                      <span className="text-gray-400 font-bold uppercase text-[9px] mb-0.5 tracking-tight">Variable Symbol</span>
                      <span className="text-gray-700 font-mono font-medium truncate">{l.variable_symbol || '—'}</span>
                    </div>
                    <div className="flex flex-col">
                      <span className="text-gray-400 font-bold uppercase text-[9px] mb-0.5 tracking-tight">Counterparty IBAN</span>
                      <span className="text-gray-700 font-mono font-medium truncate">{l.counterparty_iban || '—'}</span>
                    </div>
                  </div>
                  {selectedLineId === l.id && (
                    <div className="absolute left-0 top-0 bottom-0 w-1 bg-blue-500"></div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Right: Matches (4 cols) */}
        <div className="lg:col-span-4 bg-white p-6 rounded-lg shadow border border-gray-100">
          <h2 className="text-lg font-semibold mb-4 text-gray-800">Match Review</h2>
          {!selectedLineId ? (
            <div className="flex flex-col items-center justify-center py-24 border-2 border-dashed border-gray-100 rounded-lg text-gray-400">
              <svg className="w-12 h-12 mb-3 text-gray-200" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
              <p className="text-sm px-8 text-center">Select a transaction to see suggested invoice matches</p>
            </div>
          ) : matchesLoading ? (
             <div className="animate-pulse space-y-4">
                <div className="h-48 bg-gray-50 rounded-xl"></div>
             </div>
          ) : (
            <div className="space-y-4">
              {matches?.filter(m => m.state === 'suggested').map((m) => (
                <div key={m.id} className="border border-yellow-200 bg-gradient-to-br from-yellow-50 to-white rounded-xl p-6 shadow-sm">
                  <div className="flex justify-between items-center mb-4">
                    <div className="flex flex-col">
                      <span className="text-[10px] font-bold text-yellow-600 uppercase tracking-widest mb-1">Confidence Score</span>
                      <span className="text-2xl font-black text-yellow-700">{(parseFloat(m.confidence) * 100).toFixed(0)}%</span>
                    </div>
                    <div className="w-12 h-12 rounded-full border-4 border-yellow-100 flex items-center justify-center">
                       <div className="w-8 h-8 rounded-full bg-yellow-400 animate-pulse opacity-20"></div>
                    </div>
                  </div>
                  
                  <div className="space-y-3 mb-6 p-3 bg-white/50 rounded-lg border border-yellow-100/50">
                    <div>
                      <span className="text-[9px] font-bold text-gray-400 uppercase block mb-0.5">Invoice Reference</span>
                      <p className="text-xs font-bold text-gray-800 font-mono">{m.invoice_id}</p>
                    </div>
                    <div>
                      <span className="text-[9px] font-bold text-gray-400 uppercase block mb-0.5">Match Reason</span>
                      <p className="text-xs text-gray-600 leading-tight">Variable symbol and amount matches the open balance.</p>
                    </div>
                  </div>

                  <div className="flex flex-col gap-2">
                    <button
                      onClick={() => confirmMutation.mutate(m.id)}
                      disabled={confirmMutation.isPending}
                      className="w-full bg-blue-600 text-white text-xs font-bold py-3 rounded-lg hover:bg-blue-700 transition-all shadow-md active:scale-[0.98] disabled:bg-gray-400"
                    >
                      {confirmMutation.isPending ? 'Processing...' : 'Confirm Payment Match'}
                    </button>
                    <button
                      onClick={() => rejectMutation.mutate(m.id)}
                      disabled={rejectMutation.isPending}
                      className="w-full bg-white text-gray-500 border border-gray-200 text-xs font-bold py-2.5 rounded-lg hover:bg-gray-50 transition-colors disabled:bg-gray-100"
                    >
                      {rejectMutation.isPending ? 'Rejecting...' : 'Not a match'}
                    </button>
                  </div>
                </div>
              ))}
              
              {matches?.filter(m => m.state === 'confirmed').map((m) => (
                <div key={m.id} className="border border-green-200 bg-green-50/50 rounded-xl p-6 relative overflow-hidden">
                   <div className="flex items-center space-x-3 mb-2 relative z-10">
                    <div className="bg-green-500 rounded-full p-1">
                      <svg className="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                      </svg>
                    </div>
                    <span className="text-xs font-bold text-green-700 uppercase tracking-widest">Match Confirmed</span>
                  </div>
                  <p className="text-xs text-gray-600 font-medium relative z-10">Linked to Invoice: <span className="font-mono font-bold text-gray-800">{m.invoice_id}</span></p>
                  <div className="absolute right-[-20px] bottom-[-20px] opacity-10">
                    <svg className="w-24 h-24 text-green-600" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                    </svg>
                  </div>
                </div>
              ))}

              {matches?.filter(m => m.state === 'suggested').length === 0 && 
               matches?.filter(m => m.state === 'confirmed').length === 0 && (
                <div className="flex flex-col items-center justify-center py-12 text-gray-400 bg-gray-50/50 rounded-xl border border-gray-100">
                  <svg className="w-8 h-8 mb-2 opacity-20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <p className="italic text-xs font-medium">No active match suggestions</p>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
