/**
 * MediationChatThread - chat-style thread for manager/tenant mediation notes.
 * Epic 80: Dispute Mediation (Story 80.3)
 *
 * Renders mediation notes as a chat thread with role-tinted bubbles:
 *   - manager/mediator: violet-tinted (right-aligned for current user)
 *   - tenant/other: surface-tinted (left-aligned)
 *   - private notes: orange border + "Private" badge
 *
 * Uses useMediationNotes (query) + useAddMediationNote (mutation) from @ppt/api-client.
 */

import { useAddMediationNote, useMediationNotes } from '@ppt/api-client';
import { useRef, useState } from 'react';

interface MediationChatThreadProps {
  disputeId: string;
  currentUserId?: string;
  currentUserName?: string;
  /** If true, show the private-note toggle (managers/mediators only) */
  canSendPrivate?: boolean;
}

function formatTime(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60_000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString();
}

export function MediationChatThread({
  disputeId,
  currentUserId,
  currentUserName,
  canSendPrivate = false,
}: MediationChatThreadProps) {
  const [draft, setDraft] = useState('');
  const [isPrivate, setIsPrivate] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  const { data: notes = [], isLoading } = useMediationNotes(disputeId);
  const addNote = useAddMediationNote(disputeId);

  const handleSend = async () => {
    const content = draft.trim();
    if (!content || addNote.isPending) return;

    try {
      await addNote.mutateAsync({ content, isPrivate });
      setDraft('');
      setIsPrivate(false);
      // Scroll to bottom after a tick
      setTimeout(() => {
        bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
      }, 50);
    } catch {
      // error toast is handled by the mutation error boundary in App.tsx
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      handleSend();
    }
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-violet-600" />
      </div>
    );
  }

  return (
    <div className="flex flex-col">
      {/* Message list */}
      <div className="space-y-3 mb-4 max-h-96 overflow-y-auto pr-1">
        {notes.length === 0 && (
          <p className="text-sm text-gray-500 text-center py-6">
            No messages yet. Start the conversation below.
          </p>
        )}

        {notes.map((note) => {
          const isMine = note.authorId === currentUserId;

          return (
            <div key={note.id} className={`flex ${isMine ? 'justify-end' : 'justify-start'}`}>
              <div
                className={`max-w-[75%] ${isMine ? 'items-end' : 'items-start'} flex flex-col gap-1`}
              >
                {/* Author + time */}
                <div
                  className={`flex items-center gap-2 text-xs text-gray-500 ${isMine ? 'flex-row-reverse' : ''}`}
                >
                  <span className="font-medium text-gray-700">{note.authorName}</span>
                  <span>{formatTime(note.createdAt)}</span>
                  {note.isPrivate && (
                    <span className="px-1.5 py-0.5 bg-orange-100 text-orange-700 rounded text-xs">
                      Private
                    </span>
                  )}
                </div>

                {/* Bubble */}
                <div
                  className={[
                    'px-4 py-2.5 rounded-2xl text-sm whitespace-pre-wrap break-words',
                    isMine
                      ? 'bg-violet-600 text-white rounded-tr-sm'
                      : 'bg-gray-100 text-gray-900 rounded-tl-sm',
                    note.isPrivate ? 'border-2 border-orange-300' : '',
                  ]
                    .filter(Boolean)
                    .join(' ')}
                >
                  {note.content}
                </div>
              </div>
            </div>
          );
        })}

        <div ref={bottomRef} />
      </div>

      {/* Composer */}
      <div className="border border-gray-200 rounded-xl bg-white shadow-sm">
        <textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type a message… (Ctrl+Enter to send)"
          rows={3}
          className="w-full px-4 py-3 rounded-t-xl text-sm text-gray-900 placeholder-gray-400 resize-none border-0 focus:outline-none focus:ring-0"
        />
        <div className="flex items-center justify-between px-4 py-2 border-t border-gray-100">
          <div className="flex items-center gap-3">
            {canSendPrivate && (
              <label className="flex items-center gap-1.5 text-sm cursor-pointer select-none">
                <input
                  type="checkbox"
                  checked={isPrivate}
                  onChange={(e) => setIsPrivate(e.target.checked)}
                  className="rounded border-gray-300 text-violet-600"
                />
                <span className="text-gray-600">Private (mediator only)</span>
              </label>
            )}
          </div>
          <button
            type="button"
            onClick={handleSend}
            disabled={!draft.trim() || addNote.isPending}
            className="px-4 py-1.5 bg-violet-600 text-white text-sm rounded-lg hover:bg-violet-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {addNote.isPending ? 'Sending…' : 'Send'}
          </button>
        </div>
      </div>

      {/* Attribution */}
      {currentUserName && (
        <p className="text-xs text-gray-400 mt-1.5 text-right">
          Sending as <span className="font-medium">{currentUserName}</span>
        </p>
      )}
    </div>
  );
}
