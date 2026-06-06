/**
 * MediationPage - page for managing mediation sessions.
 * Epic 77: Dispute Resolution (Story 77.2)
 */

import { useState } from 'react';
import { Spinner } from '../../../components/Spinner';
import {
  type MediationSession,
  MediationSessionCard,
  type SessionAttendee,
  type SessionType,
  sessionTypeLabels,
} from '../components/MediationSessionCard';
import type { DisputeDetail, DisputeParty } from './DisputeDetailPage';

interface Submission {
  id: string;
  partyId: string;
  partyName: string;
  submissionType: string;
  content: string;
  isVisibleToAll: boolean;
  createdAt: string;
}

interface MediationPageProps {
  dispute: DisputeDetail;
  parties: DisputeParty[];
  sessions: Array<MediationSession & { attendees: SessionAttendee[] }>;
  submissions: Submission[];
  isMediator?: boolean;
  isParty?: boolean;
  currentUserId?: string;
  isLoading?: boolean;
  onBack: () => void;
  onScheduleSession: (data: {
    sessionType: SessionType;
    scheduledAt: string;
    durationMinutes?: number;
    location?: string;
    meetingUrl?: string;
    attendeePartyIds: string[];
  }) => void;
  onCancelSession: (sessionId: string) => void;
  onCompleteSession: (sessionId: string, notes: string, outcome?: string) => void;
  onConfirmAttendance: (sessionId: string, partyId: string) => void;
  onRecordAttendance: (sessionId: string, partyId: string, attended: boolean) => void;
  onSubmitResponse: (submissionType: string, content: string, isVisibleToAll: boolean) => void;
}

