/**
 * AiChatPage - AI Assistant chat page
 * Epic 127: AI Chatbot Interface
 */

import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { useAuth } from '../../../contexts';
import { isManagerRole } from '../../../routes/shared';
import { ChatInterface } from '../components';

export function AiChatPage() {
  const { t } = useTranslation();
  const { user } = useAuth();
  // The AI insight dashboards are org-wide operator views — managers only.
  const showInsights = isManagerRole(user?.role);

  return (
    <div className="container mx-auto px-4 py-6 h-[calc(100vh-120px)] flex flex-col">
      {showInsights && (
        <nav aria-label={t('aiDashboards.nav.label')} className="mb-3 flex flex-wrap gap-4 text-sm">
          <Link to="/ai/sentiment" className="font-medium text-blue-700 hover:text-blue-900">
            {t('aiDashboards.nav.sentiment')}
          </Link>
          <Link
            to="/ai/predictive-maintenance"
            className="font-medium text-blue-700 hover:text-blue-900"
          >
            {t('aiDashboards.nav.predictiveMaintenance')}
          </Link>
        </nav>
      )}
      <div className="flex-1 min-h-0">
        <ChatInterface showSidebar={true} />
      </div>
    </div>
  );
}
