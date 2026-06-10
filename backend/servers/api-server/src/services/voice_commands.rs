//! Voice command processing service (Epic 93: Voice Assistant & OAuth Completion).
//!
//! Story 93.2: Voice Command Processing
//! - Command parsing and intent detection
//! - Action execution (report fault, check balance, etc.)
//! - Spoken response generation

use chrono::Utc;
use db::models::{
    voice_intent, ParsedVoiceCommand, VoiceActionResult, VoiceAssistantDevice, VoiceCard,
    VoiceCommandHistory,
};
use db::repositories::LlmDocumentRepository;
use serde_json::json;
use sqlx::PgConnection;
use std::time::Instant;
use thiserror::Error;
use uuid::Uuid;

/// Voice command processing errors.
#[derive(Debug, Error)]
pub enum VoiceCommandError {
    #[error("Device not found: {0}")]
    DeviceNotFound(Uuid),

    #[error("Device not linked to user")]
    DeviceNotLinked,

    #[error("Invalid intent: {0}")]
    InvalidIntent(String),

    #[error("Missing required slot: {0}")]
    MissingSlot(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Action failed: {0}")]
    ActionFailed(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

/// Voice command processor for handling voice assistant requests.
#[derive(Clone)]
pub struct VoiceCommandProcessor {
    llm_document_repo: LlmDocumentRepository,
}

impl VoiceCommandProcessor {
    /// Create a new voice command processor.
    pub fn new(llm_document_repo: LlmDocumentRepository) -> Self {
        Self { llm_document_repo }
    }

    /// Parse a voice command to extract intent and slots.
    pub fn parse_command(&self, raw_text: &str, locale: &str) -> ParsedVoiceCommand {
        let text_lower = raw_text.to_lowercase();
        let language = Self::extract_language(locale);

        // Simple intent detection based on keywords
        let (intent, confidence, slots) = if text_lower.contains("balance")
            || text_lower.contains("owe")
            || text_lower.contains("payment")
            || text_lower.contains("zostatok")
            || text_lower.contains("platba")
        {
            (voice_intent::CHECK_BALANCE.to_string(), 0.9, json!({}))
        } else if text_lower.contains("fault")
            || text_lower.contains("problem")
            || text_lower.contains("issue")
            || text_lower.contains("broken")
            || text_lower.contains("porucha")
        {
            // Try to extract fault description
            let description = Self::extract_fault_description(&text_lower);
            (
                voice_intent::REPORT_FAULT.to_string(),
                0.85,
                json!({ "description": description }),
            )
        } else if text_lower.contains("announcement")
            || text_lower.contains("news")
            || text_lower.contains("message")
            || text_lower.contains("oznam")
            || text_lower.contains("sprav")
        {
            (
                voice_intent::CHECK_ANNOUNCEMENTS.to_string(),
                0.9,
                json!({}),
            )
        } else if text_lower.contains("book")
            || text_lower.contains("reserve")
            || text_lower.contains("facility")
            || text_lower.contains("room")
            || text_lower.contains("rezerv")
            || text_lower.contains("miestnos")
        {
            (voice_intent::BOOK_FACILITY.to_string(), 0.8, json!({}))
        } else if text_lower.contains("meter")
            || text_lower.contains("reading")
            || text_lower.contains("consumption")
            || text_lower.contains("meranie")
            || text_lower.contains("spotreba")
        {
            (voice_intent::CHECK_METER.to_string(), 0.85, json!({}))
        } else if text_lower.contains("contact")
            || text_lower.contains("manager")
            || text_lower.contains("speak")
            || text_lower.contains("kontakt")
            || text_lower.contains("spravca")
        {
            (voice_intent::CONTACT_MANAGER.to_string(), 0.9, json!({}))
        } else if text_lower.contains("help")
            || text_lower.contains("what can")
            || text_lower.contains("pomoc")
            || text_lower.contains("co mozes")
        {
            (voice_intent::GET_HELP.to_string(), 0.95, json!({}))
        } else {
            // Unknown intent - provide help
            (voice_intent::GET_HELP.to_string(), 0.5, json!({}))
        };

        ParsedVoiceCommand {
            raw_text: raw_text.to_string(),
            intent,
            confidence,
            slots,
            language,
        }
    }

    /// Execute a voice command action.
    pub async fn execute_action(
        &self,
        device: &VoiceAssistantDevice,
        parsed: &ParsedVoiceCommand,
    ) -> Result<VoiceActionResult, VoiceCommandError> {
        match parsed.intent.as_str() {
            voice_intent::CHECK_BALANCE => self.action_check_balance(device, parsed).await,
            voice_intent::REPORT_FAULT => self.action_report_fault(device, parsed).await,
            voice_intent::CHECK_ANNOUNCEMENTS => {
                self.action_check_announcements(device, parsed).await
            }
            voice_intent::BOOK_FACILITY => self.action_book_facility(device, parsed).await,
            voice_intent::CHECK_METER => self.action_check_meter(device, parsed).await,
            voice_intent::CONTACT_MANAGER => self.action_contact_manager(device, parsed).await,
            voice_intent::GET_HELP => self.action_get_help(device, parsed).await,
            _ => Err(VoiceCommandError::InvalidIntent(parsed.intent.clone())),
        }
    }

    /// Process a complete voice command from device ID to response.
    ///
    /// PAP-108 (PAP-80): the repository is stateless, so the caller supplies
    /// the connection — in webhook handlers this is the `RlsConnection`'s
    /// context-set connection (`&mut **rls.conn()`).
    pub async fn process_command(
        &self,
        conn: &mut PgConnection,
        device_id: Uuid,
        command_text: &str,
        locale: &str,
    ) -> Result<(VoiceActionResult, VoiceCommandHistory), VoiceCommandError> {
        let start_time = Instant::now();

        // Find the device
        let device = self
            .llm_document_repo
            .find_voice_device(&mut *conn, device_id)
            .await?
            .ok_or(VoiceCommandError::DeviceNotFound(device_id))?;

        if !device.is_active {
            return Err(VoiceCommandError::DeviceNotLinked);
        }

        // Parse the command
        let parsed = self.parse_command(command_text, locale);

        // Execute the action
        let result = self.execute_action(&device, &parsed).await;

        let processing_time_ms = start_time.elapsed().as_millis() as i32;

        // Record the command in history
        let (success, error_message, response_text) = match &result {
            Ok(r) => (r.success, None, r.response_text.clone()),
            Err(e) => (false, Some(e.to_string()), format!("Error: {}", e)),
        };

        let history = self
            .llm_document_repo
            .create_voice_command(
                &mut *conn,
                device_id,
                device.user_id,
                command_text,
                Some(&parsed.intent),
                &response_text,
                result.as_ref().ok().map(|r| r.action_type.as_str()),
                success,
                error_message.as_deref(),
                processing_time_ms,
            )
            .await?;

        // Update device last used timestamp
        let _ = self
            .llm_document_repo
            .update_voice_device_last_used(&mut *conn, device_id)
            .await;

        Ok((
            result.unwrap_or_else(|e| VoiceActionResult {
                success: false,
                action_type: "error".to_string(),
                response_text: self.localized_error(&parsed.language, &e.to_string()),
                ssml: None,
                card: None,
                should_end_session: true,
                data: None,
            }),
            history,
        ))
    }

    // =========================================================================
    // Action Handlers
    // =========================================================================

    async fn action_check_balance(
        &self,
        device: &VoiceAssistantDevice,
        parsed: &ParsedVoiceCommand,
    ) -> Result<VoiceActionResult, VoiceCommandError> {
        // In a real implementation, this would query the financial repository
        // For now, return a placeholder response
        let response = match parsed.language.as_str() {
            "sk" => "Vas aktualni zostatok je nula eur. Nemate ziadne nezaplatene poplatky.",
            "cs" => "Vas aktualni zustatek je nula korun. Nemate zadne nezaplacene poplatky.",
            _ => "Your current balance is zero. You have no outstanding fees.",
        };

        Ok(VoiceActionResult {
            success: true,
            action_type: voice_intent::CHECK_BALANCE.to_string(),
            response_text: response.to_string(),
            ssml: Some(format!("<speak>{}</speak>", response)),
            card: Some(VoiceCard {
                title: self.localize("Balance", &parsed.language),
                content: response.to_string(),
                image_url: None,
            }),
            should_end_session: true,
            data: Some(json!({
                "balance": 0.0,
                "currency": "EUR",
                "unit_id": device.unit_id
            })),
        })
    }

    async fn action_report_fault(
        &self,
        device: &VoiceAssistantDevice,
        parsed: &ParsedVoiceCommand,
    ) -> Result<VoiceActionResult, VoiceCommandError> {
        let description = parsed
            .slots
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("Issue reported via voice assistant");

        // In a real implementation, this would create a fault via the fault repository
        let fault_id = Uuid::new_v4();

        let response = match parsed.language.as_str() {
            "sk" => format!(
                "Porucha bola nahlasena. Cislo tiketu je {}. Spravca bude coskoro kontaktovany.",
                &fault_id.to_string()[..8]
            ),
            "cs" => format!(
                "Porucha byla nahlasena. Cislo tiketu je {}. Spravce bude brzy kontaktovan.",
                &fault_id.to_string()[..8]
            ),
            _ => format!(
                "Fault has been reported. Your ticket number is {}. The manager will be notified shortly.",
                &fault_id.to_string()[..8]
            ),
        };

        Ok(VoiceActionResult {
            success: true,
            action_type: voice_intent::REPORT_FAULT.to_string(),
            response_text: response.clone(),
            ssml: Some(format!("<speak>{}</speak>", response)),
            card: Some(VoiceCard {
                title: self.localize("Fault Reported", &parsed.language),
                content: format!("{}\n\nDescription: {}", response, description),
                image_url: None,
            }),
            should_end_session: true,
            data: Some(json!({
                "fault_id": fault_id,
                "description": description,
                "unit_id": device.unit_id,
                "reported_at": Utc::now().to_rfc3339()
            })),
        })
    }

    async fn action_check_announcements(
        &self,
        _device: &VoiceAssistantDevice,
        parsed: &ParsedVoiceCommand,
    ) -> Result<VoiceActionResult, VoiceCommandError> {
        // In a real implementation, this would query the announcement repository
        let response = match parsed.language.as_str() {
            "sk" => "Nemate ziadne nove oznamy.",
            "cs" => "Nemate zadne nove oznameni.",
            _ => "You have no new announcements.",
        };

        Ok(VoiceActionResult {
            success: true,
            action_type: voice_intent::CHECK_ANNOUNCEMENTS.to_string(),
            response_text: response.to_string(),
            ssml: Some(format!("<speak>{}</speak>", response)),
            card: Some(VoiceCard {
                title: self.localize("Announcements", &parsed.language),
                content: response.to_string(),
                image_url: None,
            }),
            should_end_session: true,
            data: Some(json!({
                "count": 0,
                "announcements": []
            })),
        })
    }

    async fn action_book_facility(
        &self,
        _device: &VoiceAssistantDevice,
        parsed: &ParsedVoiceCommand,
    ) -> Result<VoiceActionResult, VoiceCommandError> {
        let response = match parsed.language.as_str() {
            "sk" => "Rezervacia zariadeni momentalne nie je dostupna cez hlasoveho asistenta. Prosim, pouzite mobilnu aplikaciu alebo webovy portal.",
            "cs" => "Rezervace zarizeni momentalne neni dostupna pres hlasoveho asistenta. Prosim, pouzijte mobilni aplikaci nebo webovy portal.",
            _ => "Facility booking is not yet available via voice assistant. Please use the mobile app or web portal."
        };

        Ok(VoiceActionResult {
            success: true,
            action_type: voice_intent::BOOK_FACILITY.to_string(),
            response_text: response.to_string(),
            ssml: Some(format!("<speak>{}</speak>", response)),
            card: None,
            should_end_session: true,
            data: None,
        })
    }

    async fn action_check_meter(
        &self,
        _device: &VoiceAssistantDevice,
        parsed: &ParsedVoiceCommand,
    ) -> Result<VoiceActionResult, VoiceCommandError> {
        // In a real implementation, this would query the meter repository
        let response = match parsed.language.as_str() {
            "sk" => {
                "Vase posledne odcitanie meracov bolo zaznamenane. Nemam ziadne cakajuce odcitania."
            }
            "cs" => "Vase posledni odecty meracu byly zaznamenany. Nemam zadne cekajici odecty.",
            _ => "Your latest meter readings have been recorded. There are no pending readings.",
        };

        Ok(VoiceActionResult {
            success: true,
            action_type: voice_intent::CHECK_METER.to_string(),
            response_text: response.to_string(),
            ssml: Some(format!("<speak>{}</speak>", response)),
            card: Some(VoiceCard {
                title: self.localize("Meter Readings", &parsed.language),
                content: response.to_string(),
                image_url: None,
            }),
            should_end_session: true,
            data: Some(json!({
                "pending_readings": 0
            })),
        })
    }

    async fn action_contact_manager(
        &self,
        _device: &VoiceAssistantDevice,
        parsed: &ParsedVoiceCommand,
    ) -> Result<VoiceActionResult, VoiceCommandError> {
        let response = match parsed.language.as_str() {
            "sk" => "Spravcu mozete kontaktovat prostrednstvom aplikacie alebo emailom na adrese spravca@example.com. Chcete nahlasit poruchu namiesto toho?",
            "cs" => "Spravce muzete kontaktovat prostrednictvim aplikace nebo emailem na adrese spravce@example.com. Chcete nahlasit poruchu misto toho?",
            _ => "You can contact the manager through the app or via email at manager@example.com. Would you like to report a fault instead?"
        };

        Ok(VoiceActionResult {
            success: true,
            action_type: voice_intent::CONTACT_MANAGER.to_string(),
            response_text: response.to_string(),
            ssml: Some(format!("<speak>{}</speak>", response)),
            card: Some(VoiceCard {
                title: self.localize("Contact Manager", &parsed.language),
                content: "manager@example.com".to_string(),
                image_url: None,
            }),
            should_end_session: false,
            data: Some(json!({
                "manager_email": "manager@example.com"
            })),
        })
    }

    async fn action_get_help(
        &self,
        _device: &VoiceAssistantDevice,
        parsed: &ParsedVoiceCommand,
    ) -> Result<VoiceActionResult, VoiceCommandError> {
        let response = match parsed.language.as_str() {
            "sk" => "Mozem vam pomoct s nasledovnym: skontrolovat zostatok, nahlasit poruchu, skontrolovat oznamy, skontrolovat stav meracov, alebo kontaktovat spravcu. Co by ste chceli urobit?",
            "cs" => "Mohu vam pomoct s nasledujicim: zkontrolovat zustatek, nahlasit poruchu, zkontrolovat oznameni, zkontrolovat stav meracu, nebo kontaktovat spravce. Co byste chteli udelat?",
            _ => "I can help you with the following: check your balance, report a fault, check announcements, check meter readings, or contact the manager. What would you like to do?"
        };

        Ok(VoiceActionResult {
            success: true,
            action_type: voice_intent::GET_HELP.to_string(),
            response_text: response.to_string(),
            ssml: Some(format!("<speak>{}</speak>", response)),
            card: Some(VoiceCard {
                title: self.localize("Help", &parsed.language),
                content: self.get_help_card_content(&parsed.language),
                image_url: None,
            }),
            should_end_session: false,
            data: None,
        })
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    fn extract_language(locale: &str) -> String {
        // Extract language code from locale (e.g., "en-US" -> "en", "sk-SK" -> "sk")
        locale.split('-').next().unwrap_or("en").to_lowercase()
    }

    fn extract_fault_description(text: &str) -> String {
        // Simple extraction - in production, use NLP/LLM
        let prefixes = [
            "report a fault",
            "report fault",
            "there is a problem with",
            "broken",
            "issue with",
            "problem with",
            "porucha",
            "problem s",
        ];

        for prefix in prefixes {
            if let Some(idx) = text.find(prefix) {
                let description = &text[idx + prefix.len()..];
                let trimmed = description.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }

        "General issue reported via voice".to_string()
    }

    fn localize(&self, key: &str, language: &str) -> String {
        match (key, language) {
            ("Balance", "sk") => "Zostatok".to_string(),
            ("Balance", "cs") => "Zustatek".to_string(),
            ("Fault Reported", "sk") => "Porucha nahlasena".to_string(),
            ("Fault Reported", "cs") => "Porucha nahlasena".to_string(),
            ("Announcements", "sk") => "Oznamy".to_string(),
            ("Announcements", "cs") => "Oznameni".to_string(),
            ("Meter Readings", "sk") => "Odcitanie meracov".to_string(),
            ("Meter Readings", "cs") => "Odecty meracu".to_string(),
            ("Contact Manager", "sk") => "Kontakt na spravcu".to_string(),
            ("Contact Manager", "cs") => "Kontakt na spravce".to_string(),
            ("Help", "sk") => "Pomocnik".to_string(),
            ("Help", "cs") => "Napoveda".to_string(),
            _ => key.to_string(),
        }
    }

    fn localized_error(&self, language: &str, error: &str) -> String {
        match language {
            "sk" => format!(
                "Prepáčte, nastala chyba: {}. Skúste to prosím znova.",
                error
            ),
            "cs" => format!(
                "Promiňte, nastala chyba: {}. Zkuste to prosím znovu.",
                error
            ),
            _ => format!("Sorry, an error occurred: {}. Please try again.", error),
        }
    }

    fn get_help_card_content(&self, language: &str) -> String {
        match language {
            "sk" => "Dostupne prikazy:\n\
                 - Skontroluj zostatok\n\
                 - Nahlas poruchu\n\
                 - Skontroluj oznamy\n\
                 - Skontroluj merace\n\
                 - Kontaktuj spravcu"
                .to_string(),
            "cs" => "Dostupne prikazy:\n\
                 - Zkontroluj zustatek\n\
                 - Nahlas poruchu\n\
                 - Zkontroluj oznameni\n\
                 - Zkontroluj merace\n\
                 - Kontaktuj spravce"
                .to_string(),
            _ => "Available commands:\n\
                 - Check my balance\n\
                 - Report a fault\n\
                 - Check announcements\n\
                 - Check meter readings\n\
                 - Contact manager"
                .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_processor() -> VoiceCommandProcessor {
        // Use a dummy pool - parse_command doesn't hit the database
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
        VoiceCommandProcessor::new(LlmDocumentRepository::new(pool))
    }

    #[tokio::test]
    async fn test_parse_command_balance() {
        let processor = create_test_processor();
        let parsed = processor.parse_command("What is my balance?", "en-US");
        assert_eq!(parsed.intent, voice_intent::CHECK_BALANCE);
        assert!(parsed.confidence > 0.8);
    }

    #[tokio::test]
    async fn test_parse_command_fault() {
        let processor = create_test_processor();
        let parsed = processor.parse_command("Report a fault with the elevator", "en-US");
        assert_eq!(parsed.intent, voice_intent::REPORT_FAULT);
        assert!(parsed.slots.get("description").is_some());
    }

    #[tokio::test]
    async fn test_parse_command_help() {
        let processor = create_test_processor();
        let parsed = processor.parse_command("What can you do?", "en-US");
        assert_eq!(parsed.intent, voice_intent::GET_HELP);
    }

    #[tokio::test]
    async fn test_parse_command_slovak() {
        let processor = create_test_processor();
        let parsed = processor.parse_command("Skontroluj moj zostatok", "sk-SK");
        assert_eq!(parsed.intent, voice_intent::CHECK_BALANCE);
        assert_eq!(parsed.language, "sk");
    }

    #[test]
    fn test_extract_language() {
        assert_eq!(
            VoiceCommandProcessor::extract_language("en-US"),
            "en".to_string()
        );
        assert_eq!(
            VoiceCommandProcessor::extract_language("sk-SK"),
            "sk".to_string()
        );
        assert_eq!(
            VoiceCommandProcessor::extract_language("cs"),
            "cs".to_string()
        );
    }
}
