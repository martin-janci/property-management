//! Provider-specific request parsing and response building for the voice
//! webhook handlers.
//!
//! Behaviour-preserving extraction from the monolithic `voice_webhooks.rs`.
//! These functions translate between the neutral [`VoiceActionResult`] produced
//! by the command processor and each platform's wire shape:
//!
//! - **Alexa** — map built-in/custom intents to command text
//!   (`extract_alexa_command_text`) and build skill / account-link responses.
//! - **Google** — build Actions responses and the account-link prompt.

use db::models::{
    AlexaCard, AlexaIntent, AlexaOutputSpeech, AlexaResponseBody, AlexaSkillResponse,
    GoogleActionsResponse, GoogleContent, GooglePrompt, GoogleSceneResponse, GoogleSessionResponse,
    GoogleSimpleResponse, VoiceActionResult,
};

/// Extract command text from Alexa intent.
pub(crate) fn extract_alexa_command_text(intent: &AlexaIntent) -> String {
    // Map Alexa built-in intents to our commands
    match intent.name.as_str() {
        "AMAZON.HelpIntent" => "help".to_string(),
        "AMAZON.StopIntent" | "AMAZON.CancelIntent" => "goodbye".to_string(),
        "CheckBalanceIntent" => "check my balance".to_string(),
        "ReportFaultIntent" => {
            // Extract fault description from slots
            let description = intent
                .slots
                .as_ref()
                .and_then(|s| s.get("FaultDescription"))
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_str())
                .unwrap_or("a fault");
            format!("report a fault with {}", description)
        }
        "CheckAnnouncementsIntent" => "check announcements".to_string(),
        "CheckMeterIntent" => "check meter readings".to_string(),
        "ContactManagerIntent" => "contact manager".to_string(),
        _ => intent.name.clone(),
    }
}

/// Build Alexa skill response from action result.
pub(crate) fn build_alexa_response(result: &VoiceActionResult) -> AlexaSkillResponse {
    let output_speech = if let Some(ssml) = &result.ssml {
        AlexaOutputSpeech {
            speech_type: "SSML".to_string(),
            text: None,
            ssml: Some(ssml.clone()),
        }
    } else {
        AlexaOutputSpeech {
            speech_type: "PlainText".to_string(),
            text: Some(result.response_text.clone()),
            ssml: None,
        }
    };

    let card = result.card.as_ref().map(|c| AlexaCard {
        card_type: "Simple".to_string(),
        title: c.title.clone(),
        content: Some(c.content.clone()),
        text: None,
    });

    AlexaSkillResponse {
        version: "1.0".to_string(),
        response: AlexaResponseBody {
            output_speech,
            card,
            should_end_session: result.should_end_session,
        },
    }
}

/// Build Alexa response for account linking.
pub(crate) fn build_alexa_link_account_response() -> AlexaSkillResponse {
    AlexaSkillResponse {
        version: "1.0".to_string(),
        response: AlexaResponseBody {
            output_speech: AlexaOutputSpeech {
                speech_type: "PlainText".to_string(),
                text: Some(
                    "Please link your property management account in the Alexa app to use this skill."
                        .to_string(),
                ),
                ssml: None,
            },
            card: Some(AlexaCard {
                card_type: "LinkAccount".to_string(),
                title: "Link Account".to_string(),
                content: None,
                text: None,
            }),
            should_end_session: true,
        },
    }
}

/// Build Google Actions response from action result.
pub(crate) fn build_google_response(
    session_id: &str,
    result: &VoiceActionResult,
) -> GoogleActionsResponse {
    let content = result.card.as_ref().map(|c| GoogleContent {
        card: Some(db::models::GoogleCard {
            title: c.title.clone(),
            subtitle: None,
            text: c.content.clone(),
            image: None,
        }),
    });

    GoogleActionsResponse {
        session: GoogleSessionResponse {
            id: session_id.to_string(),
            params: None,
        },
        prompt: GooglePrompt {
            override_mode: false,
            first_simple: GoogleSimpleResponse {
                speech: result.response_text.clone(),
                text: Some(result.response_text.clone()),
            },
            content,
        },
        scene: if result.should_end_session {
            Some(GoogleSceneResponse {
                name: "actions.scene.END_CONVERSATION".to_string(),
            })
        } else {
            None
        },
    }
}

