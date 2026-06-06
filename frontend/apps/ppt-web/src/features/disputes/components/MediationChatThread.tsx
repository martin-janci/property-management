import { useAddMediationNote, useMediationNotes } from '@ppt/api-client';
import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Spinner } from '../../../components/Spinner';
import { formatTime } from '../utils/formatTime';

interface MediationChatThreadProps {
  disputeId: string;
  currentUserId?: string;
  currentUserName?: string;
  /** If true, show the private-note toggle (managers/mediators only) */
  canSendPrivate?: boolean;
  onError?: (title: string, message?: string) => void;
}

export function MediationChatThread({
  disputeId,
  currentUserId,
  currentUserName,
  canSendPrivate = false,
  onError,
}: MediationChatThreadProps) {
  const { t, i18n } = useTranslation();
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
      setTimeout(() => {
        bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
      }, 50);
    } catch (err) {
      onError?.(
        t('disputes.mediation.sendError'),
        err instanceof Error ? err.message : t('disputes.mediation.sendErrorDesc')
      );
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
        <Spinner size="md" />
      </div>
    );
  }

  return (
    <div className="flex flex-col">
      {/* Message list */}
      <div className="space-y-3 mb-4 max-h-96 overflow-y-auto pr-1">
        {notes.length === 0 && (
          <p className="text-sm text-gray-500 text-center py-6">
            {t('disputes.mediation.noMessages')}
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
                  <span>{formatTime(note.createdAt, i18n.language)}</span>
                  {note.isPrivate && (
                    <span className="px-1.5 py-0.5 bg-orange-100 text-orange-700 rounded text-xs">
                      {t('disputes.mediation.privateLabel')}
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
          placeholder={t('disputes.mediation.typePlaceholder')}
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
                <span className="text-gray-600">{t('disputes.mediation.privateSend')}</span>
              </label>
            )}
          </div>
          <button
            type="button"
            onClick={handleSend}
            disabled={!draft.trim() || addNote.isPending}
            className="px-4 py-1.5 bg-violet-600 text-white text-sm rounded-lg hover:bg-violet-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {addNote.isPending ? t('disputes.mediation.sending') : t('disputes.mediation.send')}
          </button>
        </div>
      </div>

      {/* Attribution */}
      {currentUserName && (
        <p className="text-xs text-gray-400 mt-1.5 text-right">
          {t('disputes.mediation.sendingAs', { name: currentUserName })}
        </p>
      )}
    </div>
  );
}
