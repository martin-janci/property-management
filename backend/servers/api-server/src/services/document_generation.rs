//! Document Generation Service (Epic 92: Intelligent Document Generation).
//!
//! Provides LLM-powered document generation capabilities:
//! - Lease agreement generation (Story 92.1)
//! - Listing description generation (Story 92.2)
//! - Document summarization (Story 92.3)
//! - Announcement draft generation (Story 92.4)

use integrations::{
    ChatCompletionRequest, ChatMessage, LeaseGenerationInput, LeaseGenerationResult,
    ListingDescriptionInput, LlmClient, LlmError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Document generation service errors.
#[derive(Error, Debug)]
pub enum DocumentGenerationError {
    #[error("LLM error: {0}")]
    LlmError(#[from] LlmError),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(Uuid),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Generation timeout: operation exceeded {0} seconds")]
    Timeout(u64),

    #[error("Content extraction failed: {0}")]
    ContentExtractionFailed(String),
}

/// Lease generation request for Story 92.1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseGenerationRequest {
    pub unit_id: Uuid,
    pub landlord_name: String,
    pub landlord_address: Option<String>,
    pub tenant_name: String,
    pub tenant_email: String,
    pub tenant_phone: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub monthly_rent: f64,
    pub security_deposit: f64,
    pub currency: String,
    pub additional_terms: Option<Vec<String>>,
    pub include_pet_clause: bool,
    pub include_parking: bool,
    /// Jurisdiction for legal requirements (SK = Slovakia, CZ = Czech Republic)
    pub jurisdiction: Option<String>,
    /// Language for the document (sk, cs, de, en)
    pub language: String,
    /// Custom template ID to use
    pub template_id: Option<Uuid>,
}

/// Listing description generation request for Story 92.2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingDescriptionGenerationRequest {
    pub listing_id: Uuid,
    pub property_type: String,
    pub transaction_type: String,
    pub size_sqm: Option<f64>,
    pub rooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub floor: Option<i32>,
    pub total_floors: Option<i32>,
    pub features: Vec<String>,
    pub city: String,
    pub district: Option<String>,
    pub nearby_amenities: Option<Vec<String>>,
    pub price: f64,
    pub currency: String,
    /// Languages to generate descriptions for (sk, cs, de, en)
    pub languages: Vec<String>,
    /// Style of description (professional, casual, luxury, etc.)
    pub style: Option<String>,
    /// Maximum word length
    pub max_length: Option<i32>,
}

/// Multi-language listing description result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLanguageDescriptionResult {
    pub listing_id: Uuid,
    pub descriptions: Vec<LanguageDescription>,
    pub total_tokens_used: i32,
    pub generation_time_ms: u64,
}

/// Single language description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDescription {
    pub language: String,
    pub description: String,
    pub key_highlights: Vec<String>,
    pub suggested_title: Option<String>,
    pub seo_keywords: Option<Vec<String>>,
    pub tokens_used: i32,
}

/// Document summarization request for Story 92.3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSummarizationRequest {
    pub document_id: Uuid,
    /// Document content (extracted text)
    pub content: String,
    /// Original file type (pdf, docx, txt)
    pub file_type: String,
    /// Target summary length (short, medium, long)
    pub summary_length: Option<String>,
    /// Language for the summary
    pub language: Option<String>,
    /// Whether to extract key points
    pub extract_key_points: bool,
}

/// Document summarization result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSummarizationResult {
    pub document_id: Uuid,
    pub summary: String,
    pub key_points: Vec<String>,
    pub word_count: usize,
    pub tokens_used: i32,
    pub processing_time_ms: u64,
}

