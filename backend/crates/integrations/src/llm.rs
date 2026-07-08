//! LLM API client for AI capabilities (Epic 64: Advanced AI & LLM Capabilities).
//!
//! Provides unified interface for:
//! - OpenAI (GPT-4, GPT-4o)
//! - Anthropic (Claude)
//! - Azure OpenAI
//!
//! Features:
//! - Lease agreement generation (Story 64.1)
//! - Property listing descriptions (Story 64.2)
//! - Conversational AI with RAG (Story 64.3)
//! - Photo enhancement coordination (Story 64.4)
//! - Embedding generation for RAG (Story 97.2)
//! - Sentiment analysis (Story 97.4)
//! - Context management with token limits (Story 97.1)

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

/// Default token limits for different models.
pub mod token_limits {
    /// GPT-4o context window
    pub const GPT4O_CONTEXT: u32 = 128_000;
    /// GPT-4o-mini context window
    pub const GPT4O_MINI_CONTEXT: u32 = 128_000;
    /// Claude 3.5 Sonnet context window
    pub const CLAUDE_35_SONNET_CONTEXT: u32 = 200_000;
    /// Claude 3.5 Haiku context window
    pub const CLAUDE_35_HAIKU_CONTEXT: u32 = 200_000;
    /// Default max tokens for chat responses
    pub const DEFAULT_MAX_TOKENS: i32 = 4096;
    /// Reserved tokens for response generation
    pub const RESERVED_FOR_RESPONSE: u32 = 4096;
    /// Approximate characters per token (for estimation)
    pub const CHARS_PER_TOKEN: usize = 4;
}

/// LLM API errors.
#[derive(Error, Debug)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limited: retry after {retry_after} seconds")]
    RateLimited { retry_after: u64 },

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Missing API key for provider: {0}")]
    MissingApiKey(String),

    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),

    #[error("Context too long: {tokens} tokens exceeds limit of {limit}")]
    ContextTooLong { tokens: u32, limit: u32 },

    #[error("Content filtered: {reason}")]
    ContentFiltered { reason: String },

    #[error("Timeout")]
    Timeout,
}

/// LLM provider configuration.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub anthropic_api_version: String,
    pub azure_openai_endpoint: Option<String>,
    pub azure_openai_api_key: Option<String>,
    pub azure_openai_deployment: Option<String>,
    pub default_timeout_secs: u64,
    pub max_retries: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            anthropic_api_version: std::env::var("ANTHROPIC_API_VERSION")
                .unwrap_or_else(|_| "2024-10-22".to_string()),
            azure_openai_endpoint: std::env::var("AZURE_OPENAI_ENDPOINT").ok(),
            azure_openai_api_key: std::env::var("AZURE_OPENAI_API_KEY").ok(),
            azure_openai_deployment: std::env::var("AZURE_OPENAI_DEPLOYMENT").ok(),
            default_timeout_secs: 120,
            max_retries: 3,
        }
    }
}

/// Unified LLM client.
#[derive(Debug, Clone)]
pub struct LlmClient {
    http_client: Client,
    config: LlmConfig,
}

impl LlmClient {
    /// Create a new LLM client with default configuration.
    pub fn new() -> Self {
        Self::with_config(LlmConfig::default())
    }

