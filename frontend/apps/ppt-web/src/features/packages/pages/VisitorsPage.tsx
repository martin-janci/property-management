/**
 * VisitorsPage
 *
 * Main page for visitor management (Epic 58, Story 58.4-58.5).
 * Epic 90, Story 90.2: Wire up check-in/out/cancel handlers to API.
 */

import {
  type ApiConfig,
  type VisitorStatus,
  useCancelVisitor,
  useCheckInVisitor,
  useCheckOutVisitor,
  useVerifyAccessCode,
  useVisitors,
} from '@ppt/api-client';
import { useCallback, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useAuth } from '../../../contexts/AuthContext';
import { VisitorCard } from '../components/VisitorCard';

// API base URL from environment
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

type FilterStatus = 'all' | 'today' | VisitorStatus;

export function VisitorsPage() {
  const navigate = useNavigate();
  const { getAccessToken } = useAuth();
  const { buildingId: urlBuildingId } = useParams<{ buildingId?: string }>();
  const [filter, setFilter] = useState<FilterStatus>('all');
  const [accessCode, setAccessCode] = useState('');
  const [verificationResult, setVerificationResult] = useState<{
    type: 'success' | 'error';
    message: string;
  } | null>(null);

  // API configuration - token is reactive via auth context
  const apiConfig: ApiConfig = useMemo(
    () => ({
      baseUrl: API_BASE_URL,
      accessToken: getAccessToken() ?? undefined,
    }),
    [getAccessToken]
  );

  // Fetch visitors from API
  const { data: visitorsData, isLoading, error } = useVisitors(apiConfig);

  // Mutations for visitor status updates
  const checkInVisitor = useCheckInVisitor(apiConfig);
  const checkOutVisitor = useCheckOutVisitor(apiConfig);
  const cancelVisitor = useCancelVisitor(apiConfig);
  const verifyAccessCodeMutation = useVerifyAccessCode(apiConfig);

  // Get visitors from API response or use empty array
  const visitors = visitorsData?.visitors ?? [];
  const filteredVisitors = visitors.filter((visitor) => {
    if (filter === 'all') return true;
    if (filter === 'today') {
      const today = new Date().toDateString();
      const arrivalDate = new Date(visitor.expectedArrival).toDateString();
      return today === arrivalDate;
    }
    return visitor.status === filter;
  });

  const handleView = useCallback(
    (id: string) => {
      navigate(`/visitors/${id}`);
    },
    [navigate]
  );

  const handleCheckIn = useCallback(
    (id: string) => {
      checkInVisitor.mutate(
        { id },
        {
          onError: (err) => {
            console.error('Failed to check in visitor:', err);
            alert('Failed to check in visitor. Please try again.');
          },
        }
      );
    },
    [checkInVisitor]
  );

  const handleCheckOut = useCallback(
    (id: string) => {
      checkOutVisitor.mutate(
        { id },
        {
          onError: (err) => {
            console.error('Failed to check out visitor:', err);
            alert('Failed to check out visitor. Please try again.');
          },
        }
      );
    },
    [checkOutVisitor]
  );

  const handleCancel = useCallback(
    (id: string) => {
      if (!window.confirm('Are you sure you want to cancel this visitor registration?')) {
        return;
      }
      cancelVisitor.mutate(id, {
        onError: (err) => {
          console.error('Failed to cancel visitor:', err);
          alert('Failed to cancel visitor registration. Please try again.');
        },
      });
    },
    [cancelVisitor]
  );

  const handleVerifyCode = useCallback(() => {
    if (!accessCode.trim()) {
      setVerificationResult({ type: 'error', message: 'Please enter an access code' });
      return;
    }
    verifyAccessCodeMutation.mutate(
      { accessCode: accessCode.trim().toUpperCase(), buildingId: urlBuildingId ?? '' },
      {
        onSuccess: (result) => {
          if (result.valid) {
            setVerificationResult({
              type: 'success',
              message: `Valid code for ${result.visitor?.visitorName ?? 'visitor'}`,
            });
          } else {
            setVerificationResult({
              type: 'error',
              message: result.message || 'Invalid access code',
            });
          }
        },
        onError: (err) => {
          setVerificationResult({
            type: 'error',
            message: err.message || 'Failed to verify code',
          });
        },
      }
    );
  }, [accessCode, verifyAccessCodeMutation]);

  // Loading state
  if (isLoading) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="text-center py-12">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto" />
          <p className="text-gray-500 mt-4">Loading visitors...</p>
        </div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="text-center py-12 bg-red-50 rounded-lg">
          <p className="text-red-600">Failed to load visitors: {error.message}</p>
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="mt-4 px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Visitor Management</h1>
          <p className="text-gray-600 mt-1">Pre-register visitors and manage access codes</p>
        </div>
        <button
          type="button"
          className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 flex items-center gap-2"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <title>Add</title>
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
          Register Visitor
        </button>
      </div>

      {/* Access Code Verification */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
        <div className="flex items-center gap-4">
          <svg
            className="w-8 h-8 text-blue-600"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <title>Verify Code</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
            />
          </svg>
          <div className="flex-1">
            <p className="text-sm font-medium text-blue-900">Verify Access Code</p>
            <p className="text-xs text-blue-700">Enter a visitor&apos;s access code to verify</p>
            {verificationResult && (
              <p
                className={`text-xs mt-1 ${
                  verificationResult.type === 'success' ? 'text-green-700' : 'text-red-700'
                }`}
              >
                {verificationResult.message}
              </p>
            )}
          </div>
          <div className="flex gap-2">
            <input
              type="text"
              placeholder="Enter code..."
              className="px-3 py-2 border border-blue-300 rounded-md text-sm uppercase font-mono"
              maxLength={8}
              value={accessCode}
              onChange={(e) => {
                setAccessCode(e.target.value);
                setVerificationResult(null);
              }}
              onKeyDown={(e) => e.key === 'Enter' && handleVerifyCode()}
            />
            <button
              type="button"
              onClick={handleVerifyCode}
              disabled={verifyAccessCodeMutation.isPending}
              className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 text-sm disabled:opacity-50"
            >
              {verifyAccessCodeMutation.isPending ? 'Verifying...' : 'Verify'}
            </button>
          </div>
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-2 mb-6">
        {(
          ['all', 'today', 'pending', 'checked_in', 'checked_out', 'expired', 'cancelled'] as const
        ).map((status) => (
          <button
            key={status}
            type="button"
            onClick={() => setFilter(status)}
            className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
              filter === status
                ? 'bg-blue-600 text-white'
                : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
            }`}
          >
            {status === 'all'
              ? 'All'
              : status === 'today'
                ? 'Today'
                : status === 'checked_in'
                  ? 'Checked In'
                  : status === 'checked_out'
                    ? 'Checked Out'
                    : status.charAt(0).toUpperCase() + status.slice(1)}
          </button>
        ))}
      </div>

      {/* Visitor List */}
      <div className="space-y-4">
        {filteredVisitors.length === 0 ? (
          <div className="text-center py-12 bg-gray-50 rounded-lg">
            <svg
              className="w-12 h-12 text-gray-400 mx-auto mb-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <title>No visitors</title>
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"
              />
            </svg>
            <p className="text-gray-500">No visitors found</p>
          </div>
        ) : (
          filteredVisitors.map((visitor) => (
            <VisitorCard
              key={visitor.id}
              visitor={visitor}
              onView={handleView}
              onCheckIn={handleCheckIn}
              onCheckOut={handleCheckOut}
              onCancel={handleCancel}
            />
          ))
        )}
      </div>
    </div>
  );
}