/// Announcement draft generation request for Story 92.4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnouncementDraftRequest {
    pub organization_id: Uuid,
    pub topic: String,
    pub key_points: Vec<String>,
    /// Urgency level (low, medium, high, critical)
    pub urgency: Option<String>,
    /// Target audience description
    pub audience: Option<String>,
    /// Tone (formal, friendly, urgent, informative)
    pub tone: Option<String>,
    /// Language (sk, cs, de, en)
    pub language: String,
    /// Number of draft variants to generate
    pub num_drafts: Option<i32>,
    /// Building or unit context for personalization
    pub context: Option<AnnouncementContext>,
}

/// Context for announcement personalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnouncementContext {
    pub building_name: Option<String>,
    pub building_address: Option<String>,
    pub management_contact: Option<String>,
}

/// Announcement draft result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnouncementDraftResult {
    pub drafts: Vec<AnnouncementDraft>,
    pub tokens_used: i32,
    pub generation_time_ms: u64,
}

/// Single announcement draft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnouncementDraft {
    pub title: String,
    pub content: String,
    pub suggested_target: String,
    pub tone_analysis: String,
}

/// Document generation service.
#[derive(Clone)]
pub struct DocumentGenerationService {
    llm_client: LlmClient,
    default_provider: String,
    default_model: String,
}

impl DocumentGenerationService {
    /// Create a new document generation service.
    pub fn new(llm_client: LlmClient) -> Self {
        let default_provider =
            std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
        let default_model =
            std::env::var("LLM_MODEL").unwrap_or_else(|_| match default_provider.as_str() {
                "openai" => "gpt-4o".to_string(),
                "azure_openai" => "gpt-4o".to_string(),
                _ => "claude-3-5-sonnet-20241022".to_string(),
            });

        Self {
            llm_client,
            default_provider,
            default_model,
        }
    }

    // =========================================================================
    // Story 92.1: Lease Document Generation
    // =========================================================================

    /// Generate a lease agreement document.
    ///
    /// Generates legally appropriate lease content based on Slovak/Czech law templates,
    /// with property details populated from unit data.
    pub async fn generate_lease(
        &self,
        request: LeaseGenerationRequest,
    ) -> Result<LeaseGenerationResult, DocumentGenerationError> {
        // Validate input
        if request.landlord_name.is_empty() {
            return Err(DocumentGenerationError::InvalidInput(
                "Landlord name is required".to_string(),
            ));
        }
        if request.tenant_name.is_empty() {
            return Err(DocumentGenerationError::InvalidInput(
                "Tenant name is required".to_string(),
            ));
        }
        if request.monthly_rent <= 0.0 {
            return Err(DocumentGenerationError::InvalidInput(
                "Monthly rent must be positive".to_string(),
            ));
        }

        let jurisdiction = request.jurisdiction.as_deref().unwrap_or("SK");
        let system_prompt = self.get_lease_system_prompt(jurisdiction, &request.language);

        let lease_input = LeaseGenerationInput {
            unit_id: request.unit_id,
            landlord_name: request.landlord_name,
            landlord_address: request.landlord_address,
            tenant_name: request.tenant_name,
            tenant_email: request.tenant_email,
            tenant_phone: request.tenant_phone,
            start_date: request.start_date,
            end_date: request.end_date,
            monthly_rent: request.monthly_rent,
            security_deposit: request.security_deposit,
            currency: request.currency,
            additional_terms: request.additional_terms,
            include_pet_clause: request.include_pet_clause,
            include_parking: request.include_parking,
            jurisdiction: Some(jurisdiction.to_string()),
        };

        self.llm_client
            .generate_lease(
                &self.default_provider,
                &self.default_model,
                &system_prompt,
                &lease_input,
                &request.language,
            )
            .await
            .map_err(DocumentGenerationError::from)
    }