    /// Create a new LLM client with custom configuration.
    pub fn with_config(config: LlmConfig) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.default_timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            config,
        }
    }

    /// Whether an OpenAI API key is configured.
    ///
    /// Callers use this to decide between the live OpenAI embedding provider
    /// and the deterministic offline [`crate::embedding::StubEmbeddingProvider`]
    /// (Story 84.5) — e.g. the RAG embedding-write flow degrades to the stub
    /// when no key is present so indexing still works in CI / air-gapped
    /// deployments instead of hard-failing with `MissingApiKey`.
    pub fn has_openai_key(&self) -> bool {
        self.config.openai_api_key.is_some()
    }

    /// Complete a chat using OpenAI API.
    pub async fn openai_chat(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, LlmError> {
        let api_key = self
            .config
            .openai_api_key
            .as_ref()
            .ok_or_else(|| LlmError::MissingApiKey("openai".to_string()))?;

        let response = self
            .http_client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&OpenAiRequest::from(request.clone()))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            if status == 429 {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(60);
                return Err(LlmError::RateLimited { retry_after });
            }
            let error_body = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError {
                status,
                message: error_body,
            });
        }

        let openai_response: OpenAiResponse = response.json().await?;
        Ok(openai_response.into())
    }

    /// Complete a chat using Anthropic API.
    pub async fn anthropic_chat(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, LlmError> {
        let api_key = self
            .config
            .anthropic_api_key
            .as_ref()
            .ok_or_else(|| LlmError::MissingApiKey("anthropic".to_string()))?;

        let anthropic_request = AnthropicRequest::from(request.clone());

        let response = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", &self.config.anthropic_api_version)
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            if status == 429 {
                return Err(LlmError::RateLimited { retry_after: 60 });
            }
            let error_body = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError {
                status,
                message: error_body,
            });
        }

        let anthropic_response: AnthropicResponse = response.json().await?;
        Ok(anthropic_response.into())
    }

    /// Complete a chat using the specified provider.
    pub async fn chat(
        &self,
        provider: &str,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, LlmError> {
        match provider {
            "openai" => self.openai_chat(request).await,
            "anthropic" => self.anthropic_chat(request).await,
            "azure_openai" => self.azure_openai_chat(request).await,
            _ => Err(LlmError::UnsupportedProvider(provider.to_string())),
        }
    }

    /// Complete a chat using Azure OpenAI API.
    pub async fn azure_openai_chat(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, LlmError> {
        let endpoint = self
            .config
            .azure_openai_endpoint
            .as_ref()
            .ok_or_else(|| LlmError::MissingApiKey("azure_openai_endpoint".to_string()))?;
        let api_key = self
            .config
            .azure_openai_api_key
            .as_ref()
            .ok_or_else(|| LlmError::MissingApiKey("azure_openai".to_string()))?;
        let deployment = self
            .config
            .azure_openai_deployment
            .as_ref()
            .ok_or_else(|| LlmError::MissingApiKey("azure_openai_deployment".to_string()))?;

        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version=2024-02-15-preview",
            endpoint, deployment
        );

        let response = self
            .http_client
            .post(&url)
            .header("api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&OpenAiRequest::from(request.clone()))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError {
                status,
                message: error_body,
            });
        }

        let openai_response: OpenAiResponse = response.json().await?;
        Ok(openai_response.into())
    }

    /// Generate a lease agreement.
    pub async fn generate_lease(
        &self,
        provider: &str,
        model: &str,
        system_prompt: &str,
        lease_details: &LeaseGenerationInput,
        language: &str,
    ) -> Result<LeaseGenerationResult, LlmError> {
        let user_prompt = format!(
            r#"Generate a comprehensive lease agreement in {} language with the following details:

Landlord: {}
Landlord Address: {}
Tenant: {}
Tenant Email: {}
Tenant Phone: {}
Property: Unit {}
Start Date: {}
End Date: {}
Monthly Rent: {} {}
Security Deposit: {} {}
Additional Terms: {}
Include Pet Clause: {}
Include Parking: {}
Jurisdiction: {}

Please generate a legally appropriate lease agreement with all standard clauses for the specified jurisdiction. Format the output as follows:
1. Document HTML (wrapped in <document_html></document_html> tags)
2. Plain text version (wrapped in <document_text></document_text> tags)
3. List of clauses with their titles and whether they are mandatory
4. Any warnings or compliance notes"#,
            language,
            lease_details.landlord_name,
            lease_details.landlord_address.as_deref().unwrap_or("N/A"),
            lease_details.tenant_name,
            lease_details.tenant_email,
            lease_details.tenant_phone.as_deref().unwrap_or("N/A"),
            lease_details.unit_id,
            lease_details.start_date,
            lease_details.end_date,
            lease_details.monthly_rent,
            lease_details.currency,
            lease_details.security_deposit,
            lease_details.currency,
            lease_details
                .additional_terms
                .as_ref()
                .map(|t| t.join(", "))
                .unwrap_or_else(|| "None".to_string()),
            lease_details.include_pet_clause,
            lease_details.include_parking,
            lease_details.jurisdiction.as_deref().unwrap_or("Slovakia"),
        );

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt,
                },
            ],
            temperature: Some(0.3),
            max_tokens: Some(8000),
        };

        let response = self.chat(provider, &request).await?;
        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Parse the response
        let document_html = extract_between(&content, "<document_html>", "</document_html>")
            .unwrap_or_else(|| content.clone());
        let document_text = extract_between(&content, "<document_text>", "</document_text>")
            .unwrap_or_else(|| content.clone());

        Ok(LeaseGenerationResult {
            document_html,
            document_text,
            clauses: vec![],
            warnings: vec![],
            compliance_notes: None,
            tokens_used: response.usage.total_tokens,
        })
    }

    /// Generate a property listing description.
    pub async fn generate_listing_description(
        &self,
        provider: &str,
        model: &str,
        system_prompt: &str,
        input: &ListingDescriptionInput,
    ) -> Result<ListingDescriptionResult, LlmError> {
        let features_str = input.features.join(", ");
        let amenities_str = input
            .nearby_amenities
            .as_ref()
            .map(|a| a.join(", "))
            .unwrap_or_default();

        let user_prompt = format!(
            r#"Generate an attractive property listing description in {} language:

Property Type: {}
Transaction: {} (Price: {} {})
Size: {} sqm
Rooms: {} rooms, {} bathrooms
Floor: {} of {}
Location: {}, {}
Features: {}
Nearby: {}
Style: {}
Max Length: {} words

Generate:
1. An engaging description highlighting the property's best features
2. 3-5 key highlights as bullet points
3. A suggested title
4. SEO keywords (comma-separated)"#,
            input.language,
            input.property_type,
            input.transaction_type,
            input.price,
            input.currency,
            input.size_sqm.unwrap_or(0.0),
            input.rooms.unwrap_or(0),
            input.bathrooms.unwrap_or(0),
            input.floor.unwrap_or(0),
            input.total_floors.unwrap_or(0),
            input.city,
            input.district.as_deref().unwrap_or(""),
            features_str,
            amenities_str,
            input.style.as_deref().unwrap_or("professional"),
            input.max_length.unwrap_or(300),
        );

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt,
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(2000),
        };

        let response = self.chat(provider, &request).await?;
        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(ListingDescriptionResult {
            description: content,
            key_highlights: vec![],
            suggested_title: None,
            seo_keywords: None,
            tokens_used: response.usage.total_tokens,
        })
    }

    /// Enhanced chat with context for RAG.
    pub async fn enhanced_chat(
        &self,
        provider: &str,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        context_chunks: &[ContextChunk],
        language: &str,
    ) -> Result<EnhancedChatResult, LlmError> {
        let context_text = if !context_chunks.is_empty() {
            let chunks: Vec<String> = context_chunks
                .iter()
                .map(|c| format!("[Source: {}]\n{}", c.source_title, c.text))
                .collect();
            format!(
                "Relevant context from building documents:\n\n{}\n\n",
                chunks.join("\n---\n")
            )
        } else {
            String::new()
        };

        let enhanced_system = format!(
            "{}\n\nIMPORTANT: Respond in {} language. If you cannot answer confidently (confidence below 80%), indicate that escalation to a human is needed.",
            system_prompt, language
        );

        let user_content = format!("{}{}", context_text, user_message);

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: enhanced_system,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_content,
                },
            ],
            temperature: Some(0.5),
            max_tokens: Some(2000),
        };

        let response = self.chat(provider, &request).await?;
        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Simple escalation detection
        let escalated = content.to_lowercase().contains("escalat")
            || content.to_lowercase().contains("human")
            || content.to_lowercase().contains("cannot answer");

        Ok(EnhancedChatResult {
            response: content,
            confidence: if escalated { 0.5 } else { 0.9 },
            escalated,
            escalation_reason: if escalated {
                Some("Low confidence or complex query".to_string())
            } else {
                None
            },
            sources_used: context_chunks.iter().map(|c| c.source_id).collect(),
            tokens_used: response.usage.total_tokens,
        })
    }

    // =========================================================================
    // Story 97.1: Context Management
    // =========================================================================

    /// Estimate token count from text using simple character-based approximation.
    /// For production use, consider using tiktoken or similar library.
    pub fn estimate_tokens(text: &str) -> u32 {
        (text.len() / token_limits::CHARS_PER_TOKEN) as u32
    }

    /// Get context window limit for a model.
    pub fn get_context_limit(model: &str) -> u32 {
        match model {
            m if m.contains("gpt-4o") => token_limits::GPT4O_CONTEXT,
            m if m.contains("gpt-4") => 128_000,
            m if m.contains("claude-3") => token_limits::CLAUDE_35_SONNET_CONTEXT,
            _ => 32_000, // Conservative default
        }
    }

    /// Truncate messages to fit within token limit, preserving system prompt and recent messages.
    pub fn truncate_messages_to_fit(
        messages: &[ChatMessage],
        model: &str,
        max_response_tokens: u32,
    ) -> Vec<ChatMessage> {
        let context_limit = Self::get_context_limit(model);
        let available_tokens = context_limit.saturating_sub(max_response_tokens);

        let mut result = Vec::new();
        let mut total_tokens: u32 = 0;

        // Always include system message first
        if let Some(system_msg) = messages.iter().find(|m| m.role == "system") {
            let msg_tokens = Self::estimate_tokens(&system_msg.content);
            total_tokens += msg_tokens;
            result.push(system_msg.clone());
        }

        // Get non-system messages in reverse order (most recent first)
        let non_system_msgs: Vec<_> = messages.iter().filter(|m| m.role != "system").collect();

        // Add messages from most recent, stopping when we hit the limit
        for msg in non_system_msgs.into_iter().rev() {
            let msg_tokens = Self::estimate_tokens(&msg.content);
            if total_tokens + msg_tokens <= available_tokens {
                total_tokens += msg_tokens;
                result.push(msg.clone());
            } else {
                break;
            }
        }

        // Reverse non-system messages to restore chronological order
        // (system message stays at index 0, rest are reversed)
        if result.len() > 1 {
            let system_msg = result.remove(0);
            result.reverse();
            result.insert(0, system_msg);
        }

        result
    }

    /// Build a system prompt with tenant-specific configuration.
    pub fn build_system_prompt(
        base_prompt: &str,
        tenant_config: Option<&TenantAiConfig>,
        language: &str,
    ) -> String {
        let mut prompt = base_prompt.to_string();

        if let Some(config) = tenant_config {
            if let Some(ref personality) = config.personality {
                prompt = format!("{}\n\nPersonality: {}", prompt, personality);
            }
            if let Some(ref building_context) = config.building_context {
                prompt = format!(
                    "{}\n\nBuilding-specific information:\n{}",
                    prompt, building_context
                );
            }
            if !config.custom_instructions.is_empty() {
                prompt = format!(
                    "{}\n\nCustom instructions:\n{}",
                    prompt,
                    config.custom_instructions.join("\n")
                );
            }
        }

        format!(
            "{}\n\nIMPORTANT: Always respond in {} language.",
            prompt, language
        )
    }

    // =========================================================================
    // Story 97.2: Embedding Generation
    // =========================================================================

    /// Generate embeddings using OpenAI's embedding API.
    pub async fn generate_embedding(
        &self,
        text: &str,
        model: Option<&str>,
    ) -> Result<EmbeddingResult, LlmError> {
        let api_key = self
            .config
            .openai_api_key
            .as_ref()
            .ok_or_else(|| LlmError::MissingApiKey("openai".to_string()))?;

        let embedding_model = model.unwrap_or("text-embedding-3-small");

        let request = OpenAiEmbeddingRequest {
            model: embedding_model.to_string(),
            input: text.to_string(),
        };

        let response = self
            .http_client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            if status == 429 {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(60);
                return Err(LlmError::RateLimited { retry_after });
            }
            let error_body = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError {
                status,
                message: error_body,
            });
        }

        let embedding_response: OpenAiEmbeddingResponse = response.json().await?;

        let embedding = embedding_response
            .data
            .first()
            .map(|d| d.embedding.clone())
            .ok_or_else(|| LlmError::InvalidResponse("No embedding in response".to_string()))?;

        Ok(EmbeddingResult {
            embedding,
            model: embedding_model.to_string(),
            tokens_used: embedding_response.usage.total_tokens,
        })
    }

    /// Generate embeddings for multiple texts in batch.
    pub async fn generate_embeddings_batch(
        &self,
        texts: &[String],
        model: Option<&str>,
    ) -> Result<Vec<EmbeddingResult>, LlmError> {
        // Process in batches to avoid rate limits
        let mut results = Vec::new();
        for text in texts {
            let result = self.generate_embedding(text, model).await?;
            results.push(result);
        }
        Ok(results)
    }

    // =========================================================================
    // Story 97.4: Sentiment Analysis
    // =========================================================================

    /// Analyze sentiment of text using OpenAI with structured output.
    pub async fn analyze_sentiment(
        &self,
        text: &str,
        provider: Option<&str>,
    ) -> Result<SentimentResult, LlmError> {
        let provider = provider.unwrap_or("openai");
        let model = match provider {
            "openai" => "gpt-4o-mini",
            "anthropic" => "claude-3-5-haiku-20241022",
            _ => "gpt-4o-mini",
        };

        let system_prompt = r#"You are a sentiment analysis assistant. Analyze the sentiment of the given text and respond with a JSON object containing:
- score: a number from -1.0 (very negative) to 1.0 (very positive)
- label: one of "negative", "neutral", "positive"
- confidence: a number from 0.0 to 1.0 indicating your confidence
- key_phrases: an array of key phrases that influenced the sentiment
- requires_attention: boolean, true if the message indicates urgent issues, complaints, or frustration that management should address

Respond ONLY with valid JSON, no other text."#;

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: text.to_string(),
                },
            ],
            temperature: Some(0.1), // Low temperature for consistent analysis
            max_tokens: Some(500),
        };

        let response = self.chat(provider, &request).await?;
        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Parse the JSON response
        let sentiment: SentimentResult = serde_json::from_str(&content).map_err(|e| {
            LlmError::InvalidResponse(format!("Failed to parse sentiment response: {}", e))
        })?;

        Ok(sentiment)
    }

    /// Analyze sentiment for multiple texts, returning aggregate statistics.
    pub async fn analyze_sentiment_batch(
        &self,
        texts: &[String],
        provider: Option<&str>,
    ) -> Result<BatchSentimentResult, LlmError> {
        let mut results = Vec::new();
        let mut total_score = 0.0;
        let mut negative_count = 0;
        let mut neutral_count = 0;
        let mut positive_count = 0;
        let mut attention_required = Vec::new();

        for (i, text) in texts.iter().enumerate() {
            let result = self.analyze_sentiment(text, provider).await?;

            match result.label.as_str() {
                "negative" => negative_count += 1,
                "neutral" => neutral_count += 1,
                "positive" => positive_count += 1,
                _ => {}
            }

            total_score += result.score;

            if result.requires_attention {
                attention_required.push(i);
            }

            results.push(result);
        }

        let count = texts.len() as f64;
        let avg_score = if count > 0.0 {
            total_score / count
        } else {
            0.0
        };

        Ok(BatchSentimentResult {
            results,
            average_score: avg_score,
            negative_count,
            neutral_count,
            positive_count,
            attention_required_indices: attention_required,
        })
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

/// Chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

/// Chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: TokenUsage,
}