/// Build Google response for account linking.
pub(crate) fn build_google_link_account_response(session_id: &str) -> GoogleActionsResponse {
    GoogleActionsResponse {
        session: GoogleSessionResponse {
            id: session_id.to_string(),
            params: None,
        },
        prompt: GooglePrompt {
            override_mode: false,
            first_simple: GoogleSimpleResponse {
                speech: "Please link your property management account to use this action."
                    .to_string(),
                text: Some(
                    "Please link your property management account to use this action.".to_string(),
                ),
            },
            content: None,
        },
        scene: Some(GoogleSceneResponse {
            name: "AccountLinking".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- helpers ---------------------------------------------------------

    fn action_result(text: &str, ssml: Option<&str>, end: bool) -> VoiceActionResult {
        VoiceActionResult {
            success: true,
            action_type: "test".to_string(),
            response_text: text.to_string(),
            ssml: ssml.map(|s| s.to_string()),
            card: None,
            should_end_session: end,
            data: None,
        }
    }

    fn action_result_with_card(text: &str) -> VoiceActionResult {
        VoiceActionResult {
            success: true,
            action_type: "test".to_string(),
            response_text: text.to_string(),
            ssml: None,
            card: Some(db::models::VoiceCard {
                title: "Balance".to_string(),
                content: "You owe 12 EUR".to_string(),
                image_url: None,
            }),
            should_end_session: false,
            data: None,
        }
    }

    // --- extract_alexa_command_text -------------------------------------

    fn intent(name: &str, slots: Option<serde_json::Value>) -> AlexaIntent {
        AlexaIntent {
            name: name.to_string(),
            slots,
        }
    }

    #[test]
    fn command_text_maps_builtin_intents() {
        assert_eq!(
            extract_alexa_command_text(&intent("AMAZON.HelpIntent", None)),
            "help"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("AMAZON.StopIntent", None)),
            "goodbye"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("AMAZON.CancelIntent", None)),
            "goodbye"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("CheckBalanceIntent", None)),
            "check my balance"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("CheckAnnouncementsIntent", None)),
            "check announcements"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("CheckMeterIntent", None)),
            "check meter readings"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("ContactManagerIntent", None)),
            "contact manager"
        );
    }

    #[test]
    fn command_text_extracts_fault_description_slot() {
        let slots = serde_json::json!({
            "FaultDescription": { "value": "broken heating" }
        });
        assert_eq!(
            extract_alexa_command_text(&intent("ReportFaultIntent", Some(slots))),
            "report a fault with broken heating"
        );
    }

    #[test]
    fn command_text_fault_defaults_without_slot() {
        assert_eq!(
            extract_alexa_command_text(&intent("ReportFaultIntent", None)),
            "report a fault with a fault"
        );
    }

    #[test]
    fn command_text_passes_through_unknown_intent() {
        assert_eq!(
            extract_alexa_command_text(&intent("MyCustomIntent", None)),
            "MyCustomIntent"
        );
    }

    // --- Alexa response builders ----------------------------------------

    #[test]
    fn build_alexa_response_plaintext() {
        let r = build_alexa_response(&action_result("hello", None, true));
        assert_eq!(r.response.output_speech.speech_type, "PlainText");
        assert_eq!(r.response.output_speech.text.as_deref(), Some("hello"));
        assert!(r.response.output_speech.ssml.is_none());
        assert!(r.response.should_end_session);
        assert!(r.response.card.is_none());
    }

    #[test]
    fn build_alexa_response_ssml() {
        let r = build_alexa_response(&action_result("hi", Some("<speak>hi</speak>"), false));
        assert_eq!(r.response.output_speech.speech_type, "SSML");
        assert_eq!(
            r.response.output_speech.ssml.as_deref(),
            Some("<speak>hi</speak>")
        );
        assert!(r.response.output_speech.text.is_none());
    }

    #[test]
    fn build_alexa_response_with_card() {
        let r = build_alexa_response(&action_result_with_card("your balance"));
        let card = r.response.card.expect("card present");
        assert_eq!(card.card_type, "Simple");
        assert_eq!(card.title, "Balance");
        assert_eq!(card.content.as_deref(), Some("You owe 12 EUR"));
    }

    #[test]
    fn build_alexa_link_account_response_shape() {
        let r = build_alexa_link_account_response();
        let card = r.response.card.expect("link card present");
        assert_eq!(card.card_type, "LinkAccount");
        assert!(r.response.should_end_session);
    }

    // --- Google response builders ---------------------------------------

    #[test]
    fn build_google_response_ends_conversation() {
        let r = build_google_response("sess-1", &action_result("done", None, true));
        assert_eq!(r.session.id, "sess-1");
        assert_eq!(r.prompt.first_simple.speech, "done");
        let scene = r.scene.expect("end scene present");
        assert_eq!(scene.name, "actions.scene.END_CONVERSATION");
    }

    #[test]
    fn build_google_response_continues_without_scene() {
        let r = build_google_response("sess-2", &action_result_with_card("balance"));
        assert!(r.scene.is_none());
        let content = r.prompt.content.expect("content present");
        let card = content.card.expect("card present");
        assert_eq!(card.title, "Balance");
    }

    #[test]
    fn build_google_link_account_response_shape() {
        let r = build_google_link_account_response("sess-3");
        assert_eq!(r.session.id, "sess-3");
        let scene = r.scene.expect("link scene present");
        assert_eq!(scene.name, "AccountLinking");
    }
}