    /// Get the system prompt for lease generation based on jurisdiction.
    fn get_lease_system_prompt(&self, jurisdiction: &str, language: &str) -> String {
        let legal_framework = match jurisdiction {
            "SK" => "Slovak Civil Code (Obciansky zakonnik) and Act No. 116/1990 Coll. on Rental and Sub-rental of Non-residential Premises",
            "CZ" => "Czech Civil Code (Obcansky zakonik) No. 89/2012 Coll., specifically sections 2235-2301",
            "DE" => "German Civil Code (BGB), Mietrecht (sections 535-580a)",
            _ => "applicable local tenancy laws",
        };

        let language_name = match language {
            "sk" => "Slovak",
            "cs" => "Czech",
            "de" => "German",
            _ => "English",
        };

        format!(
            r#"You are an expert legal document assistant specializing in residential lease agreements for Central European jurisdictions.

Your task is to generate a comprehensive lease agreement that complies with {}.

Requirements:
1. Generate the document in {} language
2. Include all mandatory clauses required by law
3. Use clear, professional legal language appropriate for the jurisdiction
4. Include proper formatting with numbered sections
5. Add placeholders for signatures and dates
6. Include any required notices or disclosures

The generated lease should be ready for review and signing, with proper legal structure and comprehensive terms covering:
- Parties and property identification
- Lease term and renewal conditions
- Rent amount, payment terms, and late fees
- Security deposit terms and conditions for return
- Maintenance responsibilities
- Rules for property use
- Termination conditions
- Dispute resolution procedures

Respond with well-structured content that can be converted to a professional document."#,
            legal_framework, language_name
        )
    }

    // =========================================================================
    // Story 92.2: Listing Description Generation
    // =========================================================================

    /// Generate property listing descriptions in multiple languages.
    ///
    /// Generates compelling descriptions highlighting property features and location benefits.
    pub async fn generate_listing_descriptions(
        &self,
        request: ListingDescriptionGenerationRequest,
    ) -> Result<MultiLanguageDescriptionResult, DocumentGenerationError> {
        use std::time::Instant;

        let start_time = Instant::now();

        if request.languages.is_empty() {
            return Err(DocumentGenerationError::InvalidInput(
                "At least one language must be specified".to_string(),
            ));
        }

        let system_prompt = self.get_listing_system_prompt(request.style.as_deref());

        let mut descriptions = Vec::new();
        let mut total_tokens = 0;

        for language in &request.languages {
            let input = ListingDescriptionInput {
                property_type: request.property_type.clone(),
                transaction_type: request.transaction_type.clone(),
                size_sqm: request.size_sqm,
                rooms: request.rooms,
                bathrooms: request.bathrooms,
                floor: request.floor,
                total_floors: request.total_floors,
                features: request.features.clone(),
                city: request.city.clone(),
                district: request.district.clone(),
                nearby_amenities: request.nearby_amenities.clone(),
                price: request.price,
                currency: request.currency.clone(),
                language: language.clone(),
                style: request.style.clone(),
                max_length: request.max_length,
            };

            let result = self
                .llm_client
                .generate_listing_description(
                    &self.default_provider,
                    &self.default_model,
                    &system_prompt,
                    &input,
                )
                .await?;

            total_tokens += result.tokens_used;

            descriptions.push(LanguageDescription {
                language: language.clone(),
                description: result.description,
                key_highlights: result.key_highlights,
                suggested_title: result.suggested_title,
                seo_keywords: result.seo_keywords,
                tokens_used: result.tokens_used,
            });
        }

        let generation_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(MultiLanguageDescriptionResult {
            listing_id: request.listing_id,
            descriptions,
            total_tokens_used: total_tokens,
            generation_time_ms,
        })
    }