/// Chat choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: i32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Token usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

// =============================================================================
// OpenAI Types
// =============================================================================

#[derive(Debug, Clone, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
}

impl From<ChatCompletionRequest> for OpenAiRequest {
    fn from(req: ChatCompletionRequest) -> Self {
        Self {
            model: req.model,
            messages: req.messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiResponse {
    id: String,
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: OpenAiUsage,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiChoice {
    index: i32,
    message: ChatMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

impl From<OpenAiResponse> for ChatCompletionResponse {
    fn from(resp: OpenAiResponse) -> Self {
        Self {
            id: resp.id,
            model: resp.model,
            choices: resp
                .choices
                .into_iter()
                .map(|c| ChatChoice {
                    index: c.index,
                    message: c.message,
                    finish_reason: c.finish_reason,
                })
                .collect(),
            usage: TokenUsage {
                prompt_tokens: resp.usage.prompt_tokens,
                completion_tokens: resp.usage.completion_tokens,
                total_tokens: resp.usage.total_tokens,
            },
        }
    }
}

// =============================================================================
// Anthropic Types
// =============================================================================

#[derive(Debug, Clone, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

impl From<ChatCompletionRequest> for AnthropicRequest {
    fn from(req: ChatCompletionRequest) -> Self {
        let system = req
            .messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        let messages: Vec<AnthropicMessage> = req
            .messages
            .into_iter()
            .filter(|m| m.role != "system")
            .map(|m| AnthropicMessage {
                role: m.role,
                content: m.content,
            })
            .collect();

        Self {
            model: req.model,
            messages,
            system,
            max_tokens: req.max_tokens.unwrap_or(4096),
            temperature: req.temperature,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<AnthropicContent>,
    usage: AnthropicUsage,
    stop_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicUsage {
    input_tokens: i32,
    output_tokens: i32,
}

impl From<AnthropicResponse> for ChatCompletionResponse {
    fn from(resp: AnthropicResponse) -> Self {
        let content = resp
            .content
            .into_iter()
            .filter_map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Self {
            id: resp.id,
            model: resp.model,
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content,
                },
                finish_reason: resp.stop_reason,
            }],
            usage: TokenUsage {
                prompt_tokens: resp.usage.input_tokens,
                completion_tokens: resp.usage.output_tokens,
                total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
            },
        }
    }
}

// =============================================================================
// Domain-Specific Types
// =============================================================================

/// Lease generation input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseGenerationInput {
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
    pub jurisdiction: Option<String>,
}

/// Lease generation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseGenerationResult {
    pub document_html: String,
    pub document_text: String,
    pub clauses: Vec<LeaseClause>,
    pub warnings: Vec<String>,
    pub compliance_notes: Option<String>,
    pub tokens_used: i32,
}

/// Lease clause.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseClause {
    pub title: String,
    pub content: String,
    pub is_mandatory: bool,
}

/// Listing description input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingDescriptionInput {
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
    pub language: String,
    pub style: Option<String>,
    pub max_length: Option<i32>,
}

/// Listing description result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingDescriptionResult {
    pub description: String,
    pub key_highlights: Vec<String>,
    pub suggested_title: Option<String>,
    pub seo_keywords: Option<Vec<String>>,
    pub tokens_used: i32,
}

