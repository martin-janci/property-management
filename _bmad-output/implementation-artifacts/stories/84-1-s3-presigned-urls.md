# Story 84.1: S3 Presigned URL Implementation

Status: done

## Story

As a **system user**,
I want to **download documents securely via presigned URLs**,
So that **I can access files without exposing storage credentials**.

## Acceptance Criteria

1. **AC-1: Generate Download URLs**
   - Given a document exists in storage
   - When I request to download it
   - Then a presigned URL is generated
   - And the URL has a limited validity period
   - And the download starts automatically

2. **AC-2: URL Expiration**
   - Given a presigned URL is generated
   - When the expiration time passes
   - Then the URL no longer works
   - And access is denied with appropriate error

3. **AC-3: Access Control**
   - Given I request a document URL
   - When I don't have permission for that document
   - Then the URL generation is denied
   - And an authorization error is returned

4. **AC-4: Upload via Presigned URL**
   - Given I want to upload a file
   - When I request an upload URL
   - Then a presigned PUT URL is generated
   - And I can upload directly to S3
   - And the server is notified of completion

5. **AC-5: Multiple File Formats**
   - Given documents of various types exist
   - When downloading different file types
   - Then correct Content-Type headers are set
   - And files download with proper names
   - And Content-Disposition is set correctly

## Tasks / Subtasks

- [ ] Task 1: Implement S3 Presigned URL Service (AC: 1, 2, 5)
  - [ ] 1.1 Create `/backend/crates/integrations/storage/src/presigned.rs`
  - [ ] 1.2 Implement generate_download_url function
  - [ ] 1.3 Configure URL expiration (default: 15 minutes)
  - [ ] 1.4 Set Content-Type based on file extension
  - [ ] 1.5 Set Content-Disposition for downloads

- [ ] Task 2: Update Documents Route (AC: 1, 3)
  - [ ] 2.1 Update `/backend/servers/api-server/src/routes/documents.rs:1058`
  - [ ] 2.2 Verify document access permissions
  - [ ] 2.3 Generate presigned URL via storage service
  - [ ] 2.4 Return URL and expiration to client

- [ ] Task 3: Implement Upload Presigned URLs (AC: 4)
  - [ ] 3.1 Update `/backend/servers/api-server/src/routes/documents.rs:1116`
  - [ ] 3.2 Create upload request endpoint
  - [ ] 3.3 Generate presigned PUT URL
  - [ ] 3.4 Create pending upload record
  - [ ] 3.5 Implement upload completion callback

- [ ] Task 4: Configure S3 CORS (AC: 4)
  - [ ] 4.1 Add CORS configuration for direct upload
  - [ ] 4.2 Allow PUT from frontend origins
  - [ ] 4.3 Configure allowed headers

- [ ] Task 5: Update Frontend Download Logic (AC: 1, 5)
  - [ ] 5.1 Update document download to use presigned URLs
  - [ ] 5.2 Handle URL expiration and retry
  - [ ] 5.3 Show download progress
  - [ ] 5.4 Handle download errors

## Dev Notes

### Architecture Requirements
- Never expose S3 credentials to clients
- Short-lived URLs for security
- Direct upload to reduce server load
- Support all common document types

### Technical Specifications
- URL expiration: 15 minutes for download, 5 minutes for upload
- Supported formats: PDF, DOCX, XLSX, PNG, JPG, etc.
- Maximum file size: 50MB
- Bucket: Configured via environment variable

### Existing TODO References
```rust
// backend/servers/api-server/src/routes/documents.rs:1058
// TODO: Generate presigned URL for download
// - Use S3 storage service
// - Set appropriate expiration
// - Include Content-Disposition

// backend/servers/api-server/src/routes/documents.rs:1116
// TODO: Implement presigned upload URL
// - Generate PUT URL
// - Track pending uploads
// - Handle completion callback
```

### Presigned URL Service
```rust
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

pub struct StorageService {
    client: aws_sdk_s3::Client,
    bucket: String,
}

impl StorageService {
    pub async fn generate_download_url(
        &self,
        key: &str,
        filename: &str,
        content_type: &str,
        expires_in: Duration,
    ) -> Result<PresignedUrl, StorageError> {
        let config = PresigningConfig::expires_in(expires_in)?;

        let presigned = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .response_content_type(content_type)
            .response_content_disposition(format!(
                "attachment; filename=\"{}\"",
                filename
            ))
            .presigned(config)
            .await?;

        Ok(PresignedUrl {
            url: presigned.uri().to_string(),
            expires_at: Utc::now() + expires_in,
        })
    }

    pub async fn generate_upload_url(
        &self,
        key: &str,
        content_type: &str,
        expires_in: Duration,
    ) -> Result<PresignedUrl, StorageError> {
        let config = PresigningConfig::expires_in(expires_in)?;

        let presigned = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(config)
            .await?;

        Ok(PresignedUrl {
            url: presigned.uri().to_string(),
            expires_at: Utc::now() + expires_in,
        })
    }
}
```

### API Response
```rust
#[derive(Serialize)]
pub struct DownloadUrlResponse {
    pub url: String,
    pub expires_at: DateTime<Utc>,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
}

#[derive(Serialize)]
pub struct UploadUrlResponse {
    pub url: String,
    pub key: String,
    pub expires_at: DateTime<Utc>,
    pub callback_token: String, // For upload completion
}
```

### Content Type Mapping
```rust
fn get_content_type(filename: &str) -> &'static str {
    let extension = filename.rsplit('.').next().unwrap_or("");
    match extension.to_lowercase().as_str() {
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "txt" => "text/plain",
        "csv" => "text/csv",
        _ => "application/octet-stream",
    }
}
```

### File List (to create/modify)

**Create:**
- `/backend/crates/integrations/storage/src/presigned.rs` - Presigned URL generation

**Modify:**
- `/backend/servers/api-server/src/routes/documents.rs` - Download/upload handlers
- `/backend/crates/integrations/storage/src/lib.rs` - Export presigned module
- `/frontend/packages/api-client/src/documents/api.ts` - Use presigned URLs

### Frontend Integration
```typescript
async function downloadDocument(documentId: string): Promise<void> {
  // 1. Get presigned URL from API
  const { url, filename } = await documentsApi.getDownloadUrl(documentId);

  // 2. Create temporary link and click
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

async function uploadDocument(file: File, documentId: string): Promise<void> {
  // 1. Get presigned upload URL
  const { url, key, callbackToken } = await documentsApi.getUploadUrl({
    filename: file.name,
    contentType: file.type,
    size: file.size,
  });

  // 2. Upload directly to S3
  await fetch(url, {
    method: 'PUT',
    body: file,
    headers: {
      'Content-Type': file.type,
    },
  });

  // 3. Notify backend of completion
  await documentsApi.confirmUpload(key, callbackToken);
}
```

### Dependencies
- None (foundational infrastructure story)

### References
- [Source: backend/servers/api-server/src/routes/documents.rs:1058,1116]
- [UC-10: Document Management]
- [AWS S3 Presigned URLs Documentation]