    /// Get the system prompt for listing description generation.
    fn get_listing_system_prompt(&self, style: Option<&str>) -> String {
        let style_instruction = match style {
            Some("luxury") => "Use elegant, sophisticated language that appeals to high-end buyers. Emphasize exclusivity, premium finishes, and prestigious location.",
            Some("casual") => "Use friendly, approachable language. Focus on livability and community aspects.",
            Some("investment") => "Focus on investment potential, rental yields, and market position. Include relevant statistics.",
            _ => "Use professional, engaging language that highlights the property's best features while remaining factual.",
        };

        format!(
            r#"You are an expert real estate copywriter specializing in property listings for Central European markets.

Your task is to generate compelling property listing descriptions that attract potential buyers or renters.

Style Guidelines:
{}

Requirements:
1. Generate a main description (150-300 words unless specified otherwise)
2. Create 3-5 key highlights as bullet points
3. Suggest an attention-grabbing title
4. Provide SEO-friendly keywords
5. Highlight unique selling points
6. Include location benefits
7. Use appropriate language for the target market

The description should:
- Be engaging and persuasive
- Be accurate and not misleading
- Follow real estate advertising best practices
- Be culturally appropriate for the Central European market

Format your response with clear sections for each component."#,
            style_instruction
        )
    }

    // =========================================================================
    // Story 92.3: Document Summarization
    // =========================================================================

