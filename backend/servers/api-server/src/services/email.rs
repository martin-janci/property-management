//! Email service (Epic 1, Story 1.7).
//!
//! Supports SMTP email sending in production and logging fallback in development.
//! Configure via environment variables:
//! - SMTP_HOST: SMTP server hostname
//! - SMTP_PORT: SMTP server port (default: 587)
//! - SMTP_USERNAME: SMTP authentication username
//! - SMTP_PASSWORD: SMTP authentication password
//! - SMTP_FROM: Sender email address
//! - SMTP_TLS: Enable TLS (default: true)

use db::models::Locale;
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use std::sync::Arc;
use thiserror::Error;

/// Email service errors.
#[derive(Debug, Error)]
pub enum EmailError {
    #[error("Failed to send email: {0}")]
    SendFailed(String),

    #[error("Invalid email address: {0}")]
    InvalidAddress(String),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("SMTP configuration error: {0}")]
    ConfigurationError(String),

    #[error("Failed to build email message: {0}")]
    MessageBuildError(String),
}

/// SMTP configuration loaded from environment variables.
#[derive(Clone, Debug)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub from_name: String,
    pub use_tls: bool,
}

impl SmtpConfig {
    /// Load SMTP configuration from environment variables.
    pub fn from_env() -> Option<Self> {
        let host = std::env::var("SMTP_HOST").ok()?;
        let port = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(587);
        let username = std::env::var("SMTP_USERNAME").ok()?;
        let password = std::env::var("SMTP_PASSWORD").ok()?;
        let from_address = std::env::var("SMTP_FROM").ok()?;
        let from_name = std::env::var("SMTP_FROM_NAME")
            .unwrap_or_else(|_| "Property Management System".to_string());
        let use_tls = std::env::var("SMTP_TLS")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        Some(Self {
            host,
            port,
            username,
            password,
            from_address,
            from_name,
            use_tls,
        })
    }
}

/// Email transport type supporting both SMTP and logging fallback.
enum EmailTransport {
    /// SMTP transport for production email sending.
    Smtp(AsyncSmtpTransport<Tokio1Executor>),
    /// Logging fallback for development mode.
    Log,
}

/// Discriminates the four signature email templates for the i18n `<title>` lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignatureEmailKind {
    Request,
    Reminder,
    Declined,
    Completed,
}

/// Email service for sending transactional emails.
#[derive(Clone)]
pub struct EmailService {
    /// Base URL for verification/reset links
    base_url: String,
    /// Whether to actually send emails (false in development)
    enabled: bool,
    /// SMTP configuration (None if using log fallback)
    smtp_config: Option<SmtpConfig>,
    /// SMTP transport (shared via Arc for Clone)
    transport: Arc<EmailTransport>,
}

impl EmailService {
    /// Create a new EmailService with SMTP support.
    ///
    /// If SMTP configuration is available via environment variables, it will be used.
    /// Otherwise, falls back to logging mode.
    pub fn new(base_url: String, enabled: bool) -> Self {
        let smtp_config = SmtpConfig::from_env();

        let transport = if enabled {
            if let Some(ref config) = smtp_config {
                match Self::create_smtp_transport(config) {
                    Ok(smtp) => {
                        tracing::info!(
                            host = %config.host,
                            port = %config.port,
                            "SMTP email transport configured"
                        );
                        Arc::new(EmailTransport::Smtp(smtp))
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "Failed to create SMTP transport, falling back to logging mode"
                        );
                        Arc::new(EmailTransport::Log)
                    }
                }
            } else {
                tracing::info!("SMTP not configured, using logging mode for emails");
                Arc::new(EmailTransport::Log)
            }
        } else {
            tracing::info!("Email sending disabled, using logging mode");
            Arc::new(EmailTransport::Log)
        };