/// Context chunk for RAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    pub source_id: Uuid,
    pub source_title: String,
    pub text: String,
    pub relevance_score: f64,
}

/// Enhanced chat result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedChatResult {
    pub response: String,
    pub confidence: f64,
    pub escalated: bool,
    pub escalation_reason: Option<String>,
    pub sources_used: Vec<Uuid>,
    pub tokens_used: i32,
}

// =============================================================================
// Story 97.1: Tenant AI Configuration
// =============================================================================

/// Tenant-specific AI configuration for customizing chatbot behavior.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TenantAiConfig {
    /// Custom personality for the AI (e.g., "friendly and professional")
    pub personality: Option<String>,
    /// Building-specific context and knowledge
    pub building_context: Option<String>,
    /// Custom instructions to follow
    pub custom_instructions: Vec<String>,
    /// Preferred language for responses
    pub preferred_language: Option<String>,
    /// Topics that should trigger escalation
    pub escalation_topics: Vec<String>,
}

// =============================================================================
// Story 97.2: Embedding Types
// =============================================================================

/// OpenAI embedding request.
#[derive(Debug, Clone, Serialize)]
struct OpenAiEmbeddingRequest {
    model: String,
    input: String,
}

/// OpenAI embedding response.
#[derive(Debug, Clone, Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<OpenAiEmbeddingData>,
    usage: OpenAiEmbeddingUsage,
}