    /// Generate a summary for a document.
    ///
    /// Works for PDF, DOCX, and TXT documents by summarizing their extracted text content.
    pub async fn summarize_document(
        &self,
        request: DocumentSummarizationRequest,
    ) -> Result<DocumentSummarizationResult, DocumentGenerationError> {
        use std::time::Instant;

        let start_time = Instant::now();

        // Validate supported formats
        let supported_formats = ["pdf", "docx", "doc", "txt", "md", "rtf"];
        if !supported_formats.contains(&request.file_type.to_lowercase().as_str()) {
            return Err(DocumentGenerationError::UnsupportedFormat(format!(
                "File type '{}' is not supported. Supported formats: {}",
                request.file_type,
                supported_formats.join(", ")
            )));
        }

        if request.content.trim().is_empty() {
            return Err(DocumentGenerationError::ContentExtractionFailed(
                "Document content is empty".to_string(),
            ));
        }

        let language = request.language.as_deref().unwrap_or("en");
        let summary_length = request.summary_length.as_deref().unwrap_or("medium");

        let word_target = match summary_length {
            "short" => "50-100 words",
            "long" => "300-500 words",
            _ => "150-250 words", // medium
        };

        let system_prompt = format!(
            r#"You are an expert document analyst and summarizer.

Your task is to create a concise summary of the provided document in {} language.

Requirements:
1. Generate a summary of {} (target length)
2. Extract 3-7 key points as bullet points
3. Maintain the document's core message and important details
4. Use clear, professional language
5. Do not add information not present in the original document

Format your response as:
SUMMARY:
[Your summary here]

KEY POINTS:
- [Key point 1]
- [Key point 2]
..."#,
            language, word_target
        );

        // For very long documents, use chunked summarization
        let content_tokens = LlmClient::estimate_tokens(&request.content);
        let max_context = 100_000; // Leave room for system prompt and response

        let content_to_summarize = if content_tokens > max_context {
            // Take first and last portions for very long documents. Split on
            // UTF-8 char boundaries so multibyte content (e.g. Slovak/Czech
            // diacritics) never panics with "byte index N is not a char
            // boundary".
            let char_limit = (max_context * 4) as usize; // ~4 chars per token
            truncate_head_tail(&request.content, char_limit)
        } else {
            request.content.clone()
        };

        let llm_request = ChatCompletionRequest {
            model: self.default_model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: format!(
                        "Please summarize the following document:\n\n{}",
                        content_to_summarize
                    ),
                },
            ],
            temperature: Some(0.3),
            max_tokens: Some(2000),
        };

        let response = self
            .llm_client
            .chat(&self.default_provider, &llm_request)
            .await?;

        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Parse the response
        let (summary, key_points) = self.parse_summary_response(&content);

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(DocumentSummarizationResult {
            document_id: request.document_id,
            summary: summary.clone(),
            key_points,
            word_count: summary.split_whitespace().count(),
            tokens_used: response.usage.total_tokens,
            processing_time_ms,
        })
    }

    /// Parse the summary response from LLM.
    fn parse_summary_response(&self, content: &str) -> (String, Vec<String>) {
        let mut summary = String::new();
        let mut key_points = Vec::new();
        let mut in_summary = false;
        let mut in_key_points = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.to_uppercase().starts_with("SUMMARY:") {
                in_summary = true;
                in_key_points = false;
                let after_label = trimmed.trim_start_matches(|c: char| {
                    !c.is_alphabetic() || c.to_uppercase().next() == Some('S')
                });
                if let Some(rest) = after_label.strip_prefix("SUMMARY:") {
                    if !rest.trim().is_empty() {
                        summary.push_str(rest.trim());
                        summary.push(' ');
                    }
                }
                continue;
            }

            if trimmed.to_uppercase().starts_with("KEY POINTS:")
                || trimmed.to_uppercase().starts_with("KEY_POINTS:")
            {
                in_summary = false;
                in_key_points = true;
                continue;
            }

            if in_summary && !trimmed.is_empty() {
                summary.push_str(trimmed);
                summary.push(' ');
            }

            if in_key_points && trimmed.starts_with('-') {
                let point = trimmed.trim_start_matches('-').trim();
                if !point.is_empty() {
                    key_points.push(point.to_string());
                }
            }
        }

        // If parsing failed, use the whole content as summary
        if summary.is_empty() {
            summary = content.to_string();
        }

        (summary.trim().to_string(), key_points)
    }

    // =========================================================================
    // Story 92.4: Announcement Draft Generation
    // =========================================================================

    /// Generate announcement drafts.
    ///
    /// Creates well-structured announcements with appropriate tone based on urgency and audience.
    pub async fn generate_announcement_drafts(
        &self,
        request: AnnouncementDraftRequest,
    ) -> Result<AnnouncementDraftResult, DocumentGenerationError> {
        use std::time::Instant;

        let start_time = Instant::now();

        if request.topic.is_empty() {
            return Err(DocumentGenerationError::InvalidInput(
                "Topic is required".to_string(),
            ));
        }

        let num_drafts = request.num_drafts.unwrap_or(1).clamp(1, 3);
        let urgency = request.urgency.as_deref().unwrap_or("medium");
        let tone = request.tone.as_deref().unwrap_or("professional");

        let system_prompt = self.get_announcement_system_prompt(
            urgency,
            tone,
            &request.language,
            request.context.as_ref(),
        );

        let key_points_text = if request.key_points.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nKey points to include:\n{}",
                request
                    .key_points
                    .iter()
                    .map(|p| format!("- {}", p))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };

        let audience_text = request
            .audience
            .as_ref()
            .map(|a| format!("\n\nTarget audience: {}", a))
            .unwrap_or_default();

        let user_prompt = format!(
            r#"Generate {} announcement draft(s) for the following topic:

Topic: {}{}{}

Please provide {} complete draft(s), each with:
1. A clear, attention-grabbing title
2. Well-structured content (200-400 words)
3. Appropriate call to action if needed
4. Suggested target audience type (all, building, unit, role)

Format each draft as:
DRAFT 1:
TITLE: [Title here]
CONTENT:
[Content here]
TARGET: [Suggested target type]
TONE: [Brief analysis of the tone used]

[Repeat for additional drafts if requested]"#,
            num_drafts, request.topic, key_points_text, audience_text, num_drafts
        );

        let llm_request = ChatCompletionRequest {
            model: self.default_model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt,
                },
            ],
            temperature: Some(0.7), // Higher temperature for creative variety
            max_tokens: Some(4000),
        };

        let response = self
            .llm_client
            .chat(&self.default_provider, &llm_request)
            .await?;

        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let drafts = self.parse_announcement_drafts(&content);

        let generation_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(AnnouncementDraftResult {
            drafts,
            tokens_used: response.usage.total_tokens,
            generation_time_ms,
        })
    }

    /// Get the system prompt for announcement generation.
    fn get_announcement_system_prompt(
        &self,
        urgency: &str,
        tone: &str,
        language: &str,
        context: Option<&AnnouncementContext>,
    ) -> String {
        let urgency_instruction = match urgency {
            "critical" => "This is a CRITICAL announcement. Use urgent language, emphasize importance, and include clear action items with deadlines.",
            "high" => "This is a high-priority announcement. Use clear, direct language and emphasize the importance of timely action.",
            "low" => "This is a routine announcement. Use a relaxed, informative tone.",
            _ => "This is a standard announcement. Use clear, professional language.",
        };

        let tone_instruction = match tone {
            "formal" => {
                "Use formal, official language appropriate for legal or regulatory matters."
            }
            "friendly" => "Use warm, approachable language that builds community connection.",
            "urgent" => "Use direct, action-oriented language with clear calls to action.",
            _ => "Use professional, clear language that is respectful and informative.",
        };

        let language_name = match language {
            "sk" => "Slovak",
            "cs" => "Czech",
            "de" => "German",
            _ => "English",
        };

        let context_info = context
            .map(|c| {
                let mut info = String::new();
                if let Some(ref name) = c.building_name {
                    info.push_str(&format!("\nBuilding Name: {}", name));
                }
                if let Some(ref address) = c.building_address {
                    info.push_str(&format!("\nBuilding Address: {}", address));
                }
                if let Some(ref contact) = c.management_contact {
                    info.push_str(&format!("\nManagement Contact: {}", contact));
                }
                if !info.is_empty() {
                    format!("\n\nBuilding Context:{}", info)
                } else {
                    String::new()
                }
            })
            .unwrap_or_default();

        format!(
            r#"You are an expert property management communications specialist.

Your task is to create professional announcements for building residents and owners.

Language: Write in {} language
Urgency Level: {}
Tone: {}
{}

Guidelines:
1. Write clear, well-structured announcements
2. Use appropriate formatting (paragraphs, bullet points where helpful)
3. Include relevant dates, times, and locations when applicable
4. Provide clear contact information or next steps
5. Be culturally appropriate for Central European readers
6. Avoid jargon and use accessible language

The announcements should be ready for immediate publication after minimal editing."#,
            language_name, urgency_instruction, tone_instruction, context_info
        )
    }

    /// Parse announcement drafts from LLM response.
    fn parse_announcement_drafts(&self, content: &str) -> Vec<AnnouncementDraft> {
        let mut drafts = Vec::new();
        let mut current_draft: Option<AnnouncementDraft> = None;
        let mut current_section = "";
        let mut current_content = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Check for new draft
            if trimmed.to_uppercase().starts_with("DRAFT ") {
                // Save previous draft if exists
                if let Some(mut draft) = current_draft.take() {
                    if current_section == "content" {
                        draft.content = current_content.trim().to_string();
                    }
                    if !draft.title.is_empty() {
                        drafts.push(draft);
                    }
                }
                current_draft = Some(AnnouncementDraft {
                    title: String::new(),
                    content: String::new(),
                    suggested_target: "all".to_string(),
                    tone_analysis: String::new(),
                });
                current_section = "";
                current_content.clear();
                continue;
            }

            if let Some(ref mut draft) = current_draft {
                if trimmed.to_uppercase().starts_with("TITLE:") {
                    if current_section == "content" {
                        draft.content = current_content.trim().to_string();
                    }
                    // Strip the "TITLE:" prefix (case-insensitive)
                    draft.title = trimmed
                        .strip_prefix("TITLE:")
                        .or_else(|| trimmed.strip_prefix("Title:"))
                        .or_else(|| trimmed.strip_prefix("title:"))
                        .unwrap_or(trimmed)
                        .trim()
                        .to_string();
                    current_section = "title";
                    current_content.clear();
                } else if trimmed.to_uppercase().starts_with("CONTENT:") {
                    current_section = "content";
                    current_content.clear();
                } else if trimmed.to_uppercase().starts_with("TARGET:") {
                    if current_section == "content" {
                        draft.content = current_content.trim().to_string();
                    }
                    // Strip the "TARGET:" prefix (case-insensitive)
                    draft.suggested_target = trimmed
                        .strip_prefix("TARGET:")
                        .or_else(|| trimmed.strip_prefix("Target:"))
                        .or_else(|| trimmed.strip_prefix("target:"))
                        .unwrap_or(trimmed)
                        .trim()
                        .to_lowercase();
                    current_section = "target";
                    current_content.clear();
                } else if trimmed.to_uppercase().starts_with("TONE:") {
                    if current_section == "content" {
                        draft.content = current_content.trim().to_string();
                    }
                    // Strip the "TONE:" prefix (case-insensitive)
                    draft.tone_analysis = trimmed
                        .strip_prefix("TONE:")
                        .or_else(|| trimmed.strip_prefix("Tone:"))
                        .or_else(|| trimmed.strip_prefix("tone:"))
                        .unwrap_or(trimmed)
                        .trim()
                        .to_string();
                    current_section = "tone";
                    current_content.clear();
                } else if current_section == "content" {
                    current_content.push_str(trimmed);
                    current_content.push('\n');
                }
            }
        }

        // Don't forget the last draft
        if let Some(mut draft) = current_draft {
            if current_section == "content" {
                draft.content = current_content.trim().to_string();
            }
            if !draft.title.is_empty() {
                drafts.push(draft);
            }
        }

        // If no drafts were parsed, create one from the entire content
        if drafts.is_empty() && !content.trim().is_empty() {
            drafts.push(AnnouncementDraft {
                title: "Generated Announcement".to_string(),
                content: content.to_string(),
                suggested_target: "all".to_string(),
                tone_analysis: "Unable to parse structured response".to_string(),
            });
        }

        drafts
    }
}