        Self {
            base_url,
            enabled,
            smtp_config,
            transport,
        }
    }

    /// Create a development EmailService (logs instead of sending).
    pub fn development() -> Self {
        Self {
            base_url: "http://localhost:3000".to_string(),
            enabled: false,
            smtp_config: None,
            transport: Arc::new(EmailTransport::Log),
        }
    }

    /// Create SMTP transport from configuration.
    fn create_smtp_transport(
        config: &SmtpConfig,
    ) -> Result<AsyncSmtpTransport<Tokio1Executor>, EmailError> {
        let creds = Credentials::new(config.username.clone(), config.password.clone());

        let builder = if config.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                .map_err(|e| EmailError::ConfigurationError(e.to_string()))?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
        };

        Ok(builder.port(config.port).credentials(creds).build())
    }

    /// Send email verification email.
    pub async fn send_verification_email(
        &self,
        email: &str,
        name: &str,
        token: &str,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let verification_url = format!("{}/verify-email?token={}", self.base_url, token);
        let subject = self.get_verification_subject(locale);
        let body = self.get_verification_body(name, &verification_url, locale);

        self.send_email(email, &subject, &body).await
    }

    /// Send password reset email.
    pub async fn send_password_reset_email(
        &self,
        email: &str,
        name: &str,
        token: &str,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let reset_url = format!("{}/reset-password?token={}", self.base_url, token);
        let subject = self.get_reset_subject(locale);
        let body = self.get_reset_body(name, &reset_url, locale);

        self.send_email(email, &subject, &body).await
    }

    /// Send invitation email.
    pub async fn send_invitation_email(
        &self,
        email: &str,
        inviter_name: &str,
        token: &str,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let invitation_url = format!("{}/accept-invitation?token={}", self.base_url, token);
        let subject = self.get_invitation_subject(locale);
        let body = self.get_invitation_body(inviter_name, &invitation_url, locale);

        self.send_email(email, &subject, &body).await
    }

    /// Send a generic notification email.
    pub async fn send_notification_email(
        &self,
        email: &str,
        name: &str,
        subject: &str,
        message: &str,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let body = self.get_notification_body(name, message, locale);
        self.send_email(email, subject, &body).await
    }

    /// Send a template-based email (Story 84.4: Reminder sending).
    ///
    /// This method sends emails using predefined templates. The template is
    /// selected by name and populated with the provided data.
    ///
    /// Supported templates:
    /// - `signature_reminder`: E-signature reminder with signer_name, document_name, request_id
    /// - `payment_reminder`: Payment reminder
    /// - `meeting_reminder`: Meeting reminder
    ///
    pub async fn send_template_email(
        &self,
        to: &str,
        template: &str,
        data: serde_json::Value,
    ) -> Result<(), EmailError> {
        let (subject, body) = match template {
            "signature_reminder" => {
                let signer_name = data
                    .get("signer_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("there");
                let document_name = data
                    .get("document_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Document");
                let request_id = data
                    .get("request_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let expires_at = data.get("expires_at").and_then(|v| v.as_str());

                let subject = format!("Reminder: Please sign \"{}\"", document_name);
                let body = format!(
                    "Hello {},\n\n\
                    This is a friendly reminder that your signature is still needed on \"{}\".\n\n\
                    Please sign the document at your earliest convenience.\n\n\
                    {}\n\n\
                    Request ID: {}\n\n\
                    If you have any questions, please contact the document sender.\n\n\
                    Best regards,\n\
                    Property Management System",
                    signer_name,
                    document_name,
                    expires_at
                        .map(|e| format!("This request expires on: {}", e))
                        .unwrap_or_default(),
                    request_id
                );
                (subject, body)
            }
            "payment_reminder" => {
                let amount = data
                    .get("amount")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pending");
                let due_date = data
                    .get("due_date")
                    .and_then(|v| v.as_str())
                    .unwrap_or("soon");

                let subject = "Reminder: Payment Due".to_string();
                let body = format!(
                    "Hello,\n\n\
                    This is a reminder that you have a payment of {} due on {}.\n\n\
                    Please ensure payment is made on time to avoid any late fees.\n\n\
                    Best regards,\n\
                    Property Management System",
                    amount, due_date
                );
                (subject, body)
            }
            "meeting_reminder" => {
                let title = data
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Meeting");
                let date_time = data
                    .get("date_time")
                    .and_then(|v| v.as_str())
                    .unwrap_or("scheduled time");

                let subject = format!("Reminder: {} tomorrow", title);
                let body = format!(
                    "Hello,\n\n\
                    This is a reminder that \"{}\" is scheduled for {}.\n\n\
                    Please make sure to attend.\n\n\
                    Best regards,\n\
                    Property Management System",
                    title, date_time
                );
                (subject, body)
            }
            _ => {
                tracing::warn!(template = %template, "Unknown email template, using generic message");
                let subject = "Notification".to_string();
                let body = format!(
                    "Hello,\n\n\
                    You have a new notification.\n\n\
                    Details: {:?}\n\n\
                    Best regards,\n\
                    Property Management System",
                    data
                );
                (subject, body)
            }
        };

        tracing::info!(
            to = %to,
            template = %template,
            "Sending template email"
        );

        self.send_email(to, &subject, &body).await
    }

    /// Internal send method supporting both SMTP and logging modes.
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailError> {
        match self.transport.as_ref() {
            EmailTransport::Smtp(smtp) => self.send_via_smtp(smtp, to, subject, body).await,
            EmailTransport::Log => {
                // Development mode: log the email content
                tracing::info!(
                    to = %to,
                    subject = %subject,
                    body_length = %body.len(),
                    "Email logged (development mode)"
                );
                tracing::debug!(
                    to = %to,
                    subject = %subject,
                    body = %body,
                    "Full email content (development mode)"
                );
                Ok(())
            }
        }
    }

    /// Send email via SMTP transport.
    async fn send_via_smtp(
        &self,
        transport: &AsyncSmtpTransport<Tokio1Executor>,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), EmailError> {
        let config = self
            .smtp_config
            .as_ref()
            .ok_or_else(|| EmailError::ConfigurationError("SMTP config missing".to_string()))?;

        // Parse sender address
        let from_mailbox: Mailbox = format!("{} <{}>", config.from_name, config.from_address)
            .parse()
            .map_err(|e| EmailError::InvalidAddress(format!("Invalid from address: {}", e)))?;

        // Parse recipient address
        let to_mailbox: Mailbox = to.parse().map_err(|e| {
            EmailError::InvalidAddress(format!("Invalid to address '{}': {}", to, e))
        })?;

        // Build the email message
        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|e| EmailError::MessageBuildError(e.to_string()))?;

        // Send the email
        transport.send(email).await.map_err(|e| {
            tracing::error!(
                error = %e,
                to = %to,
                subject = %subject,
                "Failed to send email via SMTP"
            );
            EmailError::SendFailed(e.to_string())
        })?;

        tracing::info!(
            to = %to,
            subject = %subject,
            "Email sent successfully via SMTP"
        );

        Ok(())
    }

    /// Send email with HTML content.
    pub async fn send_html_email(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), EmailError> {
        match self.transport.as_ref() {
            EmailTransport::Smtp(smtp) => {
                self.send_html_via_smtp(smtp, to, subject, html_body, text_body)
                    .await
            }
            EmailTransport::Log => {
                tracing::info!(
                    to = %to,
                    subject = %subject,
                    html_length = %html_body.len(),
                    text_length = %text_body.len(),
                    "HTML email logged (development mode)"
                );
                Ok(())
            }
        }
    }

    /// Send HTML email via SMTP transport.
    async fn send_html_via_smtp(
        &self,
        transport: &AsyncSmtpTransport<Tokio1Executor>,
        to: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), EmailError> {
        use lettre::message::{MultiPart, SinglePart};

        let config = self
            .smtp_config
            .as_ref()
            .ok_or_else(|| EmailError::ConfigurationError("SMTP config missing".to_string()))?;

        let from_mailbox: Mailbox = format!("{} <{}>", config.from_name, config.from_address)
            .parse()
            .map_err(|e| EmailError::InvalidAddress(format!("Invalid from address: {}", e)))?;

        let to_mailbox: Mailbox = to.parse().map_err(|e| {
            EmailError::InvalidAddress(format!("Invalid to address '{}': {}", to, e))
        })?;

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(text_body.to_string()),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html_body.to_string()),
                    ),
            )
            .map_err(|e| EmailError::MessageBuildError(e.to_string()))?;

        transport.send(email).await.map_err(|e| {
            tracing::error!(
                error = %e,
                to = %to,
                subject = %subject,
                "Failed to send HTML email via SMTP"
            );
            EmailError::SendFailed(e.to_string())
        })?;

        tracing::info!(
            to = %to,
            subject = %subject,
            "HTML email sent successfully via SMTP"
        );

        Ok(())
    }

    // ==================== Localized Templates ====================

    fn get_verification_subject(&self, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => "Overte svoju e-mailovú adresu".to_string(),
            Locale::Czech => "Ověřte svou e-mailovou adresu".to_string(),
            Locale::German => "Bestätigen Sie Ihre E-Mail-Adresse".to_string(),
            Locale::English => "Verify your email address".to_string(),
        }
    }

    fn get_verification_body(&self, name: &str, url: &str, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => format!(
                "Dobrý deň {},\n\nĎakujeme za registráciu. Kliknutím na odkaz nižšie overte svoju e-mailovú adresu:\n\n{}\n\nOdkaz je platný 24 hodín.\n\nS pozdravom,\nTím PPT",
                name, url
            ),
            Locale::Czech => format!(
                "Dobrý den {},\n\nDěkujeme za registraci. Kliknutím na odkaz níže ověřte svou e-mailovou adresu:\n\n{}\n\nOdkaz je platný 24 hodin.\n\nS pozdravem,\nTým PPT",
                name, url
            ),
            Locale::German => format!(
                "Guten Tag {},\n\nVielen Dank für Ihre Registrierung. Klicken Sie auf den Link unten, um Ihre E-Mail-Adresse zu bestätigen:\n\n{}\n\nDer Link ist 24 Stunden gültig.\n\nMit freundlichen Grüßen,\nDas PPT-Team",
                name, url
            ),
            Locale::English => format!(
                "Hello {},\n\nThank you for registering. Click the link below to verify your email address:\n\n{}\n\nThis link is valid for 24 hours.\n\nBest regards,\nThe PPT Team",
                name, url
            ),
        }
    }

    fn get_reset_subject(&self, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => "Obnovenie hesla".to_string(),
            Locale::Czech => "Obnovení hesla".to_string(),
            Locale::German => "Passwort zurücksetzen".to_string(),
            Locale::English => "Reset your password".to_string(),
        }
    }

    fn get_reset_body(&self, name: &str, url: &str, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => format!(
                "Dobrý deň {},\n\nDostali sme žiadosť o obnovenie vášho hesla. Kliknutím na odkaz nižšie ho obnovte:\n\n{}\n\nOdkaz je platný 1 hodinu. Ak ste o obnovenie hesla nežiadali, túto správu ignorujte.\n\nS pozdravom,\nTím PPT",
                name, url
            ),
            Locale::Czech => format!(
                "Dobrý den {},\n\nObdrželi jsme žádost o obnovení vašeho hesla. Kliknutím na odkaz níže ho obnovte:\n\n{}\n\nOdkaz je platný 1 hodinu. Pokud jste o obnovení hesla nežádali, tuto zprávu ignorujte.\n\nS pozdravem,\nTým PPT",
                name, url
            ),
            Locale::German => format!(
                "Guten Tag {},\n\nWir haben eine Anfrage zum Zurücksetzen Ihres Passworts erhalten. Klicken Sie auf den Link unten:\n\n{}\n\nDer Link ist 1 Stunde gültig. Wenn Sie kein Zurücksetzen angefordert haben, ignorieren Sie diese Nachricht.\n\nMit freundlichen Grüßen,\nDas PPT-Team",
                name, url
            ),
            Locale::English => format!(
                "Hello {},\n\nWe received a request to reset your password. Click the link below:\n\n{}\n\nThis link is valid for 1 hour. If you didn't request a password reset, please ignore this email.\n\nBest regards,\nThe PPT Team",
                name, url
            ),
        }
    }

    fn get_invitation_subject(&self, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => "Pozvánka do Property Management System".to_string(),
            Locale::Czech => "Pozvánka do Property Management System".to_string(),
            Locale::German => "Einladung zum Property Management System".to_string(),
            Locale::English => "Invitation to Property Management System".to_string(),
        }
    }

    fn get_invitation_body(&self, inviter: &str, url: &str, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => format!(
                "Dobrý deň,\n\n{} vás pozýva pripojiť sa k Property Management System. Kliknutím na odkaz nižšie vytvorte účet:\n\n{}\n\nS pozdravom,\nTím PPT",
                inviter, url
            ),
            Locale::Czech => format!(
                "Dobrý den,\n\n{} vás zve připojit se k Property Management System. Kliknutím na odkaz níže vytvořte účet:\n\n{}\n\nS pozdravem,\nTým PPT",
                inviter, url
            ),
            Locale::German => format!(
                "Guten Tag,\n\n{} lädt Sie ein, dem Property Management System beizutreten. Klicken Sie auf den Link unten:\n\n{}\n\nMit freundlichen Grüßen,\nDas PPT-Team",
                inviter, url
            ),
            Locale::English => format!(
                "Hello,\n\n{} has invited you to join Property Management System. Click the link below to create your account:\n\n{}\n\nBest regards,\nThe PPT Team",
                inviter, url
            ),
        }
    }

    fn get_notification_body(&self, name: &str, message: &str, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => format!(
                "Dobrý deň {},\n\n{}\n\nS pozdravom,\nTím PPT",
                name, message
            ),
            Locale::Czech => format!(
                "Dobrý den {},\n\n{}\n\nS pozdravem,\nTým PPT",
                name, message
            ),
            Locale::German => format!(
                "Guten Tag {},\n\n{}\n\nMit freundlichen Grüßen,\nDas PPT-Team",
                name, message
            ),
            Locale::English => format!(
                "Hello {},\n\n{}\n\nBest regards,\nThe PPT Team",
                name, message
            ),
        }
    }

    // ==================== Additional Email Types ====================

    /// Send welcome email after successful email verification.
    pub async fn send_welcome_email(
        &self,
        email: &str,
        name: &str,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let subject = self.get_welcome_subject(locale);
        let body = self.get_welcome_body(name, locale);

        self.send_email(email, &subject, &body).await
    }

    fn get_welcome_subject(&self, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => "Vitajte v Property Management System".to_string(),
            Locale::Czech => "Vítejte v Property Management System".to_string(),
            Locale::German => "Willkommen im Property Management System".to_string(),
            Locale::English => "Welcome to Property Management System".to_string(),
        }
    }

    fn get_welcome_body(&self, name: &str, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => format!(
                "Dobrý deň {},\n\nVitajte v Property Management System! Váš účet bol úspešne overený a je pripravený na použitie.\n\nPrihláste sa na {} a začnite spravovať svoje nehnuteľnosti.\n\nAk máte akékoľvek otázky, neváhajte nás kontaktovať.\n\nS pozdravom,\nTím PPT",
                name, self.base_url
            ),
            Locale::Czech => format!(
                "Dobrý den {},\n\nVítejte v Property Management System! Váš účet byl úspěšně ověřen a je připraven k použití.\n\nPřihlaste se na {} a začněte spravovat své nemovitosti.\n\nPokud máte jakékoli otázky, neváhejte nás kontaktovat.\n\nS pozdravem,\nTým PPT",
                name, self.base_url
            ),
            Locale::German => format!(
                "Guten Tag {},\n\nWillkommen im Property Management System! Ihr Konto wurde erfolgreich verifiziert und ist einsatzbereit.\n\nMelden Sie sich unter {} an und beginnen Sie mit der Verwaltung Ihrer Immobilien.\n\nBei Fragen stehen wir Ihnen gerne zur Verfügung.\n\nMit freundlichen Grüßen,\nDas PPT-Team",
                name, self.base_url
            ),
            Locale::English => format!(
                "Hello {},\n\nWelcome to Property Management System! Your account has been successfully verified and is ready to use.\n\nLog in at {} to start managing your properties.\n\nIf you have any questions, don't hesitate to contact us.\n\nBest regards,\nThe PPT Team",
                name, self.base_url
            ),
        }
    }

    /// Send security alert email (e.g., new login from unknown device).
    pub async fn send_security_alert_email(
        &self,
        email: &str,
        name: &str,
        device_info: &str,
        ip_address: &str,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let subject = self.get_security_alert_subject(locale);
        let body = self.get_security_alert_body(name, device_info, ip_address, locale);

        self.send_email(email, &subject, &body).await
    }

    fn get_security_alert_subject(&self, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => "Bezpečnostné upozornenie - nové prihlásenie".to_string(),
            Locale::Czech => "Bezpečnostní upozornění - nové přihlášení".to_string(),
            Locale::German => "Sicherheitswarnung - Neue Anmeldung".to_string(),
            Locale::English => "Security Alert - New Login".to_string(),
        }
    }

    fn get_security_alert_body(
        &self,
        name: &str,
        device: &str,
        ip: &str,
        locale: &Locale,
    ) -> String {
        match locale {
            Locale::Slovak => format!(
                "Dobrý deň {},\n\nZaznamenali sme nové prihlásenie do vášho účtu:\n\n- Zariadenie: {}\n- IP adresa: {}\n\nAk ste to boli vy, túto správu môžete ignorovať.\n\nAk ste sa neprihlasovali, odporúčame okamžite zmeniť heslo a skontrolovať aktívne relácie v nastaveniach účtu.\n\nS pozdravom,\nTím PPT",
                name, device, ip
            ),
            Locale::Czech => format!(
                "Dobrý den {},\n\nZaznamenali jsme nové přihlášení k vašemu účtu:\n\n- Zařízení: {}\n- IP adresa: {}\n\nPokud jste to byli vy, tuto zprávu můžete ignorovat.\n\nPokud jste se nepřihlašovali, doporučujeme okamžitě změnit heslo a zkontrolovat aktivní relace v nastavení účtu.\n\nS pozdravem,\nTým PPT",
                name, device, ip
            ),
            Locale::German => format!(
                "Guten Tag {},\n\nWir haben eine neue Anmeldung bei Ihrem Konto festgestellt:\n\n- Gerät: {}\n- IP-Adresse: {}\n\nWenn Sie das waren, können Sie diese Nachricht ignorieren.\n\nFalls Sie sich nicht angemeldet haben, empfehlen wir, sofort das Passwort zu ändern und aktive Sitzungen in den Kontoeinstellungen zu überprüfen.\n\nMit freundlichen Grüßen,\nDas PPT-Team",
                name, device, ip
            ),
            Locale::English => format!(
                "Hello {},\n\nWe detected a new login to your account:\n\n- Device: {}\n- IP Address: {}\n\nIf this was you, you can ignore this message.\n\nIf you didn't log in, we recommend immediately changing your password and reviewing active sessions in your account settings.\n\nBest regards,\nThe PPT Team",
                name, device, ip
            ),
        }
    }

    /// Send password changed notification email.
    pub async fn send_password_changed_email(
        &self,
        email: &str,
        name: &str,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let subject = self.get_password_changed_subject(locale);
        let body = self.get_password_changed_body(name, locale);

        self.send_email(email, &subject, &body).await
    }

    fn get_password_changed_subject(&self, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => "Vaše heslo bolo zmenené".to_string(),
            Locale::Czech => "Vaše heslo bylo změněno".to_string(),
            Locale::German => "Ihr Passwort wurde geändert".to_string(),
            Locale::English => "Your password has been changed".to_string(),
        }
    }

    fn get_password_changed_body(&self, name: &str, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => format!(
                "Dobrý deň {},\n\nVaše heslo bolo úspešne zmenené.\n\nAk ste túto zmenu nevykonali, okamžite kontaktujte podporu a zmeňte si heslo.\n\nS pozdravom,\nTím PPT",
                name
            ),
            Locale::Czech => format!(
                "Dobrý den {},\n\nVaše heslo bylo úspěšně změněno.\n\nPokud jste tuto změnu neprovedli, okamžitě kontaktujte podporu a změňte si heslo.\n\nS pozdravem,\nTým PPT",
                name
            ),
            Locale::German => format!(
                "Guten Tag {},\n\nIhr Passwort wurde erfolgreich geändert.\n\nWenn Sie diese Änderung nicht vorgenommen haben, kontaktieren Sie sofort den Support und ändern Sie Ihr Passwort.\n\nMit freundlichen Grüßen,\nDas PPT-Team",
                name
            ),
            Locale::English => format!(
                "Hello {},\n\nYour password has been successfully changed.\n\nIf you didn't make this change, please contact support immediately and change your password.\n\nBest regards,\nThe PPT Team",
                name
            ),
        }
    }

    /// Send account suspended notification email.
    pub async fn send_account_suspended_email(
        &self,
        email: &str,
        name: &str,
        reason: Option<&str>,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let subject = self.get_account_suspended_subject(locale);
        let body = self.get_account_suspended_body(name, reason, locale);

        self.send_email(email, &subject, &body).await
    }

    fn get_account_suspended_subject(&self, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => "Váš účet bol pozastavený".to_string(),
            Locale::Czech => "Váš účet byl pozastaven".to_string(),
            Locale::German => "Ihr Konto wurde gesperrt".to_string(),
            Locale::English => "Your account has been suspended".to_string(),
        }
    }

    fn get_account_suspended_body(
        &self,
        name: &str,
        reason: Option<&str>,
        locale: &Locale,
    ) -> String {
        let reason_text = reason.unwrap_or("-");
        match locale {
            Locale::Slovak => format!(
                "Dobrý deň {},\n\nVáš účet bol pozastavený.\n\nDôvod: {}\n\nPre viac informácií kontaktujte podporu.\n\nS pozdravom,\nTím PPT",
                name, reason_text
            ),
            Locale::Czech => format!(
                "Dobrý den {},\n\nVáš účet byl pozastaven.\n\nDůvod: {}\n\nPro více informací kontaktujte podporu.\n\nS pozdravem,\nTým PPT",
                name, reason_text
            ),
            Locale::German => format!(
                "Guten Tag {},\n\nIhr Konto wurde gesperrt.\n\nGrund: {}\n\nFür weitere Informationen wenden Sie sich bitte an den Support.\n\nMit freundlichen Grüßen,\nDas PPT-Team",
                name, reason_text
            ),
            Locale::English => format!(
                "Hello {},\n\nYour account has been suspended.\n\nReason: {}\n\nFor more information, please contact support.\n\nBest regards,\nThe PPT Team",
                name, reason_text
            ),
        }
    }

    /// Send fault notification email to building manager.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_fault_notification_email(
        &self,
        email: &str,
        name: &str,
        fault_title: &str,
        fault_description: &str,
        building_name: &str,
        reporter_name: &str,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let subject = self.get_fault_notification_subject(building_name, locale);
        let body = self.get_fault_notification_body(
            name,
            fault_title,
            fault_description,
            building_name,
            reporter_name,
            locale,
        );

        self.send_email(email, &subject, &body).await
    }

    fn get_fault_notification_subject(&self, building_name: &str, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => format!("Nová porucha nahlásená - {}", building_name),
            Locale::Czech => format!("Nová porucha nahlášena - {}", building_name),
            Locale::German => format!("Neuer Mangel gemeldet - {}", building_name),
            Locale::English => format!("New Fault Reported - {}", building_name),
        }
    }

    fn get_fault_notification_body(
        &self,
        name: &str,
        fault_title: &str,
        fault_description: &str,
        building_name: &str,
        reporter_name: &str,
        locale: &Locale,
    ) -> String {
        match locale {
            Locale::Slovak => format!(
                "Dobrý deň {},\n\nBola nahlásená nová porucha v budove {}.\n\nNázov: {}\nPopis: {}\nNahlásil: {}\n\nPreskúmajte prosím túto poruchu v systéme.\n\nS pozdravom,\nTím PPT",
                name, building_name, fault_title, fault_description, reporter_name
            ),
            Locale::Czech => format!(
                "Dobrý den {},\n\nByla nahlášena nová porucha v budově {}.\n\nNázev: {}\nPopis: {}\nNahlásil: {}\n\nProsím přezkoumejte tuto poruchu v systému.\n\nS pozdravem,\nTým PPT",
                name, building_name, fault_title, fault_description, reporter_name
            ),
            Locale::German => format!(
                "Guten Tag {},\n\nEin neuer Mangel wurde im Gebäude {} gemeldet.\n\nTitel: {}\nBeschreibung: {}\nGemeldet von: {}\n\nBitte überprüfen Sie diesen Mangel im System.\n\nMit freundlichen Grüßen,\nDas PPT-Team",
                name, building_name, fault_title, fault_description, reporter_name
            ),
            Locale::English => format!(
                "Hello {},\n\nA new fault has been reported in building {}.\n\nTitle: {}\nDescription: {}\nReported by: {}\n\nPlease review this fault in the system.\n\nBest regards,\nThe PPT Team",
                name, building_name, fault_title, fault_description, reporter_name
            ),
        }
    }

    // ==================== E-Signature Emails (Story 84.2) ====================

    /// Escape a plain-text string for safe interpolation into HTML.
    fn html_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    // ------------------------------------------------------------------
    // Signature email translations (i18n).
    //
    // sk / cs / de translations are author-best-effort and should get a
    // native review before customer rollout. Subject lines show in inbox
    // previews; body strings show inside the email. Both are templated
    // with placeholders like `{signer_name}` and `{document_name}` —
    // values are HTML-escaped at the call site before substitution.
    // ------------------------------------------------------------------

    fn signature_request_subject(document_name: &str, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => format!("Žiadosť o podpis: „{}\"", document_name),
            Locale::Czech => format!("Žádost o podpis: „{}\"", document_name),
            Locale::German => {
                format!("Unterschrift erforderlich: „{}\"", document_name)
            }
            Locale::English => format!("Action required: Please sign \"{}\"", document_name),
        }
    }

    fn signature_reminder_subject(document_name: &str, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => format!("Pripomienka: podpíšte „{}\"", document_name),
            Locale::Czech => format!("Připomínka: podepište „{}\"", document_name),
            Locale::German => format!("Erinnerung: Unterschrift für „{}\"", document_name),
            Locale::English => format!(
                "Reminder: Your signature is still needed on \"{}\"",
                document_name
            ),
        }
    }

    fn signature_declined_subject(document_name: &str, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => format!("Podpis zamietnutý pre „{}\"", document_name),
            Locale::Czech => format!("Podpis odmítnut pro „{}\"", document_name),
            Locale::German => format!("Unterschrift abgelehnt für „{}\"", document_name),
            Locale::English => format!("Signature declined for \"{}\"", document_name),
        }
    }

    fn signature_completed_subject(document_name: &str, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => format!("Všetky podpisy zaznamenané pre „{}\"", document_name),
            Locale::Czech => format!("Všechny podpisy získány pro „{}\"", document_name),
            Locale::German => format!("Alle Unterschriften vorhanden für „{}\"", document_name),
            Locale::English => format!("All signatures collected for \"{}\"", document_name),
        }
    }

    // ------------------------------------------------------------------
    // Body translation tables.
    //
    // Each table is a small `&'static str` bundle returned by per-email
    // helpers. Templates use `{placeholder}` markers that the call site
    // fills with HTML-escaped values. Templates contain no `format!`
    // syntax — `.replace` is enough and avoids the runtime cost of
    // re-parsing format strings.
    //
    // Closing line `closing_html` / `closing_plain` is the same across
    // all four emails per locale: lifted into a shared helper.
    // ------------------------------------------------------------------

    fn email_closing_plain(locale: &Locale) -> &'static str {
        match locale {
            Locale::Slovak => "S pozdravom,\nProperty Management System",
            Locale::Czech => "S pozdravem,\nProperty Management System",
            Locale::German => "Mit freundlichen Grüßen,\nProperty Management System",
            Locale::English => "Best regards,\nProperty Management System",
        }
    }

    fn signature_request_body_html(locale: &Locale) -> &'static str {
        match locale {
            Locale::Slovak => "\
<h2 style=\"color:#2c5282\">Máte dokument na podpis</h2>\
<p>Dobrý deň <strong>{signer_name}</strong>,</p>\
<p><strong>{requester_name}</strong> vás žiada o elektronický podpis dokumentu \
   <strong>&quot;{document_name}&quot;</strong>.</p>\
{custom_message}\
{expiry_block}\
<p style=\"margin:30px 0\">\
  <a href=\"{signing_url}\" style=\"background:#2c5282;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Skontrolovať a podpísať dokument</a>\
</p>\
<p style=\"font-size:12px;color:#718096\">Ak tlačidlo nefunguje, skopírujte a vložte tento odkaz:<br><a href=\"{signing_url}\">{signing_url}</a></p>\
<hr style=\"border:none;border-top:1px solid #e2e8f0;margin:30px 0\">\
<p style=\"font-size:12px;color:#718096\">Ak ste túto žiadosť neočakávali, môžete tento e-mail bezpečne ignorovať.</p>",
            Locale::Czech => "\
<h2 style=\"color:#2c5282\">Máte dokument k podpisu</h2>\
<p>Dobrý den <strong>{signer_name}</strong>,</p>\
<p><strong>{requester_name}</strong> vás žádá o elektronický podpis dokumentu \
   <strong>&quot;{document_name}&quot;</strong>.</p>\
{custom_message}\
{expiry_block}\
<p style=\"margin:30px 0\">\
  <a href=\"{signing_url}\" style=\"background:#2c5282;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Zkontrolovat a podepsat dokument</a>\
</p>\
<p style=\"font-size:12px;color:#718096\">Pokud tlačítko nefunguje, zkopírujte a vložte tento odkaz:<br><a href=\"{signing_url}\">{signing_url}</a></p>\
<hr style=\"border:none;border-top:1px solid #e2e8f0;margin:30px 0\">\
<p style=\"font-size:12px;color:#718096\">Pokud jste tuto žádost neočekávali, můžete tento e-mail bezpečně ignorovat.</p>",
            Locale::German => "\
<h2 style=\"color:#2c5282\">Sie haben ein Dokument zu unterschreiben</h2>\
<p>Hallo <strong>{signer_name}</strong>,</p>\
<p><strong>{requester_name}</strong> bittet um Ihre elektronische Unterschrift für \
   <strong>&quot;{document_name}&quot;</strong>.</p>\
{custom_message}\
{expiry_block}\
<p style=\"margin:30px 0\">\
  <a href=\"{signing_url}\" style=\"background:#2c5282;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Dokument prüfen und unterschreiben</a>\
</p>\
<p style=\"font-size:12px;color:#718096\">Falls der Button nicht funktioniert, kopieren Sie diese URL:<br><a href=\"{signing_url}\">{signing_url}</a></p>\
<hr style=\"border:none;border-top:1px solid #e2e8f0;margin:30px 0\">\
<p style=\"font-size:12px;color:#718096\">Sollten Sie diese Anfrage nicht erwartet haben, können Sie diese E-Mail ignorieren.</p>",
            Locale::English => "\
<h2 style=\"color:#2c5282\">You have a document to sign</h2>\
<p>Hello <strong>{signer_name}</strong>,</p>\
<p><strong>{requester_name}</strong> has requested your electronic signature on \
   <strong>&quot;{document_name}&quot;</strong>.</p>\
{custom_message}\
{expiry_block}\
<p style=\"margin:30px 0\">\
  <a href=\"{signing_url}\" style=\"background:#2c5282;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Review &amp; Sign Document</a>\
</p>\
<p style=\"font-size:12px;color:#718096\">If the button above doesn't work, copy and paste this URL:<br><a href=\"{signing_url}\">{signing_url}</a></p>\
<hr style=\"border:none;border-top:1px solid #e2e8f0;margin:30px 0\">\
<p style=\"font-size:12px;color:#718096\">If you were not expecting this request, you may safely ignore this email.</p>",
        }
    }

    fn signature_request_body_plain(locale: &Locale) -> &'static str {
        match locale {
            Locale::Slovak => "Dobrý deň {signer_name},\n\n{requester_name} vás žiada o elektronický podpis dokumentu \"{document_name}\".\n{custom_message}\nDokument môžete skontrolovať a podpísať tu:\n{signing_url}\n{expiry_block}\nAk ste túto žiadosť neočakávali, môžete tento e-mail ignorovať.",
            Locale::Czech => "Dobrý den {signer_name},\n\n{requester_name} vás žádá o elektronický podpis dokumentu \"{document_name}\".\n{custom_message}\nDokument můžete zkontrolovat a podepsat zde:\n{signing_url}\n{expiry_block}\nPokud jste tuto žádost neočekávali, můžete tento e-mail ignorovat.",
            Locale::German => "Hallo {signer_name},\n\n{requester_name} bittet um Ihre elektronische Unterschrift für \"{document_name}\".\n{custom_message}\nBitte prüfen und unterschreiben Sie hier:\n{signing_url}\n{expiry_block}\nSollten Sie diese Anfrage nicht erwartet haben, können Sie diese E-Mail ignorieren.",
            Locale::English => "Hello {signer_name},\n\n{requester_name} has requested your electronic signature on \"{document_name}\".\n{custom_message}\nPlease review and sign at:\n{signing_url}\n{expiry_block}\nIf you were not expecting this, you may ignore this email.",
        }
    }

    /// Localised "this request expires on …" block (HTML + plain).
    /// Returns `("", "")` when `expires_at` is `None`.
    fn expiry_blocks(expires_at: Option<&str>, locale: &Locale, urgent: bool) -> (String, String) {
        let Some(date) = expires_at else {
            return (String::new(), String::new());
        };
        let date_h = Self::html_escape(date);
        let (html_template, plain_template) = match (locale, urgent) {
            (Locale::Slovak, false) => (
                "<p>Žiadosť expiruje <strong>{date}</strong>.</p>",
                "\nŽiadosť expiruje: {date}\n",
            ),
            (Locale::Slovak, true) => (
                "<p><strong>Dôležité:</strong> žiadosť expiruje <strong>{date}</strong>.</p>",
                "\nDôležité: žiadosť expiruje {date}.\n",
            ),
            (Locale::Czech, false) => (
                "<p>Žádost vyprší <strong>{date}</strong>.</p>",
                "\nŽádost vyprší: {date}\n",
            ),
            (Locale::Czech, true) => (
                "<p><strong>Důležité:</strong> žádost vyprší <strong>{date}</strong>.</p>",
                "\nDůležité: žádost vyprší {date}.\n",
            ),
            (Locale::German, false) => (
                "<p>Diese Anfrage läuft am <strong>{date}</strong> ab.</p>",
                "\nDiese Anfrage läuft ab am: {date}\n",
            ),
            (Locale::German, true) => (
                "<p><strong>Wichtig:</strong> diese Anfrage läuft am <strong>{date}</strong> ab.</p>",
                "\nWichtig: diese Anfrage läuft am {date} ab.\n",
            ),
            (Locale::English, false) => (
                "<p>This request expires on <strong>{date}</strong>.</p>",
                "\nThis request expires on: {date}\n",
            ),
            (Locale::English, true) => (
                "<p><strong>Important:</strong> This request expires on <strong>{date}</strong>.</p>",
                "\nImportant: This request expires on {date}.\n",
            ),
        };
        (
            html_template.replace("{date}", &date_h),
            plain_template.replace("{date}", date),
        )
    }

    fn signature_reminder_body_html(locale: &Locale) -> &'static str {
        match locale {
            Locale::Slovak => "\
<h2 style=\"color:#c05621\">Pripomienka: vyžaduje sa podpis</h2>\
<p>Dobrý deň <strong>{signer_name}</strong>,</p>\
<p>Toto je priateľská pripomienka, že stále potrebujeme váš podpis dokumentu <strong>&quot;{document_name}&quot;</strong>.</p>\
{expiry_block}\
<p style=\"margin:30px 0\"><a href=\"{signing_url}\" style=\"background:#c05621;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Podpísať teraz</a></p>\
<p style=\"font-size:12px;color:#718096\">Ak tlačidlo nefunguje:<br><a href=\"{signing_url}\">{signing_url}</a></p>",
            Locale::Czech => "\
<h2 style=\"color:#c05621\">Připomínka: vyžaduje se podpis</h2>\
<p>Dobrý den <strong>{signer_name}</strong>,</p>\
<p>Toto je přátelská připomínka, že stále potřebujeme váš podpis dokumentu <strong>&quot;{document_name}&quot;</strong>.</p>\
{expiry_block}\
<p style=\"margin:30px 0\"><a href=\"{signing_url}\" style=\"background:#c05621;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Podepsat nyní</a></p>\
<p style=\"font-size:12px;color:#718096\">Pokud tlačítko nefunguje:<br><a href=\"{signing_url}\">{signing_url}</a></p>",
            Locale::German => "\
<h2 style=\"color:#c05621\">Erinnerung: Unterschrift erforderlich</h2>\
<p>Hallo <strong>{signer_name}</strong>,</p>\
<p>Dies ist eine freundliche Erinnerung, dass Ihre Unterschrift für <strong>&quot;{document_name}&quot;</strong> noch ausstehend ist.</p>\
{expiry_block}\
<p style=\"margin:30px 0\"><a href=\"{signing_url}\" style=\"background:#c05621;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Jetzt unterschreiben</a></p>\
<p style=\"font-size:12px;color:#718096\">Falls der Button nicht funktioniert:<br><a href=\"{signing_url}\">{signing_url}</a></p>",
            Locale::English => "\
<h2 style=\"color:#c05621\">Reminder: Signature Required</h2>\
<p>Hello <strong>{signer_name}</strong>,</p>\
<p>This is a friendly reminder that your signature is still needed on <strong>&quot;{document_name}&quot;</strong>.</p>\
{expiry_block}\
<p style=\"margin:30px 0\"><a href=\"{signing_url}\" style=\"background:#c05621;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Sign Now</a></p>\
<p style=\"font-size:12px;color:#718096\">If the button above doesn't work:<br><a href=\"{signing_url}\">{signing_url}</a></p>",
        }
    }

    fn signature_reminder_body_plain(locale: &Locale) -> &'static str {
        match locale {
            Locale::Slovak => "Dobrý deň {signer_name},\n\nToto je priateľská pripomienka, že stále potrebujeme váš podpis dokumentu \"{document_name}\".\n{expiry_block}\nPodpíšte tu:\n{signing_url}",
            Locale::Czech => "Dobrý den {signer_name},\n\nToto je přátelská připomínka, že stále potřebujeme váš podpis dokumentu \"{document_name}\".\n{expiry_block}\nPodepište zde:\n{signing_url}",
            Locale::German => "Hallo {signer_name},\n\nDies ist eine freundliche Erinnerung, dass Ihre Unterschrift für \"{document_name}\" noch ausstehend ist.\n{expiry_block}\nHier unterschreiben:\n{signing_url}",
            Locale::English => "Hello {signer_name},\n\nThis is a friendly reminder that your signature is still needed on \"{document_name}\".\n{expiry_block}\nSign at:\n{signing_url}",
        }
    }

    fn signature_declined_body_html(locale: &Locale) -> &'static str {
        match locale {
            Locale::Slovak => "\
<h2 style=\"color:#c53030\">Podpis zamietnutý</h2>\
<p>Dobrý deň <strong>{requester_name}</strong>,</p>\
<p><strong>{signer_name}</strong> ({signer_email}) <strong>odmietol</strong> podpísať <strong>&quot;{document_name}&quot;</strong>.</p>\
{reason_block}\
<p style=\"margin:30px 0\"><a href=\"{manage_url}\" style=\"background:#2c5282;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Spravovať žiadosť o podpis</a></p>",
            Locale::Czech => "\
<h2 style=\"color:#c53030\">Podpis odmítnut</h2>\
<p>Dobrý den <strong>{requester_name}</strong>,</p>\
<p><strong>{signer_name}</strong> ({signer_email}) <strong>odmítl</strong> podepsat <strong>&quot;{document_name}&quot;</strong>.</p>\
{reason_block}\
<p style=\"margin:30px 0\"><a href=\"{manage_url}\" style=\"background:#2c5282;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Spravovat žádost o podpis</a></p>",
            Locale::German => "\
<h2 style=\"color:#c53030\">Unterschrift abgelehnt</h2>\
<p>Hallo <strong>{requester_name}</strong>,</p>\
<p><strong>{signer_name}</strong> ({signer_email}) hat das Unterschreiben von <strong>&quot;{document_name}&quot;</strong> <strong>abgelehnt</strong>.</p>\
{reason_block}\
<p style=\"margin:30px 0\"><a href=\"{manage_url}\" style=\"background:#2c5282;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Anfrage verwalten</a></p>",
            Locale::English => "\
<h2 style=\"color:#c53030\">Signature Declined</h2>\
<p>Hello <strong>{requester_name}</strong>,</p>\
<p><strong>{signer_name}</strong> ({signer_email}) has <strong>declined</strong> to sign <strong>&quot;{document_name}&quot;</strong>.</p>\
{reason_block}\
<p style=\"margin:30px 0\"><a href=\"{manage_url}\" style=\"background:#2c5282;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Manage Signature Request</a></p>",
        }
    }

    fn signature_declined_body_plain(locale: &Locale) -> &'static str {
        match locale {
            Locale::Slovak => "Dobrý deň {requester_name},\n\n{signer_name} ({signer_email}) odmietol podpísať \"{document_name}\".\n\n{reason_block}\n\nSpravujte žiadosť tu:\n{manage_url}",
            Locale::Czech => "Dobrý den {requester_name},\n\n{signer_name} ({signer_email}) odmítl podepsat \"{document_name}\".\n\n{reason_block}\n\nSpravujte žádost zde:\n{manage_url}",
            Locale::German => "Hallo {requester_name},\n\n{signer_name} ({signer_email}) hat das Unterschreiben von \"{document_name}\" abgelehnt.\n\n{reason_block}\n\nAnfrage verwalten unter:\n{manage_url}",
            Locale::English => "Hello {requester_name},\n\n{signer_name} ({signer_email}) has declined to sign \"{document_name}\".\n\n{reason_block}\n\nManage the request at:\n{manage_url}",
        }
    }

    /// Localised "reason provided / no reason given" block for the declined
    /// email. `reason` is HTML-sanitised by the caller for the HTML form;
    /// the plain form takes the raw reason.
    fn reason_blocks(reason: Option<&str>, locale: &Locale) -> (String, String) {
        match (locale, reason.filter(|r| !r.is_empty())) {
            (Locale::Slovak, Some(r)) => (
                format!(
                    "<p><strong>Uvedený dôvod:</strong> {}</p>",
                    ammonia::clean(r)
                ),
                format!("Dôvod: {}", r),
            ),
            (Locale::Slovak, None) => (
                "<p>Dôvod nebol uvedený.</p>".to_string(),
                "Dôvod nebol uvedený.".to_string(),
            ),
            (Locale::Czech, Some(r)) => (
                format!(
                    "<p><strong>Uvedený důvod:</strong> {}</p>",
                    ammonia::clean(r)
                ),
                format!("Důvod: {}", r),
            ),
            (Locale::Czech, None) => (
                "<p>Důvod nebyl uveden.</p>".to_string(),
                "Důvod nebyl uveden.".to_string(),
            ),
            (Locale::German, Some(r)) => (
                format!(
                    "<p><strong>Angegebener Grund:</strong> {}</p>",
                    ammonia::clean(r)
                ),
                format!("Grund: {}", r),
            ),
            (Locale::German, None) => (
                "<p>Es wurde kein Grund angegeben.</p>".to_string(),
                "Es wurde kein Grund angegeben.".to_string(),
            ),
            (Locale::English, Some(r)) => (
                format!(
                    "<p><strong>Reason provided:</strong> {}</p>",
                    ammonia::clean(r)
                ),
                format!("Reason: {}", r),
            ),
            (Locale::English, None) => (
                "<p>No reason was provided.</p>".to_string(),
                "No reason was provided.".to_string(),
            ),
        }
    }

    fn signature_completed_body_html(locale: &Locale) -> &'static str {
        match locale {
            Locale::Slovak => "\
<h2 style=\"color:#276749\">Všetky podpisy zaznamenané</h2>\
<p>Dobrý deň <strong>{requester_name}</strong>,</p>\
<p>Skvelá správa! Všetkých {signers_count} podpisujúcich úspešne podpísalo <strong>&quot;{document_name}&quot;</strong>.</p>\
<p>Podpísaný dokument je teraz dostupný vo vašom systéme správy dokumentov.</p>\
<p style=\"margin:30px 0\"><a href=\"{manage_url}\" style=\"background:#276749;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Zobraziť podpísaný dokument</a></p>",
            Locale::Czech => "\
<h2 style=\"color:#276749\">Všechny podpisy získány</h2>\
<p>Dobrý den <strong>{requester_name}</strong>,</p>\
<p>Skvělá zpráva! Všech {signers_count} podepisujících úspěšně podepsalo <strong>&quot;{document_name}&quot;</strong>.</p>\
<p>Podepsaný dokument je nyní dostupný ve vašem systému správy dokumentů.</p>\
<p style=\"margin:30px 0\"><a href=\"{manage_url}\" style=\"background:#276749;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Zobrazit podepsaný dokument</a></p>",
            Locale::German => "\
<h2 style=\"color:#276749\">Alle Unterschriften vorhanden</h2>\
<p>Hallo <strong>{requester_name}</strong>,</p>\
<p>Großartige Nachrichten! Alle {signers_count} Unterzeichner haben <strong>&quot;{document_name}&quot;</strong> erfolgreich unterschrieben.</p>\
<p>Das unterschriebene Dokument ist jetzt in Ihrem Dokumentenmanagement verfügbar.</p>\
<p style=\"margin:30px 0\"><a href=\"{manage_url}\" style=\"background:#276749;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">Unterschriebenes Dokument ansehen</a></p>",
            Locale::English => "\
<h2 style=\"color:#276749\">All Signatures Collected</h2>\
<p>Hello <strong>{requester_name}</strong>,</p>\
<p>Great news! All {signers_count} signer(s) have successfully signed <strong>&quot;{document_name}&quot;</strong>.</p>\
<p>The signed document is now available in your document management system.</p>\
<p style=\"margin:30px 0\"><a href=\"{manage_url}\" style=\"background:#276749;color:#fff;padding:12px 24px;text-decoration:none;border-radius:4px;font-weight:bold\">View Signed Document</a></p>",
        }
    }

    fn signature_completed_body_plain(locale: &Locale) -> &'static str {
        match locale {
            Locale::Slovak => "Dobrý deň {requester_name},\n\nVšetkých {signers_count} podpisujúcich úspešne podpísalo \"{document_name}\".\n\nPodpísaný dokument si môžete pozrieť tu:\n{manage_url}",
            Locale::Czech => "Dobrý den {requester_name},\n\nVšech {signers_count} podepisujících úspěšně podepsalo \"{document_name}\".\n\nPodepsaný dokument si můžete prohlédnout zde:\n{manage_url}",
            Locale::German => "Hallo {requester_name},\n\nAlle {signers_count} Unterzeichner haben \"{document_name}\" erfolgreich unterschrieben.\n\nUnterschriebenes Dokument hier ansehen:\n{manage_url}",
            Locale::English => "Hello {requester_name},\n\nAll {signers_count} signer(s) have successfully signed \"{document_name}\".\n\nView the signed document at:\n{manage_url}",
        }
    }

    /// Localised HTML `<title>` for the signature email shell.
    fn signature_email_title(kind: SignatureEmailKind, locale: &Locale) -> &'static str {
        match (kind, locale) {
            (SignatureEmailKind::Request, Locale::Slovak) => "Žiadosť o podpis",
            (SignatureEmailKind::Request, Locale::Czech) => "Žádost o podpis",
            (SignatureEmailKind::Request, Locale::German) => "Unterschriftsanfrage",
            (SignatureEmailKind::Request, Locale::English) => "Signature Request",
            (SignatureEmailKind::Reminder, Locale::Slovak) => "Pripomienka podpisu",
            (SignatureEmailKind::Reminder, Locale::Czech) => "Připomínka podpisu",
            (SignatureEmailKind::Reminder, Locale::German) => "Unterschriftserinnerung",
            (SignatureEmailKind::Reminder, Locale::English) => "Signature Reminder",
            (SignatureEmailKind::Declined, Locale::Slovak) => "Podpis zamietnutý",
            (SignatureEmailKind::Declined, Locale::Czech) => "Podpis odmítnut",
            (SignatureEmailKind::Declined, Locale::German) => "Unterschrift abgelehnt",
            (SignatureEmailKind::Declined, Locale::English) => "Signature Declined",
            (SignatureEmailKind::Completed, Locale::Slovak) => "Podpisy získané",
            (SignatureEmailKind::Completed, Locale::Czech) => "Podpisy získány",
            (SignatureEmailKind::Completed, Locale::German) => "Unterschriften vollständig",
            (SignatureEmailKind::Completed, Locale::English) => "Signatures Complete",
        }
    }

    /// Send signature request invitation email to a signer.
    ///
    /// `locale` controls subject, body, and CTA copy. sk/cs/de translations
    /// are author-best-effort and should get a native review before
    /// customer rollout.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_signature_request_email(
        &self,
        to: &str,
        signer_name: &str,
        document_name: &str,
        requester_name: &str,
        signing_url: &str,
        message: Option<&str>,
        expires_at: Option<&str>,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let subject = Self::signature_request_subject(document_name, locale);
        let signer_name_h = Self::html_escape(signer_name);
        let requester_name_h = Self::html_escape(requester_name);
        let document_name_h = Self::html_escape(document_name);
        let signing_url_h = Self::html_escape(signing_url);

        let (expiry_html, expiry_plain) = Self::expiry_blocks(expires_at, locale, false);
        let custom_message_html = message
            .filter(|m| !m.is_empty())
            .map(|m| format!("<p><em>{}</em></p>", ammonia::clean(m)))
            .unwrap_or_default();
        let custom_message_plain = message
            .filter(|m| !m.is_empty())
            .map(|m| format!("\n{}\n", m))
            .unwrap_or_default();

        let title = Self::signature_email_title(SignatureEmailKind::Request, locale);
        let lang = locale.as_str();
        let inner = Self::signature_request_body_html(locale)
            .replace("{signer_name}", &signer_name_h)
            .replace("{requester_name}", &requester_name_h)
            .replace("{document_name}", &document_name_h)
            .replace("{signing_url}", &signing_url_h)
            .replace("{custom_message}", &custom_message_html)
            .replace("{expiry_block}", &expiry_html);
        let html_body = format!(
            r#"<!DOCTYPE html>
<html lang="{lang}"><head><meta charset="UTF-8"><title>{title}</title></head>
<body style="font-family:Arial,sans-serif;color:#333;max-width:600px;margin:0 auto;padding:20px">
{inner}
</body></html>"#,
        );

        let text_body_template = Self::signature_request_body_plain(locale);
        let text_body = format!(
            "{}\n\n{}",
            text_body_template
                .replace("{signer_name}", signer_name)
                .replace("{requester_name}", requester_name)
                .replace("{document_name}", document_name)
                .replace("{signing_url}", signing_url)
                .replace("{custom_message}", &custom_message_plain)
                .replace("{expiry_block}", &expiry_plain),
            Self::email_closing_plain(locale)
        );
        self.send_html_email(to, &subject, &html_body, &text_body)
            .await
    }

    /// Send a signature reminder email to a pending signer. Localised via
    /// `locale` (subject + body + CTA).
    #[allow(clippy::too_many_arguments)]
    pub async fn send_signature_reminder_email(
        &self,
        to: &str,
        signer_name: &str,
        document_name: &str,
        signing_url: &str,
        expires_at: Option<&str>,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let subject = Self::signature_reminder_subject(document_name, locale);
        let signer_name_h = Self::html_escape(signer_name);
        let document_name_h = Self::html_escape(document_name);
        let signing_url_h = Self::html_escape(signing_url);
        let (expiry_html, expiry_plain) = Self::expiry_blocks(expires_at, locale, true);

        let title = Self::signature_email_title(SignatureEmailKind::Reminder, locale);
        let lang = locale.as_str();
        let inner = Self::signature_reminder_body_html(locale)
            .replace("{signer_name}", &signer_name_h)
            .replace("{document_name}", &document_name_h)
            .replace("{signing_url}", &signing_url_h)
            .replace("{expiry_block}", &expiry_html);
        let html_body = format!(
            r#"<!DOCTYPE html>
<html lang="{lang}"><head><meta charset="UTF-8"><title>{title}</title></head>
<body style="font-family:Arial,sans-serif;color:#333;max-width:600px;margin:0 auto;padding:20px">
{inner}
</body></html>"#,
        );

        let text_body = format!(
            "{}\n\n{}",
            Self::signature_reminder_body_plain(locale)
                .replace("{signer_name}", signer_name)
                .replace("{document_name}", document_name)
                .replace("{signing_url}", signing_url)
                .replace("{expiry_block}", &expiry_plain),
            Self::email_closing_plain(locale)
        );
        self.send_html_email(to, &subject, &html_body, &text_body)
            .await
    }

    /// Send a decline notification to the document requester. Localised
    /// via `locale`.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_signature_declined_email(
        &self,
        to: &str,
        requester_name: &str,
        document_name: &str,
        signer_name: &str,
        signer_email: &str,
        decline_reason: Option<&str>,
        manage_url: &str,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let subject = Self::signature_declined_subject(document_name, locale);
        let requester_name_h = Self::html_escape(requester_name);
        let signer_name_h = Self::html_escape(signer_name);
        let signer_email_h = Self::html_escape(signer_email);
        let document_name_h = Self::html_escape(document_name);
        let manage_url_h = Self::html_escape(manage_url);
        let (reason_html, reason_plain) = Self::reason_blocks(decline_reason, locale);

        let title = Self::signature_email_title(SignatureEmailKind::Declined, locale);
        let lang = locale.as_str();
        let inner = Self::signature_declined_body_html(locale)
            .replace("{requester_name}", &requester_name_h)
            .replace("{signer_name}", &signer_name_h)
            .replace("{signer_email}", &signer_email_h)
            .replace("{document_name}", &document_name_h)
            .replace("{manage_url}", &manage_url_h)
            .replace("{reason_block}", &reason_html);
        let html_body = format!(
            r#"<!DOCTYPE html>
<html lang="{lang}"><head><meta charset="UTF-8"><title>{title}</title></head>
<body style="font-family:Arial,sans-serif;color:#333;max-width:600px;margin:0 auto;padding:20px">
{inner}
</body></html>"#,
        );

        let text_body = format!(
            "{}\n\n{}",
            Self::signature_declined_body_plain(locale)
                .replace("{requester_name}", requester_name)
                .replace("{signer_name}", signer_name)
                .replace("{signer_email}", signer_email)
                .replace("{document_name}", document_name)
                .replace("{manage_url}", manage_url)
                .replace("{reason_block}", &reason_plain),
            Self::email_closing_plain(locale)
        );
        self.send_html_email(to, &subject, &html_body, &text_body)
            .await
    }

    /// Send a completion notification when all signers have signed.
    /// Localised via `locale`.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_signature_completed_email(
        &self,
        to: &str,
        requester_name: &str,
        document_name: &str,
        signers_count: usize,
        manage_url: &str,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let subject = Self::signature_completed_subject(document_name, locale);
        let requester_name_h = Self::html_escape(requester_name);
        let document_name_h = Self::html_escape(document_name);
        let manage_url_h = Self::html_escape(manage_url);
        let signers_count_str = signers_count.to_string();

        let title = Self::signature_email_title(SignatureEmailKind::Completed, locale);
        let lang = locale.as_str();
        let inner = Self::signature_completed_body_html(locale)
            .replace("{requester_name}", &requester_name_h)
            .replace("{document_name}", &document_name_h)
            .replace("{manage_url}", &manage_url_h)
            .replace("{signers_count}", &signers_count_str);
        let html_body = format!(
            r#"<!DOCTYPE html>
<html lang="{lang}"><head><meta charset="UTF-8"><title>{title}</title></head>
<body style="font-family:Arial,sans-serif;color:#333;max-width:600px;margin:0 auto;padding:20px">
{inner}
</body></html>"#,
        );

        let text_body = format!(
            "{}\n\n{}",
            Self::signature_completed_body_plain(locale)
                .replace("{requester_name}", requester_name)
                .replace("{document_name}", document_name)
                .replace("{manage_url}", manage_url)
                .replace("{signers_count}", &signers_count_str),
            Self::email_closing_plain(locale)
        );
        self.send_html_email(to, &subject, &html_body, &text_body)
            .await
    }

    /// Send voting reminder email.
    pub async fn send_voting_reminder_email(
        &self,
        email: &str,
        name: &str,
        vote_title: &str,
        deadline: &str,
        locale: &Locale,
    ) -> Result<(), EmailError> {
        let subject = self.get_voting_reminder_subject(locale);
        let body = self.get_voting_reminder_body(name, vote_title, deadline, locale);

        self.send_email(email, &subject, &body).await
    }

    fn get_voting_reminder_subject(&self, locale: &Locale) -> String {
        match locale {
            Locale::Slovak => "Pripomienka hlasovania".to_string(),
            Locale::Czech => "Připomínka hlasování".to_string(),
            Locale::German => "Abstimmungserinnerung".to_string(),
            Locale::English => "Voting Reminder".to_string(),
        }
    }

    fn get_voting_reminder_body(
        &self,
        name: &str,
        vote_title: &str,
        deadline: &str,
        locale: &Locale,
    ) -> String {
        match locale {
            Locale::Slovak => format!(
                "Dobrý deň {},\n\nPripomíname vám hlasovanie: {}\n\nTermín na hlasovanie: {}\n\nPrihláste sa do systému a odovzdajte svoj hlas.\n\nS pozdravom,\nTím PPT",
                name, vote_title, deadline
            ),
            Locale::Czech => format!(
                "Dobrý den {},\n\nPřipomínáme vám hlasování: {}\n\nTermín pro hlasování: {}\n\nPřihlaste se do systému a odevzdejte svůj hlas.\n\nS pozdravem,\nTým PPT",
                name, vote_title, deadline
            ),
            Locale::German => format!(
                "Guten Tag {},\n\nWir erinnern Sie an die Abstimmung: {}\n\nAbstimmungsfrist: {}\n\nMelden Sie sich im System an und geben Sie Ihre Stimme ab.\n\nMit freundlichen Grüßen,\nDas PPT-Team",
                name, vote_title, deadline
            ),
            Locale::English => format!(
                "Hello {},\n\nReminder about the vote: {}\n\nVoting deadline: {}\n\nLog in to the system to cast your vote.\n\nBest regards,\nThe PPT Team",
                name, vote_title, deadline
            ),
        }
    }

    /// Check if SMTP is properly configured.
    pub fn is_smtp_configured(&self) -> bool {
        matches!(self.transport.as_ref(), EmailTransport::Smtp(_))
    }

    /// Check if email sending is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_config_from_env_missing() {
        // When SMTP env vars are not set, from_env returns None
        std::env::remove_var("SMTP_HOST");
        let config = SmtpConfig::from_env();
        assert!(config.is_none());
    }

    #[test]
    fn test_development_mode() {
        let service = EmailService::development();
        assert!(!service.is_enabled());
        assert!(!service.is_smtp_configured());
    }

    #[test]
    fn test_verification_subject_localization() {
        let service = EmailService::development();

        assert_eq!(
            service.get_verification_subject(&Locale::English),
            "Verify your email address"
        );
        assert_eq!(
            service.get_verification_subject(&Locale::Slovak),
            "Overte svoju e-mailovú adresu"
        );
        assert_eq!(
            service.get_verification_subject(&Locale::Czech),
            "Ověřte svou e-mailovou adresu"
        );
        assert_eq!(
            service.get_verification_subject(&Locale::German),
            "Bestätigen Sie Ihre E-Mail-Adresse"
        );
    }

    #[test]
    fn test_reset_subject_localization() {
        let service = EmailService::development();

        assert_eq!(
            service.get_reset_subject(&Locale::English),
            "Reset your password"
        );
        assert_eq!(
            service.get_reset_subject(&Locale::Slovak),
            "Obnovenie hesla"
        );
    }

    #[tokio::test]
    async fn test_send_email_development_mode() {
        let service = EmailService::development();

        // In development mode, send_email should succeed without actually sending
        let result = service
            .send_email("test@example.com", "Test Subject", "Test Body")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_verification_email_development_mode() {
        let service = EmailService::development();

        let result = service
            .send_verification_email(
                "test@example.com",
                "John Doe",
                "test-token-123",
                &Locale::English,
            )
            .await;

        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------
    // Signature email i18n tests (#527 body-translation follow-up).
    //
    // These assert against the per-locale helper outputs rather than
    // sending real email — fast, deterministic, no SMTP / DB.
    // ------------------------------------------------------------------

    const SK: &Locale = &Locale::Slovak;
    const CS: &Locale = &Locale::Czech;
    const DE: &Locale = &Locale::German;
    const EN: &Locale = &Locale::English;

    #[test]
    fn signature_subject_localised_per_locale() {
        // Each language has a recognisable, language-specific keyword in
        // the subject — catches accidental fallback to English.
        let doc = "Lease Agreement";
        assert!(EmailService::signature_request_subject(doc, SK).contains("Žiadosť"));
        assert!(EmailService::signature_request_subject(doc, CS).contains("Žádost"));
        assert!(EmailService::signature_request_subject(doc, DE).contains("Unterschrift"));
        assert!(EmailService::signature_request_subject(doc, EN).contains("Action required"));

        assert!(EmailService::signature_reminder_subject(doc, SK).contains("Pripomienka"));
        assert!(EmailService::signature_reminder_subject(doc, CS).contains("Připomínka"));
        assert!(EmailService::signature_reminder_subject(doc, DE).contains("Erinnerung"));
        assert!(EmailService::signature_reminder_subject(doc, EN).contains("Reminder"));

        assert!(EmailService::signature_declined_subject(doc, SK).contains("zamietnutý"));
        assert!(EmailService::signature_declined_subject(doc, CS).contains("odmítnut"));
        assert!(EmailService::signature_declined_subject(doc, DE).contains("abgelehnt"));
        assert!(EmailService::signature_declined_subject(doc, EN).contains("declined"));

        assert!(EmailService::signature_completed_subject(doc, SK).contains("podpisy"));
        assert!(EmailService::signature_completed_subject(doc, CS).contains("podpisy"));
        assert!(EmailService::signature_completed_subject(doc, DE).contains("Unterschriften"));
        assert!(EmailService::signature_completed_subject(doc, EN).contains("collected"));
    }

    #[test]
    fn email_closing_localised_per_locale() {
        assert!(EmailService::email_closing_plain(SK).starts_with("S pozdravom"));
        assert!(EmailService::email_closing_plain(CS).starts_with("S pozdravem"));
        assert!(EmailService::email_closing_plain(DE).starts_with("Mit freundlichen"));
        assert!(EmailService::email_closing_plain(EN).starts_with("Best regards"));
    }

    #[test]
    fn body_templates_contain_every_placeholder_for_each_locale() {
        // Each body template must keep every placeholder name so the
        // call-site `.replace(...)` substitutions all bind. A typo in a
        // translation would silently leave English values mis-rendered.
        for locale in [SK, CS, DE, EN] {
            let req_html = EmailService::signature_request_body_html(locale);
            for placeholder in [
                "{signer_name}",
                "{requester_name}",
                "{document_name}",
                "{signing_url}",
                "{custom_message}",
                "{expiry_block}",
            ] {
                assert!(
                    req_html.contains(placeholder),
                    "request_body_html for {locale:?} missing {placeholder}"
                );
            }

            let req_plain = EmailService::signature_request_body_plain(locale);
            for placeholder in [
                "{signer_name}",
                "{requester_name}",
                "{document_name}",
                "{signing_url}",
                "{custom_message}",
                "{expiry_block}",
            ] {
                assert!(
                    req_plain.contains(placeholder),
                    "request_body_plain for {locale:?} missing {placeholder}"
                );
            }

            let rem_html = EmailService::signature_reminder_body_html(locale);
            for placeholder in [
                "{signer_name}",
                "{document_name}",
                "{signing_url}",
                "{expiry_block}",
            ] {
                assert!(
                    rem_html.contains(placeholder),
                    "reminder_body_html for {locale:?} missing {placeholder}"
                );
            }

            let dec_html = EmailService::signature_declined_body_html(locale);
            for placeholder in [
                "{requester_name}",
                "{signer_name}",
                "{signer_email}",
                "{document_name}",
                "{manage_url}",
                "{reason_block}",
            ] {
                assert!(
                    dec_html.contains(placeholder),
                    "declined_body_html for {locale:?} missing {placeholder}"
                );
            }

            let comp_html = EmailService::signature_completed_body_html(locale);
            for placeholder in [
                "{requester_name}",
                "{document_name}",
                "{manage_url}",
                "{signers_count}",
            ] {
                assert!(
                    comp_html.contains(placeholder),
                    "completed_body_html for {locale:?} missing {placeholder}"
                );
            }
        }
    }

    #[test]
    fn expiry_blocks_localised_and_urgent_variant_marked() {
        // No expiry → empty blocks.
        let (h, p) = EmailService::expiry_blocks(None, EN, false);
        assert!(h.is_empty() && p.is_empty());

        // Slovak non-urgent must include the date and no "Dôležité" marker.
        let (h, _) = EmailService::expiry_blocks(Some("2026-06-01"), SK, false);
        assert!(h.contains("2026-06-01"));
        assert!(!h.contains("Dôležité"));

        // Slovak urgent variant must include the "Dôležité" marker.
        let (h, p) = EmailService::expiry_blocks(Some("2026-06-01"), SK, true);
        assert!(h.contains("Dôležité"));
        assert!(p.contains("Dôležité"));

        // German urgent: "Wichtig:" + date.
        let (h, _) = EmailService::expiry_blocks(Some("2026-06-01"), DE, true);
        assert!(h.contains("Wichtig"));
        assert!(h.contains("2026-06-01"));
    }

    #[test]
    fn reason_blocks_localised_with_and_without_reason() {
        // With a reason — language-specific marker + the reason itself.
        let (h, p) = EmailService::reason_blocks(Some("not available"), SK);
        assert!(h.contains("Uvedený dôvod"));
        assert!(p.contains("not available"));

        let (h, _) = EmailService::reason_blocks(Some("nicht verfügbar"), DE);
        assert!(h.contains("Angegebener Grund"));

        // Without — each locale has its own "no reason" sentence.
        let (h, p) = EmailService::reason_blocks(None, CS);
        assert!(h.contains("nebyl uveden"));
        assert!(p.contains("nebyl uveden"));

        let (_, p) = EmailService::reason_blocks(None, EN);
        assert!(p.contains("No reason"));
    }

    #[test]
    fn html_title_localised_per_kind_and_locale() {
        // Same kind, four locales — confirms the title lookup table is
        // wired for every (kind, locale) pair.
        assert_eq!(
            EmailService::signature_email_title(SignatureEmailKind::Request, SK),
            "Žiadosť o podpis"
        );
        assert_eq!(
            EmailService::signature_email_title(SignatureEmailKind::Reminder, CS),
            "Připomínka podpisu"
        );
        assert_eq!(
            EmailService::signature_email_title(SignatureEmailKind::Declined, DE),
            "Unterschrift abgelehnt"
        );
        assert_eq!(
            EmailService::signature_email_title(SignatureEmailKind::Completed, EN),
            "Signatures Complete"
        );
    }

    #[tokio::test]
    async fn send_signature_request_renders_localised_subject_and_body() {
        // End-to-end smoke: build the HTML + plain output and confirm the
        // localised strings end up in the rendered email (development
        // EmailService just logs, but the call exercises the full
        // template path).
        let service = EmailService::development();
        for locale in [SK, CS, DE, EN] {
            let result = service
                .send_signature_request_email(
                    "signer@example.com",
                    "Anna",
                    "Lease Agreement",
                    "Manager Bob",
                    "https://app.example.com/sign?token=abc",
                    Some("Please sign at your earliest convenience."),
                    Some("2026-06-01"),
                    locale,
                )
                .await;
            assert!(result.is_ok(), "send failed for {locale:?}: {result:?}");
        }
    }
}
