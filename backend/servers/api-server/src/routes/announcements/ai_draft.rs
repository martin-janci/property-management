//! Epic 92.4: AI announcement draft generation route.

use super::shared::*;
use crate::state::AppState;
use api_core::{AuthUser, TenantExtractor};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use common::errors::ErrorResponse;

/// AI draft sub-router (Epic 92.4 draft generation).
pub(super) fn router() -> Router<AppState> {
    Router::new().route("/ai-draft", post(generate_ai_draft))
}

/// Generate AI-powered announcement drafts (Story 92.4).
///
/// Uses LLM to generate well-structured announcement drafts with appropriate tone.
/// Requires manager-level role.
#[utoipa::path(
    post,
    path = "/api/v1/announcements/ai-draft",
    request_body = GenerateAiDraftRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Drafts generated", body = GenerateAiDraftResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn generate_ai_draft(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Json(req): Json<GenerateAiDraftRequest>,
) -> Result<Json<GenerateAiDraftResponse>, (StatusCode, Json<ErrorResponse>)> {
    use std::time::Instant;

    // Authorization: require manager-level role
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can generate announcement drafts",
            )),
        ));
    }

    let start_time = Instant::now();

    // Validate topic
    if req.topic.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Topic is required")),
        ));
    }

    if req.topic.len() > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Topic exceeds maximum length of 500 characters",
            )),
        ));
    }

    // Determine provider and model
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider.as_str() {
        "openai" => "gpt-4o".to_string(),
        "azure_openai" => "gpt-4o".to_string(),
        _ => "claude-3-5-sonnet-20241022".to_string(),
    });

    let num_drafts = req.num_drafts.unwrap_or(1).clamp(1, 3);
    let urgency = req.urgency.as_deref().unwrap_or("medium");
    let tone = req.tone.as_deref().unwrap_or("professional");

    // Build system prompt
    let system_prompt = build_announcement_system_prompt(urgency, tone, &req.language);

    // Build user prompt
    let key_points_text = req
        .key_points
        .as_ref()
        .filter(|k| !k.is_empty())
        .map(|points| {
            format!(
                "\n\nKey points to include:\n{}",
                points
                    .iter()
                    .map(|p| format!("- {}", p))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
        .unwrap_or_default();

    let audience_text = req
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
        num_drafts, req.topic, key_points_text, audience_text, num_drafts
    );

    // Call LLM
    let llm_request = integrations::ChatCompletionRequest {
        model: model.clone(),
        messages: vec![
            integrations::ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            integrations::ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ],
        temperature: Some(0.7), // Higher temperature for creative variety
        max_tokens: Some(4000),
    };

    let response = state
        .llm_client
        .chat(&provider, &llm_request)
        .await
        .map_err(|e| {
            tracing::error!("LLM announcement generation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "LLM_ERROR",
                    format!("Failed to generate drafts: {}", e),
                )),
            )
        })?;

    let response_content = response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    // Parse the drafts
    let drafts = parse_announcement_drafts(&response_content);

    let generation_time_ms = start_time.elapsed().as_millis() as u64;

    tracing::info!(
        "Generated {} announcement drafts for topic '{}' (tokens: {}, latency: {}ms)",
        drafts.len(),
        req.topic,
        response.usage.total_tokens,
        generation_time_ms
    );

    Ok(Json(GenerateAiDraftResponse {
        drafts,
        tokens_used: response.usage.total_tokens,
        generation_time_ms,
        provider,
        model,
    }))
}

/// Build the system prompt for announcement generation.
fn build_announcement_system_prompt(urgency: &str, tone: &str, language: &str) -> String {
    let urgency_instruction = match urgency {
        "critical" => "This is a CRITICAL announcement. Use urgent language, emphasize importance, and include clear action items with deadlines.",
        "high" => "This is a high-priority announcement. Use clear, direct language and emphasize the importance of timely action.",
        "low" => "This is a routine announcement. Use a relaxed, informative tone.",
        _ => "This is a standard announcement. Use clear, professional language.",
    };

    let tone_instruction = match tone {
        "formal" => "Use formal, official language appropriate for legal or regulatory matters.",
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

    format!(
        r#"You are an expert property management communications specialist.

Your task is to create professional announcements for building residents and owners.

Language: Write in {} language
Urgency Level: {}
Tone: {}

Guidelines:
1. Write clear, well-structured announcements
2. Use appropriate formatting (paragraphs, bullet points where helpful)
3. Include relevant dates, times, and locations when applicable
4. Provide clear contact information or next steps
5. Be culturally appropriate for Central European readers
6. Avoid jargon and use accessible language

The announcements should be ready for immediate publication after minimal editing."#,
        language_name, urgency_instruction, tone_instruction
    )
}

/// Parse announcement drafts from LLM response.
fn parse_announcement_drafts(content: &str) -> Vec<AnnouncementDraft> {
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
                draft.title = trimmed
                    .strip_prefix("TITLE:")
                    .or_else(|| trimmed.strip_prefix("Title:"))
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
                draft.suggested_target = trimmed
                    .strip_prefix("TARGET:")
                    .or_else(|| trimmed.strip_prefix("Target:"))
                    .unwrap_or(trimmed)
                    .trim()
                    .to_lowercase();
                current_section = "target";
                current_content.clear();
            } else if trimmed.to_uppercase().starts_with("TONE:") {
                if current_section == "content" {
                    draft.content = current_content.trim().to_string();
                }
                draft.tone_analysis = trimmed
                    .strip_prefix("TONE:")
                    .or_else(|| trimmed.strip_prefix("Tone:"))
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