/// Largest byte index `<= index` that lies on a UTF-8 char boundary.
///
/// Mirrors the unstable `str::floor_char_boundary`. Used to truncate a
/// document head without splitting a multibyte character (Slovak/Czech
/// diacritics), which would otherwise panic with
/// "byte index N is not a char boundary".
fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        s.len()
    } else {
        let mut i = index;
        while i > 0 && !s.is_char_boundary(i) {
            i -= 1;
        }
        i
    }
}

/// Smallest byte index `>= index` that lies on a UTF-8 char boundary.
///
/// Mirrors the unstable `str::ceil_char_boundary`. Used to truncate a
/// document tail on a valid boundary.
fn ceil_char_boundary(s: &str, index: usize) -> usize {
    let mut i = index.min(s.len());
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Build a head + tail excerpt of `content` fitting within roughly
/// `char_limit` characters, splitting only on UTF-8 char boundaries so
/// multibyte content never triggers a "byte index N is not a char boundary"
/// panic.
fn truncate_head_tail(content: &str, char_limit: usize) -> String {
    let half_limit = char_limit / 2;
    let head_end = floor_char_boundary(content, half_limit.min(content.len()));
    let tail_start = ceil_char_boundary(content, content.len().saturating_sub(half_limit));
    format!(
        "[Document truncated for length]\n\n{}\n\n[...]\n\n{}",
        &content[..head_end],
        &content[tail_start..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_summary_response() {
        let service = DocumentGenerationService::new(LlmClient::new());

        let content = r#"SUMMARY:
This is a test summary that spans multiple lines and contains important information about the document.

KEY POINTS:
- First key point about the document
- Second key point with details
- Third point summarizing findings"#;

        let (summary, key_points) = service.parse_summary_response(content);

        assert!(!summary.is_empty());
        assert!(summary.contains("test summary"));
        assert_eq!(key_points.len(), 3);
        assert!(key_points[0].contains("First key point"));
    }

    #[test]
    fn test_parse_announcement_drafts() {
        let service = DocumentGenerationService::new(LlmClient::new());

        let content = r#"DRAFT 1:
TITLE: Important Maintenance Notice
CONTENT:
Dear Residents,

We will be performing scheduled maintenance on the building's water system next week.

Please ensure you are prepared for potential water outages.

TARGET: all
TONE: Professional and informative

DRAFT 2:
TITLE: Water System Maintenance
CONTENT:
Attention all residents:

Scheduled maintenance is planned for the building water system.

TARGET: building
TONE: Direct and clear"#;

        let drafts = service.parse_announcement_drafts(content);

        assert_eq!(drafts.len(), 2);
        assert_eq!(drafts[0].title, "Important Maintenance Notice");
        assert!(drafts[0].content.contains("Dear Residents"));
        assert_eq!(drafts[0].suggested_target, "all");
    }

    #[test]
    fn test_lease_system_prompt_slovakia() {
        let service = DocumentGenerationService::new(LlmClient::new());
        let prompt = service.get_lease_system_prompt("SK", "sk");

        assert!(prompt.contains("Slovak Civil Code"));
        assert!(prompt.contains("Slovak language"));
    }

    #[test]
    fn test_lease_system_prompt_czech() {
        let service = DocumentGenerationService::new(LlmClient::new());
        let prompt = service.get_lease_system_prompt("CZ", "cs");

        assert!(prompt.contains("Czech Civil Code"));
        assert!(prompt.contains("Czech language"));
    }

    #[test]
    fn test_listing_system_prompt_luxury() {
        let service = DocumentGenerationService::new(LlmClient::new());
        let prompt = service.get_listing_system_prompt(Some("luxury"));

        assert!(prompt.contains("elegant"));
        assert!(prompt.contains("sophisticated"));
    }

    #[test]
    fn test_announcement_system_prompt_critical() {
        let service = DocumentGenerationService::new(LlmClient::new());
        let prompt = service.get_announcement_system_prompt("critical", "urgent", "en", None);

        assert!(prompt.contains("CRITICAL"));
        assert!(prompt.contains("urgent"));
    }

    #[test]
    fn test_truncate_head_tail_multibyte_no_panic() {
        // Repeated 2-byte characters guarantee that most byte offsets land
        // mid-character. Before the char-boundary fix this panicked with
        // "byte index N is not a char boundary".
        let content = "áéíóú".repeat(2000); // 10_000 chars, 20_000 bytes
                                            // Odd char_limit forces half_limit to fall mid-character.
        let out = truncate_head_tail(&content, 1001);

        assert!(out.starts_with("[Document truncated for length]"));
        assert!(out.contains("[...]"));
        // Head and tail must be prefixes/suffixes of the original content —
        // proving the split landed on a clean char boundary and produced no
        // replacement/garbled characters.
        let body = out.trim_start_matches("[Document truncated for length]\n\n");
        let (head, tail) = body.split_once("\n\n[...]\n\n").expect("delimiter present");
        assert!(!head.is_empty() && content.starts_with(head));
        assert!(!tail.is_empty() && content.ends_with(tail));
    }

    #[test]
    fn test_char_boundary_helpers_align() {
        let s = "áéíóú"; // each char is 2 bytes: boundaries at 0,2,4,6,8,10
                         // floor: 3 (mid-char) -> 2
        assert_eq!(floor_char_boundary(s, 3), 2);
        assert_eq!(floor_char_boundary(s, 2), 2);
        assert_eq!(floor_char_boundary(s, 100), s.len());
        assert_eq!(floor_char_boundary(s, 0), 0);
        // ceil: 3 (mid-char) -> 4
        assert_eq!(ceil_char_boundary(s, 3), 4);
        assert_eq!(ceil_char_boundary(s, 4), 4);
        assert_eq!(ceil_char_boundary(s, 100), s.len());
        // The aligned indices are always valid slice points.
        for i in 0..=s.len() {
            let f = floor_char_boundary(s, i);
            let c = ceil_char_boundary(s, i);
            let _ = &s[..f];
            let _ = &s[c..];
        }
    }
}