export function MediationPage({
  dispute,
  parties,
  sessions,
  submissions,
  isMediator = false,
  isParty = false,
  currentUserId: _currentUserId,
  isLoading,
  onBack,
  onScheduleSession,
  onCancelSession,
  onCompleteSession,
  onConfirmAttendance,
  onRecordAttendance: _onRecordAttendance,
  onSubmitResponse,
}: MediationPageProps) {
  const [showScheduleDialog, setShowScheduleDialog] = useState(false);
  const [showCompleteDialog, setShowCompleteDialog] = useState(false);
  const [showSubmitDialog, setShowSubmitDialog] = useState(false);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);

  // Schedule form state
  const [sessionType, setSessionType] = useState<SessionType>('video_call');
  const [scheduledAt, setScheduledAt] = useState('');
  const [durationMinutes, setDurationMinutes] = useState('60');
  const [location, setLocation] = useState('');
  const [meetingUrl, setMeetingUrl] = useState('');
  const [selectedAttendees, setSelectedAttendees] = useState<string[]>([]);

  // Complete form state
  const [sessionNotes, setSessionNotes] = useState('');
  const [sessionOutcome, setSessionOutcome] = useState('');

  // Submit response state
  const [submissionType, setSubmissionType] = useState('statement');
  const [submissionContent, setSubmissionContent] = useState('');
  const [isVisibleToAll, setIsVisibleToAll] = useState(true);

  const upcomingSessions = sessions.filter((s) => s.status === 'scheduled');
  const pastSessions = sessions.filter((s) => ['completed', 'cancelled'].includes(s.status));

  const handleScheduleSession = () => {
    if (scheduledAt && selectedAttendees.length > 0) {
      onScheduleSession({
        sessionType,
        scheduledAt,
        durationMinutes: Number.parseInt(durationMinutes) || undefined,
        location: location || undefined,
        meetingUrl: meetingUrl || undefined,
        attendeePartyIds: selectedAttendees,
      });
      resetScheduleForm();
      setShowScheduleDialog(false);
    }
  };

  const resetScheduleForm = () => {
    setSessionType('video_call');
    setScheduledAt('');
    setDurationMinutes('60');
    setLocation('');
    setMeetingUrl('');
    setSelectedAttendees([]);
  };

  const handleCompleteSession = () => {
    if (selectedSessionId && sessionNotes) {
      onCompleteSession(selectedSessionId, sessionNotes, sessionOutcome || undefined);
      setSessionNotes('');
      setSessionOutcome('');
      setSelectedSessionId(null);
      setShowCompleteDialog(false);
    }
  };

  const handleSubmitResponse = () => {
    if (submissionContent.trim()) {
      onSubmitResponse(submissionType, submissionContent, isVisibleToAll);
      setSubmissionContent('');
      setShowSubmitDialog(false);
    }
  };

  const handleAttendeeToggle = (partyId: string) => {
    setSelectedAttendees((prev) =>
      prev.includes(partyId) ? prev.filter((p) => p !== partyId) : [...prev, partyId]
    );
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-12">
        <Spinner size="lg" color="blue" />
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onBack}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          Back to Dispute
        </button>
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">Mediation</h1>
            <p className="text-gray-500 mt-1">
              {dispute.referenceNumber} - {dispute.title}
            </p>
          </div>
          {isMediator && (
            <button
              type="button"
              onClick={() => setShowScheduleDialog(true)}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
            >
              Schedule Session
            </button>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Main Content */}
        <div className="lg:col-span-2 space-y-6">
          {/* Upcoming Sessions */}
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              Upcoming Sessions ({upcomingSessions.length})
            </h2>
            {upcomingSessions.length === 0 ? (
              <p className="text-gray-500">No upcoming sessions scheduled.</p>
            ) : (
              <div className="space-y-4">
                {upcomingSessions.map((session) => (
                  <MediationSessionCard
                    key={session.id}
                    session={session}
                    attendees={session.attendees}
                    isMediator={isMediator}
                    onCancel={onCancelSession}
                    onComplete={(id) => {
                      setSelectedSessionId(id);
                      setShowCompleteDialog(true);
                    }}
                    onConfirmAttendance={onConfirmAttendance}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Submissions */}
          <div className="bg-white rounded-lg shadow p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">
                Party Submissions ({submissions.length})
              </h2>
              {isParty && (
                <button
                  type="button"
                  onClick={() => setShowSubmitDialog(true)}
                  className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                >
                  Submit Response
                </button>
              )}
            </div>
            {submissions.length === 0 ? (
              <p className="text-gray-500">No submissions yet.</p>
            ) : (
              <div className="space-y-4">
                {submissions.map((submission) => (
                  <div
                    key={submission.id}
                    className="p-4 bg-gray-50 rounded-lg border border-gray-200"
                  >
                    <div className="flex items-center justify-between mb-2">
                      <div>
                        <span className="font-medium">{submission.partyName}</span>
                        <span className="text-sm text-gray-500 ml-2">
                          ({submission.submissionType})
                        </span>
                      </div>
                      <span className="text-sm text-gray-500">
                        {new Date(submission.createdAt).toLocaleDateString()}
                      </span>
                    </div>
                    <p className="text-gray-700 whitespace-pre-wrap">{submission.content}</p>
                    {!submission.isVisibleToAll && (
                      <p className="text-xs text-orange-600 mt-2">
                        (Private - visible to mediator only)
                      </p>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Past Sessions */}
          {pastSessions.length > 0 && (
            <div className="bg-white rounded-lg shadow p-6">
              <h2 className="text-lg font-semibold text-gray-900 mb-4">
                Past Sessions ({pastSessions.length})
              </h2>
              <div className="space-y-4">
                {pastSessions.map((session) => (
                  <MediationSessionCard
                    key={session.id}
                    session={session}
                    attendees={session.attendees}
                    isMediator={isMediator}
                  />
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Parties */}
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">Parties</h3>
            <div className="space-y-3">
              {parties.map((party) => (
                <div key={party.id} className="p-3 bg-gray-50 rounded-lg">
                  <div className="flex items-center justify-between">
                    <span className="font-medium">{party.userName}</span>
                    <span className="text-xs px-2 py-0.5 bg-gray-200 rounded capitalize">
                      {party.role}
                    </span>
                  </div>
                  <p className="text-sm text-gray-500">{party.userEmail}</p>
                </div>
              ))}
            </div>
          </div>

          {/* Guidelines */}
          <div className="bg-blue-50 rounded-lg p-6 border border-blue-100">
            <h3 className="font-semibold text-blue-900 mb-2">Mediation Guidelines</h3>
            <ul className="text-sm text-blue-800 space-y-2">
              <li>Be respectful and listen to all parties</li>
              <li>Focus on interests, not positions</li>
              <li>All discussions are confidential</li>
              <li>Work towards a mutually acceptable solution</li>
            </ul>
          </div>
        </div>
      </div>

      {/* Schedule Session Dialog */}
      {showScheduleDialog && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <button
            type="button"
            className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
            onClick={() => setShowScheduleDialog(false)}
            onKeyDown={(e) => e.key === 'Escape' && setShowScheduleDialog(false)}
            aria-label="Close dialog"
          />
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="relative w-full max-w-lg bg-white rounded-lg shadow-xl p-6">
              <h2 className="text-lg font-semibold mb-4">Schedule Mediation Session</h2>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium mb-1">Session Type</label>
                  <select
                    value={sessionType}
                    onChange={(e) => setSessionType(e.target.value as SessionType)}
                    className="w-full rounded-md border border-gray-300 px-3 py-2"
                  >
                    {Object.entries(sessionTypeLabels).map(([value, label]) => (
                      <option key={value} value={value}>
                        {label}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium mb-1">Date & Time</label>
                    <input
                      type="datetime-local"
                      value={scheduledAt}
                      onChange={(e) => setScheduledAt(e.target.value)}
                      className="w-full rounded-md border border-gray-300 px-3 py-2"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium mb-1">Duration (minutes)</label>
                    <input
                      type="number"
                      value={durationMinutes}
                      onChange={(e) => setDurationMinutes(e.target.value)}
                      className="w-full rounded-md border border-gray-300 px-3 py-2"
                    />
                  </div>
                </div>
                {sessionType === 'in_person' && (
                  <div>
                    <label className="block text-sm font-medium mb-1">Location</label>
                    <input
                      type="text"
                      value={location}
                      onChange={(e) => setLocation(e.target.value)}
                      placeholder="Meeting room, address..."
                      className="w-full rounded-md border border-gray-300 px-3 py-2"
                    />
                  </div>
                )}
                {sessionType === 'video_call' && (
                  <div>
                    <label className="block text-sm font-medium mb-1">Meeting URL</label>
                    <input
                      type="url"
                      value={meetingUrl}
                      onChange={(e) => setMeetingUrl(e.target.value)}
                      placeholder="https://..."
                      className="w-full rounded-md border border-gray-300 px-3 py-2"
                    />
                  </div>
                )}
                <div>
                  <label className="block text-sm font-medium mb-1">Invite Parties</label>
                  <div className="border border-gray-300 rounded-md p-3 max-h-32 overflow-y-auto space-y-2">
                    {parties.map((party) => (
                      <label key={party.id} className="flex items-center gap-2 cursor-pointer">
                        <input
                          type="checkbox"
                          checked={selectedAttendees.includes(party.id)}
                          onChange={() => handleAttendeeToggle(party.id)}
                          className="rounded border-gray-300"
                        />
                        <span className="text-sm">
                          {party.userName} ({party.role})
                        </span>
                      </label>
                    ))}
                  </div>
                </div>
              </div>
              <div className="flex justify-end gap-3 mt-6">
                <button
                  type="button"
                  onClick={() => setShowScheduleDialog(false)}
                  className="px-4 py-2 border rounded-lg"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handleScheduleSession}
                  disabled={!scheduledAt || selectedAttendees.length === 0}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg disabled:opacity-50"
                >
                  Schedule
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Complete Session Dialog */}
      {showCompleteDialog && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <button
            type="button"
            className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
            onClick={() => setShowCompleteDialog(false)}
            onKeyDown={(e) => e.key === 'Escape' && setShowCompleteDialog(false)}
            aria-label="Close dialog"
          />
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="relative w-full max-w-md bg-white rounded-lg shadow-xl p-6">
              <h2 className="text-lg font-semibold mb-4">Complete Session</h2>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium mb-1">Session Notes *</label>
                  <textarea
                    value={sessionNotes}
                    onChange={(e) => setSessionNotes(e.target.value)}
                    rows={4}
                    placeholder="Summary of discussion..."
                    className="w-full rounded-md border border-gray-300 px-3 py-2"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium mb-1">Outcome (optional)</label>
                  <textarea
                    value={sessionOutcome}
                    onChange={(e) => setSessionOutcome(e.target.value)}
                    rows={2}
                    placeholder="Any agreements or next steps..."
                    className="w-full rounded-md border border-gray-300 px-3 py-2"
                  />
                </div>
              </div>
              <div className="flex justify-end gap-3 mt-4">
                <button
                  type="button"
                  onClick={() => setShowCompleteDialog(false)}
                  className="px-4 py-2 border rounded-lg"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handleCompleteSession}
                  disabled={!sessionNotes.trim()}
                  className="px-4 py-2 bg-green-600 text-white rounded-lg disabled:opacity-50"
                >
                  Complete
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Submit Response Dialog */}
      {showSubmitDialog && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <button
            type="button"
            className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
            onClick={() => setShowSubmitDialog(false)}
            onKeyDown={(e) => e.key === 'Escape' && setShowSubmitDialog(false)}
            aria-label="Close dialog"
          />
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="relative w-full max-w-md bg-white rounded-lg shadow-xl p-6">
              <h2 className="text-lg font-semibold mb-4">Submit Response</h2>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium mb-1">Type</label>
                  <select
                    value={submissionType}
                    onChange={(e) => setSubmissionType(e.target.value)}
                    className="w-full rounded-md border border-gray-300 px-3 py-2"
                  >
                    <option value="statement">Statement</option>
                    <option value="response">Response to Claims</option>
                    <option value="proposal">Settlement Proposal</option>
                    <option value="clarification">Clarification</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium mb-1">Content</label>
                  <textarea
                    value={submissionContent}
                    onChange={(e) => setSubmissionContent(e.target.value)}
                    rows={6}
                    placeholder="Your submission..."
                    className="w-full rounded-md border border-gray-300 px-3 py-2"
                  />
                </div>
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={isVisibleToAll}
                    onChange={(e) => setIsVisibleToAll(e.target.checked)}
                    className="rounded border-gray-300"
                  />
                  <span className="text-sm text-gray-700">
                    Visible to all parties (uncheck for mediator only)
                  </span>
                </label>
              </div>
              <div className="flex justify-end gap-3 mt-4">
                <button
                  type="button"
                  onClick={() => setShowSubmitDialog(false)}
                  className="px-4 py-2 border rounded-lg"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handleSubmitResponse}
                  disabled={!submissionContent.trim()}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg disabled:opacity-50"
                >
                  Submit
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
