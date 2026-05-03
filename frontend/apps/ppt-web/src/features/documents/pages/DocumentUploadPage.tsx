/**
 * Document Upload Page (Story 39.2).
 *
 * Page wrapper for document upload functionality.
 */

import { useNavigate } from 'react-router-dom';
import { DocumentUpload } from '../components/DocumentUpload';

interface DocumentUploadPageProps {
  organizationId?: string;
  buildingId?: string;
}

export function DocumentUploadPage({
  organizationId = 'default-org',
  buildingId,
}: DocumentUploadPageProps) {
  const navigate = useNavigate();

  const handleUploadComplete = (documentId: string) => {
    // Navigate to the document detail page after upload
    navigate(`/documents/${documentId}`);
  };

  const handleCancel = () => {
    navigate('/documents');
  };

  return (
    <div className="upload-page">
      <div className="upload-container">
        <DocumentUpload
          organizationId={organizationId}
          buildingId={buildingId}
          onUploadComplete={handleUploadComplete}
          onCancel={handleCancel}
        />
      </div>

      <style>{`
        .upload-page {
          min-height: 100%;
          padding: 2rem;
          background: var(--ppt-bg-app);
        }

        .upload-container {
          max-width: 640px;
          margin: 0 auto;
          padding: 2rem;
          background: var(--ppt-bg-surface);
          border-radius: 0.75rem;
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }

        @media (max-width: 640px) {
          .upload-page {
            padding: 1rem;
          }

          .upload-container {
            padding: 1rem;
          }
        }
      `}</style>
    </div>
  );
}

export default DocumentUploadPage;