/// OpenAI embedding data.
#[derive(Debug, Clone, Deserialize)]
struct OpenAiEmbeddingData {
    embedding: Vec<f32>,
    #[allow(dead_code)]
    index: i32,
}

/// OpenAI embedding usage.
#[derive(Debug, Clone, Deserialize)]
struct OpenAiEmbeddingUsage {
    #[allow(dead_code)]
    prompt_tokens: i32,
    total_tokens: i32,
}

/// Result of embedding generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    /// The embedding vector (1536 dimensions for text-embedding-3-small)
    pub embedding: Vec<f32>,
    /// The model used for embedding
    pub model: String,
    /// Number of tokens used
    pub tokens_used: i32,
}

// =============================================================================
// Story 97.4: Sentiment Analysis Types
// =============================================================================

/// Result of sentiment analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentResult {
    /// Sentiment score from -1.0 (very negative) to 1.0 (very positive)
    pub score: f64,
    /// Sentiment label: "negative", "neutral", or "positive"
    pub label: String,
    /// Confidence in the analysis from 0.0 to 1.0
    pub confidence: f64,
    /// Key phrases that influenced the sentiment
    pub key_phrases: Vec<String>,
    /// Whether this message requires management attention
    pub requires_attention: bool,
}

/// Result of batch sentiment analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSentimentResult {
    /// Individual sentiment results
    pub results: Vec<SentimentResult>,
    /// Average sentiment score across all texts
    pub average_score: f64,
    /// Count of negative sentiments
    pub negative_count: usize,
    /// Count of neutral sentiments
    pub neutral_count: usize,
    /// Count of positive sentiments
    pub positive_count: usize,
    /// Indices of texts that require attention
    pub attention_required_indices: Vec<usize>,
}

