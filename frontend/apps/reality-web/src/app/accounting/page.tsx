/**
 * Payment Matching Page
 *
 * Bank statement upload and payment matching review (PAP-210).
 */

'use client';

import { Suspense } from 'react';
import { Header, Footer } from '@/components/ui';
import { PaymentMatchingPage } from '@/components/accounting/PaymentMatchingPage';

function PaymentMatchingFallback() {
  return (
    <div className="min-h-screen bg-gray-50 flex flex-col">
      <Header />
      <main className="flex-1 flex items-center justify-center">
        <div className="animate-pulse w-full max-w-7xl px-8">
            <div className="h-10 bg-gray-200 rounded mb-8"></div>
            <div className="h-96 bg-gray-200 rounded"></div>
        </div>
      </main>
      <Footer />
    </div>
  );
}

export default function PaymentMatchingRoute() {
  return (
    <Suspense fallback={<PaymentMatchingFallback />}>
      <PaymentMatchingPage />
    </Suspense>
  );
}