// =============================================================================
// Helper Functions
// =============================================================================

fn extract_between(text: &str, start_tag: &str, end_tag: &str) -> Option<String> {
    let start = text.find(start_tag)?;
    let end = text.find(end_tag)?;
    if start < end {
        Some(text[start + start_tag.len()..end].trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_between() {
        let text = "<doc>Hello World</doc>";
        assert_eq!(
            extract_between(text, "<doc>", "</doc>"),
            Some("Hello World".to_string())
        );

        let text_no_match = "Hello World";
        assert_eq!(extract_between(text_no_match, "<doc>", "</doc>"), None);
    }

    #[test]
    fn test_openai_request_from_chat_request() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: Some(0.7),
            max_tokens: Some(1000),
        };

        let openai_request = OpenAiRequest::from(request);
        assert_eq!(openai_request.model, "gpt-4");
        assert_eq!(openai_request.messages.len(), 1);
    }

    #[test]
    fn test_anthropic_request_extracts_system() {
        let request = ChatCompletionRequest {
            model: "claude-3-opus".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are helpful".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                },
            ],
            temperature: Some(0.5),
            max_tokens: Some(2000),
        };

        let anthropic_request = AnthropicRequest::from(request);
        assert_eq!(
            anthropic_request.system,
            Some("You are helpful".to_string())
        );
        assert_eq!(anthropic_request.messages.len(), 1);
    }

    // =========================================================================
    // Story 97.1: Context Management Tests
    // =========================================================================

    #[test]
    fn test_estimate_tokens() {
        // Approximately 4 characters per token
        let text = "Hello, world!"; // 13 chars = ~3 tokens
        let tokens = LlmClient::estimate_tokens(text);
        assert_eq!(tokens, 3);

        let empty = "";
        assert_eq!(LlmClient::estimate_tokens(empty), 0);

        // Longer text
        let long_text = "a".repeat(1000); // 1000 chars = 250 tokens
        assert_eq!(LlmClient::estimate_tokens(&long_text), 250);
    }

    #[test]
    fn test_get_context_limit() {
        assert_eq!(LlmClient::get_context_limit("gpt-4o"), 128_000);
        assert_eq!(LlmClient::get_context_limit("gpt-4o-mini"), 128_000);
        assert_eq!(LlmClient::get_context_limit("gpt-4-turbo"), 128_000);
        assert_eq!(LlmClient::get_context_limit("claude-3-5-sonnet"), 200_000);
        assert_eq!(LlmClient::get_context_limit("unknown-model"), 32_000);
    }

    #[test]
    fn test_truncate_messages_preserves_system_and_recent() {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "First message".to_string(),
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: "First response".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "Second message".to_string(),
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: "Second response".to_string(),
            },
        ];

        let truncated = LlmClient::truncate_messages_to_fit(&messages, "gpt-4o", 4096);

        // Should preserve system message
        assert_eq!(truncated[0].role, "system");

        // Should include all messages since they fit
        assert_eq!(truncated.len(), 5);
    }

    #[test]
    fn test_truncate_messages_removes_old_when_limit_exceeded() {
        // Create messages that would exceed a very small limit
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "System prompt.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "a".repeat(200000), // Very large message
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: "Response 1".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "Recent message".to_string(),
            },
        ];

        // With a model that has 32000 token limit and we want 4096 for response
        let truncated = LlmClient::truncate_messages_to_fit(&messages, "unknown-model", 4096);

        // Should at least have system message
        assert!(!truncated.is_empty());
        assert_eq!(truncated[0].role, "system");

        // Should have dropped the very large message
        assert!(truncated.len() < messages.len());
    }

    #[test]
    fn test_build_system_prompt_without_config() {
        let base = "You are a building management assistant.";
        let result = LlmClient::build_system_prompt(base, None, "en");

        assert!(result.contains(base));
        assert!(result.contains("IMPORTANT: Always respond in en language."));
    }

    #[test]
    fn test_build_system_prompt_with_tenant_config() {
        let base = "You are a building management assistant.";
        let config = TenantAiConfig {
            personality: Some("friendly and professional".to_string()),
            building_context: Some(
                "This is a 10-story residential building built in 2020.".to_string(),
            ),
            custom_instructions: vec![
                "Always mention the building name 'Sunset Towers'".to_string(),
                "Contact hours are 9am-5pm".to_string(),
            ],
            preferred_language: Some("sk".to_string()),
            escalation_topics: vec!["emergency".to_string()],
        };

        let result = LlmClient::build_system_prompt(base, Some(&config), "sk");

        assert!(result.contains("friendly and professional"));
        assert!(result.contains("10-story residential building"));
        assert!(result.contains("Sunset Towers"));
        assert!(result.contains("9am-5pm"));
        assert!(result.contains("IMPORTANT: Always respond in sk language."));
    }

    // =========================================================================
    // Story 97.2: Embedding Tests
    // =========================================================================

    #[test]
    fn test_embedding_result_structure() {
        let result = EmbeddingResult {
            embedding: vec![0.1, 0.2, 0.3],
            model: "text-embedding-3-small".to_string(),
            tokens_used: 10,
        };

        assert_eq!(result.embedding.len(), 3);
        assert_eq!(result.model, "text-embedding-3-small");
        assert_eq!(result.tokens_used, 10);
    }

    // =========================================================================
    // Story 97.4: Sentiment Analysis Tests
    // =========================================================================

    #[test]
    fn test_sentiment_result_structure() {
        let result = SentimentResult {
            score: 0.8,
            label: "positive".to_string(),
            confidence: 0.95,
            key_phrases: vec!["excellent service".to_string(), "very helpful".to_string()],
            requires_attention: false,
        };

        assert_eq!(result.score, 0.8);
        assert_eq!(result.label, "positive");
        assert_eq!(result.confidence, 0.95);
        assert_eq!(result.key_phrases.len(), 2);
        assert!(!result.requires_attention);
    }

    #[test]
    fn test_batch_sentiment_result_structure() {
        let results = vec![
            SentimentResult {
                score: -0.5,
                label: "negative".to_string(),
                confidence: 0.9,
                key_phrases: vec!["broken elevator".to_string()],
                requires_attention: true,
            },
            SentimentResult {
                score: 0.3,
                label: "neutral".to_string(),
                confidence: 0.85,
                key_phrases: vec!["general inquiry".to_string()],
                requires_attention: false,
            },
            SentimentResult {
                score: 0.8,
                label: "positive".to_string(),
                confidence: 0.95,
                key_phrases: vec!["thank you".to_string()],
                requires_attention: false,
            },
        ];

        let batch = BatchSentimentResult {
            results: results.clone(),
            average_score: 0.2,
            negative_count: 1,
            neutral_count: 1,
            positive_count: 1,
            attention_required_indices: vec![0],
        };

        assert_eq!(batch.results.len(), 3);
        assert_eq!(batch.negative_count, 1);
        assert_eq!(batch.neutral_count, 1);
        assert_eq!(batch.positive_count, 1);
        assert_eq!(batch.attention_required_indices, vec![0]);
    }

    #[test]
    fn test_tenant_ai_config_default() {
        let config = TenantAiConfig::default();

        assert!(config.personality.is_none());
        assert!(config.building_context.is_none());
        assert!(config.custom_instructions.is_empty());
        assert!(config.preferred_language.is_none());
        assert!(config.escalation_topics.is_empty());
    }

    #[test]
    fn test_sentiment_result_serialization() {
        let result = SentimentResult {
            score: -0.7,
            label: "negative".to_string(),
            confidence: 0.88,
            key_phrases: vec!["broken pipe".to_string(), "urgent".to_string()],
            requires_attention: true,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: SentimentResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.score, deserialized.score);
        assert_eq!(result.label, deserialized.label);
        assert_eq!(result.requires_attention, deserialized.requires_attention);
    }
}
