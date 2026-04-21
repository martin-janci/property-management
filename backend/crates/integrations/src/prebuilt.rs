//! Pre-Built Integrations (Epic 150, Story 150.4)
//!
//! Pre-built connectors for QuickBooks, Xero, Salesforce, HubSpot, Google Calendar,
//! Outlook, Slack, and Microsoft Teams.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::connector::{
    AuthConfig, ConnectorConfig, ConnectorError, HttpConnector, RateLimitConfig,
};

// ============================================
// QuickBooks Integration
// ============================================

/// QuickBooks API client.
pub struct QuickBooksClient {
    connector: HttpConnector,
    realm_id: String,
}

/// QuickBooks invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickBooksInvoice {
    #[serde(rename = "Id")]
    pub id: Option<String>,
    #[serde(rename = "DocNumber")]
    pub doc_number: Option<String>,
    #[serde(rename = "TxnDate")]
    pub txn_date: Option<String>,
    #[serde(rename = "DueDate")]
    pub due_date: Option<String>,
    #[serde(rename = "CustomerRef")]
    pub customer_ref: Option<QuickBooksRef>,
    #[serde(rename = "Line")]
    pub lines: Vec<QuickBooksInvoiceLine>,
    #[serde(rename = "TotalAmt")]
    pub total_amt: Option<f64>,
    #[serde(rename = "Balance")]
    pub balance: Option<f64>,
}

/// QuickBooks invoice line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickBooksInvoiceLine {
    #[serde(rename = "DetailType")]
    pub detail_type: String,
    #[serde(rename = "Amount")]
    pub amount: f64,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "SalesItemLineDetail")]
    pub sales_item_line_detail: Option<QuickBooksSalesItemLineDetail>,
}

/// QuickBooks sales item line detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickBooksSalesItemLineDetail {
    #[serde(rename = "ItemRef")]
    pub item_ref: Option<QuickBooksRef>,
    #[serde(rename = "Qty")]
    pub qty: Option<f64>,
    #[serde(rename = "UnitPrice")]
    pub unit_price: Option<f64>,
}

/// QuickBooks reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickBooksRef {
    pub value: String,
    pub name: Option<String>,
}

/// QuickBooks customer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickBooksCustomer {
    #[serde(rename = "Id")]
    pub id: Option<String>,
    #[serde(rename = "DisplayName")]
    pub display_name: String,
    #[serde(rename = "GivenName")]
    pub given_name: Option<String>,
    #[serde(rename = "FamilyName")]
    pub family_name: Option<String>,
    #[serde(rename = "PrimaryEmailAddr")]
    pub primary_email_addr: Option<QuickBooksEmailAddr>,
    #[serde(rename = "PrimaryPhone")]
    pub primary_phone: Option<QuickBooksPhone>,
    #[serde(rename = "BillAddr")]
    pub bill_addr: Option<QuickBooksAddress>,
}

/// QuickBooks email address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickBooksEmailAddr {
    #[serde(rename = "Address")]
    pub address: String,
}

/// QuickBooks phone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickBooksPhone {
    #[serde(rename = "FreeFormNumber")]
    pub free_form_number: String,
}

/// QuickBooks address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickBooksAddress {
    #[serde(rename = "Line1")]
    pub line1: Option<String>,
    #[serde(rename = "City")]
    pub city: Option<String>,
    #[serde(rename = "PostalCode")]
    pub postal_code: Option<String>,
    #[serde(rename = "Country")]
    pub country: Option<String>,
}

/// QuickBooks payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickBooksPayment {
    #[serde(rename = "Id")]
    pub id: Option<String>,
    #[serde(rename = "TxnDate")]
    pub txn_date: Option<String>,
    #[serde(rename = "TotalAmt")]
    pub total_amt: f64,
    #[serde(rename = "CustomerRef")]
    pub customer_ref: QuickBooksRef,
    #[serde(rename = "Line")]
    pub lines: Option<Vec<QuickBooksPaymentLine>>,
}

/// QuickBooks payment line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickBooksPaymentLine {
    #[serde(rename = "Amount")]
    pub amount: f64,
    #[serde(rename = "LinkedTxn")]
    pub linked_txn: Vec<QuickBooksLinkedTxn>,
}

/// QuickBooks linked transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickBooksLinkedTxn {
    #[serde(rename = "TxnId")]
    pub txn_id: String,
    #[serde(rename = "TxnType")]
    pub txn_type: String,
}

impl QuickBooksClient {
    /// Create a new QuickBooks client.
    pub fn new(
        access_token: String,
        _refresh_token: String,
        realm_id: String,
        sandbox: bool,
    ) -> Result<Self, ConnectorError> {
        let base_url = if sandbox {
            "https://sandbox-quickbooks.api.intuit.com/v3/company"
        } else {
            "https://quickbooks.api.intuit.com/v3/company"
        };

        let config = ConnectorConfig::new("quickbooks", base_url)
            .with_auth(AuthConfig::BearerToken {
                token: access_token,
            })
            .with_rate_limit(RateLimitConfig {
                requests_per_window: 500,
                window_seconds: 60,
            })
            .with_timeout(30000)
            .with_header("Accept", "application/json");

        let connector = HttpConnector::new(config)?;

        Ok(Self {
            connector,
            realm_id,
        })
    }

    /// List invoices.
    pub async fn list_invoices(
        &self,
        query: Option<&str>,
    ) -> Result<Vec<QuickBooksInvoice>, ConnectorError> {
        let path = format!(
            "/{}/query?query=SELECT * FROM Invoice {}",
            self.realm_id,
            query.map(|q| format!("WHERE {}", q)).unwrap_or_default()
        );

        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.get(&path).await?;

        if let Some(data) = result.data {
            if let Some(invoices) = data.get("QueryResponse").and_then(|qr| qr.get("Invoice")) {
                return serde_json::from_value(invoices.clone())
                    .map_err(|e| ConnectorError::SerializationError(e.to_string()));
            }
        }

        Ok(vec![])
    }

    /// Create invoice.
    pub async fn create_invoice(
        &self,
        invoice: &QuickBooksInvoice,
    ) -> Result<QuickBooksInvoice, ConnectorError> {
        let path = format!("/{}/invoice", self.realm_id);

        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.post(&path, invoice).await?;

        if let Some(data) = result.data {
            if let Some(inv) = data.get("Invoice") {
                return serde_json::from_value(inv.clone())
                    .map_err(|e| ConnectorError::SerializationError(e.to_string()));
            }
        }

        Err(ConnectorError::SerializationError(
            "Invalid response".to_string(),
        ))
    }

    /// List customers.
    pub async fn list_customers(
        &self,
        query: Option<&str>,
    ) -> Result<Vec<QuickBooksCustomer>, ConnectorError> {
        let path = format!(
            "/{}/query?query=SELECT * FROM Customer {}",
            self.realm_id,
            query.map(|q| format!("WHERE {}", q)).unwrap_or_default()
        );

        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.get(&path).await?;

        if let Some(data) = result.data {
            if let Some(customers) = data.get("QueryResponse").and_then(|qr| qr.get("Customer")) {
                return serde_json::from_value(customers.clone())
                    .map_err(|e| ConnectorError::SerializationError(e.to_string()));
            }
        }

        Ok(vec![])
    }

    /// Record payment.
    pub async fn record_payment(
        &self,
        payment: &QuickBooksPayment,
    ) -> Result<QuickBooksPayment, ConnectorError> {
        let path = format!("/{}/payment", self.realm_id);

        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.post(&path, payment).await?;

        if let Some(data) = result.data {
            if let Some(pmt) = data.get("Payment") {
                return serde_json::from_value(pmt.clone())
                    .map_err(|e| ConnectorError::SerializationError(e.to_string()));
            }
        }

        Err(ConnectorError::SerializationError(
            "Invalid response".to_string(),
        ))
    }
}

// ============================================
// Xero Integration
// ============================================

/// Xero API client.
pub struct XeroClient {
    connector: HttpConnector,
    #[allow(dead_code)]
    tenant_id: String,
}

/// Xero invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XeroInvoice {
    #[serde(rename = "InvoiceID")]
    pub invoice_id: Option<String>,
    #[serde(rename = "InvoiceNumber")]
    pub invoice_number: Option<String>,
    #[serde(rename = "Type")]
    pub invoice_type: String,
    #[serde(rename = "Contact")]
    pub contact: XeroContact,
    #[serde(rename = "DateString")]
    pub date_string: Option<String>,
    #[serde(rename = "DueDateString")]
    pub due_date_string: Option<String>,
    #[serde(rename = "LineItems")]
    pub line_items: Vec<XeroLineItem>,
    #[serde(rename = "Status")]
    pub status: Option<String>,
    #[serde(rename = "Total")]
    pub total: Option<f64>,
    #[serde(rename = "AmountDue")]
    pub amount_due: Option<f64>,
}

/// Xero contact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XeroContact {
    #[serde(rename = "ContactID")]
    pub contact_id: Option<String>,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "EmailAddress")]
    pub email_address: Option<String>,
    #[serde(rename = "FirstName")]
    pub first_name: Option<String>,
    #[serde(rename = "LastName")]
    pub last_name: Option<String>,
}

/// Xero line item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XeroLineItem {
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "Quantity")]
    pub quantity: f64,
    #[serde(rename = "UnitAmount")]
    pub unit_amount: f64,
    #[serde(rename = "AccountCode")]
    pub account_code: Option<String>,
    #[serde(rename = "TaxType")]
    pub tax_type: Option<String>,
}

/// Xero payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XeroPayment {
    #[serde(rename = "PaymentID")]
    pub payment_id: Option<String>,
    #[serde(rename = "Invoice")]
    pub invoice: XeroInvoiceRef,
    #[serde(rename = "Account")]
    pub account: XeroAccountRef,
    #[serde(rename = "Date")]
    pub date: String,
    #[serde(rename = "Amount")]
    pub amount: f64,
}

/// Xero invoice reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XeroInvoiceRef {
    #[serde(rename = "InvoiceID")]
    pub invoice_id: String,
}

/// Xero account reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XeroAccountRef {
    #[serde(rename = "AccountID")]
    pub account_id: Option<String>,
    #[serde(rename = "Code")]
    pub code: Option<String>,
}

impl XeroClient {
    /// Create a new Xero client.
    pub fn new(access_token: String, tenant_id: String) -> Result<Self, ConnectorError> {
        let config = ConnectorConfig::new("xero", "https://api.xero.com/api.xro/2.0")
            .with_auth(AuthConfig::BearerToken {
                token: access_token,
            })
            .with_rate_limit(RateLimitConfig {
                requests_per_window: 60,
                window_seconds: 60,
            })
            .with_timeout(30000)
            .with_header("xero-tenant-id", &tenant_id);

        let connector = HttpConnector::new(config)?;

        Ok(Self {
            connector,
            tenant_id,
        })
    }

    /// List invoices.
    pub async fn list_invoices(&self) -> Result<Vec<XeroInvoice>, ConnectorError> {
        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.get("/Invoices").await?;

        if let Some(data) = result.data {
            if let Some(invoices) = data.get("Invoices") {
                return serde_json::from_value(invoices.clone())
                    .map_err(|e| ConnectorError::SerializationError(e.to_string()));
            }
        }

        Ok(vec![])
    }

    /// Create invoice.
    pub async fn create_invoice(
        &self,
        invoice: &XeroInvoice,
    ) -> Result<XeroInvoice, ConnectorError> {
        let body = serde_json::json!({
            "Invoices": [invoice]
        });

        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.post("/Invoices", &body).await?;

        if let Some(data) = result.data {
            if let Some(invoices) = data.get("Invoices").and_then(|i| i.as_array()) {
                if let Some(inv) = invoices.first() {
                    return serde_json::from_value(inv.clone())
                        .map_err(|e| ConnectorError::SerializationError(e.to_string()));
                }
            }
        }

        Err(ConnectorError::SerializationError(
            "Invalid response".to_string(),
        ))
    }

    /// List contacts.
    pub async fn list_contacts(&self) -> Result<Vec<XeroContact>, ConnectorError> {
        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.get("/Contacts").await?;

        if let Some(data) = result.data {
            if let Some(contacts) = data.get("Contacts") {
                return serde_json::from_value(contacts.clone())
                    .map_err(|e| ConnectorError::SerializationError(e.to_string()));
            }
        }

        Ok(vec![])
    }

    /// Record payment.
    pub async fn record_payment(
        &self,
        payment: &XeroPayment,
    ) -> Result<XeroPayment, ConnectorError> {
        let body = serde_json::json!({
            "Payments": [payment]
        });

        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.post("/Payments", &body).await?;

        if let Some(data) = result.data {
            if let Some(payments) = data.get("Payments").and_then(|p| p.as_array()) {
                if let Some(pmt) = payments.first() {
                    return serde_json::from_value(pmt.clone())
                        .map_err(|e| ConnectorError::SerializationError(e.to_string()));
                }
            }
        }

        Err(ConnectorError::SerializationError(
            "Invalid response".to_string(),
        ))
    }
}

// ============================================
// Slack Integration
// ============================================

/// Slack API client.
pub struct SlackClient {
    connector: HttpConnector,
}

/// Slack message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackMessage {
    pub channel: String,
    pub text: Option<String>,
    pub blocks: Option<Vec<SlackBlock>>,
    pub attachments: Option<Vec<SlackAttachment>>,
    pub thread_ts: Option<String>,
    pub mrkdwn: Option<bool>,
}

/// Slack block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<SlackTextObject>,
    pub elements: Option<Vec<serde_json::Value>>,
    pub accessory: Option<serde_json::Value>,
}

/// Slack text object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackTextObject {
    #[serde(rename = "type")]
    pub text_type: String,
    pub text: String,
    pub emoji: Option<bool>,
}

/// Slack attachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackAttachment {
    pub color: Option<String>,
    pub fallback: Option<String>,
    pub title: Option<String>,
    pub title_link: Option<String>,
    pub text: Option<String>,
    pub fields: Option<Vec<SlackAttachmentField>>,
    pub footer: Option<String>,
    pub ts: Option<i64>,
}

/// Slack attachment field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackAttachmentField {
    pub title: String,
    pub value: String,
    pub short: Option<bool>,
}

/// Slack channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackChannel {
    pub id: String,
    pub name: String,
    pub is_channel: Option<bool>,
    pub is_private: Option<bool>,
    pub is_member: Option<bool>,
}

impl SlackClient {
    /// Create a new Slack client.
    pub fn new(bot_token: String) -> Result<Self, ConnectorError> {
        let config = ConnectorConfig::new("slack", "https://slack.com/api")
            .with_auth(AuthConfig::BearerToken { token: bot_token })
            .with_rate_limit(RateLimitConfig {
                requests_per_window: 50,
                window_seconds: 60,
            })
            .with_timeout(10000);

        let connector = HttpConnector::new(config)?;

        Ok(Self { connector })
    }

    /// Send a message.
    pub async fn send_message(&self, message: &SlackMessage) -> Result<String, ConnectorError> {
        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.post("/chat.postMessage", message).await?;

        if let Some(data) = result.data {
            if data.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                if let Some(ts) = data.get("ts").and_then(|v| v.as_str()) {
                    return Ok(ts.to_string());
                }
            }
            if let Some(error) = data.get("error").and_then(|v| v.as_str()) {
                return Err(ConnectorError::HttpError {
                    status: 400,
                    message: error.to_string(),
                });
            }
        }

        Err(ConnectorError::SerializationError(
            "Invalid response".to_string(),
        ))
    }

    /// List channels.
    pub async fn list_channels(&self) -> Result<Vec<SlackChannel>, ConnectorError> {
        let result: crate::connector::ExecutionResult<serde_json::Value> = self
            .connector
            .get("/conversations.list?types=public_channel,private_channel")
            .await?;

        if let Some(data) = result.data {
            if let Some(channels) = data.get("channels") {
                return serde_json::from_value(channels.clone())
                    .map_err(|e| ConnectorError::SerializationError(e.to_string()));
            }
        }

        Ok(vec![])
    }

    /// Create a notification block message.
    pub fn create_notification_message(
        channel: &str,
        title: &str,
        message: &str,
        link: Option<&str>,
        _color: Option<&str>,
    ) -> SlackMessage {
        let mut blocks = vec![
            SlackBlock {
                block_type: "header".to_string(),
                text: Some(SlackTextObject {
                    text_type: "plain_text".to_string(),
                    text: title.to_string(),
                    emoji: Some(true),
                }),
                elements: None,
                accessory: None,
            },
            SlackBlock {
                block_type: "section".to_string(),
                text: Some(SlackTextObject {
                    text_type: "mrkdwn".to_string(),
                    text: message.to_string(),
                    emoji: None,
                }),
                elements: None,
                accessory: None,
            },
        ];

        if let Some(url) = link {
            blocks.push(SlackBlock {
                block_type: "actions".to_string(),
                text: None,
                elements: Some(vec![serde_json::json!({
                    "type": "button",
                    "text": {
                        "type": "plain_text",
                        "text": "View Details",
                        "emoji": true
                    },
                    "url": url,
                    "style": "primary"
                })]),
                accessory: None,
            });
        }

        SlackMessage {
            channel: channel.to_string(),
            text: Some(format!("{}: {}", title, message)),
            blocks: Some(blocks),
            attachments: None,
            thread_ts: None,
            mrkdwn: Some(true),
        }
    }
}

// ============================================
// Microsoft Teams Integration
// ============================================

/// Microsoft Teams API client.
pub struct TeamsClient {
    connector: HttpConnector,
}

/// Teams message (adaptive card).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub attachments: Vec<TeamsAttachment>,
}

/// Teams attachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsAttachment {
    #[serde(rename = "contentType")]
    pub content_type: String,
    #[serde(rename = "contentUrl")]
    pub content_url: Option<String>,
    pub content: serde_json::Value,
}

/// Teams adaptive card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsAdaptiveCard {
    #[serde(rename = "type")]
    pub card_type: String,
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub body: Vec<serde_json::Value>,
    pub actions: Option<Vec<serde_json::Value>>,
}

impl TeamsClient {
    /// Create a new Teams client using webhook URL.
    pub fn new_webhook(webhook_url: String) -> Result<Self, ConnectorError> {
        let config = ConnectorConfig::new("teams", &webhook_url)
            .with_auth(AuthConfig::None)
            .with_timeout(10000);

        let connector = HttpConnector::new(config)?;

        Ok(Self { connector })
    }

    /// Send a message via webhook.
    pub async fn send_webhook_message(
        &self,
        card: &TeamsAdaptiveCard,
    ) -> Result<(), ConnectorError> {
        let message = TeamsMessage {
            message_type: "message".to_string(),
            attachments: vec![TeamsAttachment {
                content_type: "application/vnd.microsoft.card.adaptive".to_string(),
                content_url: None,
                content: serde_json::to_value(card)
                    .map_err(|e| ConnectorError::SerializationError(e.to_string()))?,
            }],
        };

        let _result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.post("", &message).await?;

        Ok(())
    }

    /// Create a notification adaptive card.
    pub fn create_notification_card(
        title: &str,
        message: &str,
        link: Option<&str>,
        _theme_color: Option<&str>,
    ) -> TeamsAdaptiveCard {
        let body: Vec<serde_json::Value> = vec![
            serde_json::json!({
                "type": "TextBlock",
                "size": "Large",
                "weight": "Bolder",
                "text": title,
                "wrap": true,
                "style": "heading"
            }),
            serde_json::json!({
                "type": "TextBlock",
                "text": message,
                "wrap": true
            }),
        ];

        let actions = link.map(|url| {
            vec![serde_json::json!({
                "type": "Action.OpenUrl",
                "title": "View Details",
                "url": url
            })]
        });

        TeamsAdaptiveCard {
            card_type: "AdaptiveCard".to_string(),
            schema: "http://adaptivecards.io/schemas/adaptive-card.json".to_string(),
            version: "1.4".to_string(),
            body,
            actions,
        }
    }
}

// ============================================
// Salesforce Integration
// ============================================

/// Salesforce API client.
pub struct SalesforceClient {
    connector: HttpConnector,
    #[allow(dead_code)]
    instance_url: String,
}

/// Salesforce contact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesforceContact {
    #[serde(rename = "Id")]
    pub id: Option<String>,
    #[serde(rename = "FirstName")]
    pub first_name: Option<String>,
    #[serde(rename = "LastName")]
    pub last_name: String,
    #[serde(rename = "Email")]
    pub email: Option<String>,
    #[serde(rename = "Phone")]
    pub phone: Option<String>,
    #[serde(rename = "AccountId")]
    pub account_id: Option<String>,
}

/// Salesforce lead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesforceLead {
    #[serde(rename = "Id")]
    pub id: Option<String>,
    #[serde(rename = "FirstName")]
    pub first_name: Option<String>,
    #[serde(rename = "LastName")]
    pub last_name: String,
    #[serde(rename = "Company")]
    pub company: String,
    #[serde(rename = "Email")]
    pub email: Option<String>,
    #[serde(rename = "Phone")]
    pub phone: Option<String>,
    #[serde(rename = "Status")]
    pub status: Option<String>,
}

/// Salesforce opportunity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesforceOpportunity {
    #[serde(rename = "Id")]
    pub id: Option<String>,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "AccountId")]
    pub account_id: Option<String>,
    #[serde(rename = "Amount")]
    pub amount: Option<f64>,
    #[serde(rename = "StageName")]
    pub stage_name: String,
    #[serde(rename = "CloseDate")]
    pub close_date: String,
}

impl SalesforceClient {
    /// Create a new Salesforce client.
    pub fn new(access_token: String, instance_url: String) -> Result<Self, ConnectorError> {
        let api_url = format!("{}/services/data/v58.0", instance_url);

        let config = ConnectorConfig::new("salesforce", &api_url)
            .with_auth(AuthConfig::BearerToken {
                token: access_token,
            })
            .with_rate_limit(RateLimitConfig {
                requests_per_window: 100,
                window_seconds: 60,
            })
            .with_timeout(30000);

        let connector = HttpConnector::new(config)?;

        Ok(Self {
            connector,
            instance_url,
        })
    }

    /// Query records using SOQL.
    pub async fn query<T>(&self, soql: &str) -> Result<Vec<T>, ConnectorError>
    where
        T: serde::de::DeserializeOwned,
    {
        let encoded_query = urlencoding::encode(soql);
        let path = format!("/query/?q={}", encoded_query);

        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.get(&path).await?;

        if let Some(data) = result.data {
            if let Some(records) = data.get("records") {
                return serde_json::from_value(records.clone())
                    .map_err(|e| ConnectorError::SerializationError(e.to_string()));
            }
        }

        Ok(vec![])
    }

    /// Create a record.
    pub async fn create<T>(&self, object_type: &str, record: &T) -> Result<String, ConnectorError>
    where
        T: Serialize,
    {
        let path = format!("/sobjects/{}", object_type);

        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.post(&path, record).await?;

        if let Some(data) = result.data {
            if let Some(id) = data.get("id").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }

        Err(ConnectorError::SerializationError(
            "Invalid response".to_string(),
        ))
    }

    /// Update a record.
    pub async fn update<T>(
        &self,
        object_type: &str,
        id: &str,
        record: &T,
    ) -> Result<(), ConnectorError>
    where
        T: Serialize,
    {
        let path = format!("/sobjects/{}/{}", object_type, id);

        self.connector
            .patch::<serde_json::Value, T>(&path, record)
            .await?;

        Ok(())
    }

    /// List contacts.
    pub async fn list_contacts(&self) -> Result<Vec<SalesforceContact>, ConnectorError> {
        self.query("SELECT Id, FirstName, LastName, Email, Phone, AccountId FROM Contact")
            .await
    }

    /// List leads.
    pub async fn list_leads(&self) -> Result<Vec<SalesforceLead>, ConnectorError> {
        self.query("SELECT Id, FirstName, LastName, Company, Email, Phone, Status FROM Lead")
            .await
    }

    /// Create lead.
    pub async fn create_lead(&self, lead: &SalesforceLead) -> Result<String, ConnectorError> {
        self.create("Lead", lead).await
    }
}

// ============================================
// HubSpot Integration
// ============================================

/// HubSpot API client.
pub struct HubSpotClient {
    connector: HttpConnector,
}

/// HubSpot contact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubSpotContact {
    pub id: Option<String>,
    pub properties: HubSpotContactProperties,
}

/// HubSpot contact properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubSpotContactProperties {
    pub email: Option<String>,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
}

/// HubSpot deal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubSpotDeal {
    pub id: Option<String>,
    pub properties: HubSpotDealProperties,
}

/// HubSpot deal properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubSpotDealProperties {
    pub dealname: String,
    pub amount: Option<String>,
    pub dealstage: Option<String>,
    pub closedate: Option<String>,
    pub pipeline: Option<String>,
}

impl HubSpotClient {
    /// Create a new HubSpot client.
    pub fn new(access_token: String) -> Result<Self, ConnectorError> {
        let config = ConnectorConfig::new("hubspot", "https://api.hubapi.com")
            .with_auth(AuthConfig::BearerToken {
                token: access_token,
            })
            .with_rate_limit(RateLimitConfig {
                requests_per_window: 100,
                window_seconds: 10,
            })
            .with_timeout(30000);

        let connector = HttpConnector::new(config)?;

        Ok(Self { connector })
    }

    /// List contacts.
    pub async fn list_contacts(&self) -> Result<Vec<HubSpotContact>, ConnectorError> {
        let result: crate::connector::ExecutionResult<serde_json::Value> = self
            .connector
            .get("/crm/v3/objects/contacts?properties=email,firstname,lastname,phone,company")
            .await?;

        if let Some(data) = result.data {
            if let Some(results) = data.get("results") {
                return serde_json::from_value(results.clone())
                    .map_err(|e| ConnectorError::SerializationError(e.to_string()));
            }
        }

        Ok(vec![])
    }

    /// Create contact.
    pub async fn create_contact(
        &self,
        contact: &HubSpotContact,
    ) -> Result<HubSpotContact, ConnectorError> {
        let body = serde_json::json!({
            "properties": contact.properties
        });

        let result: crate::connector::ExecutionResult<serde_json::Value> = self
            .connector
            .post("/crm/v3/objects/contacts", &body)
            .await?;

        if let Some(data) = result.data {
            return serde_json::from_value(data)
                .map_err(|e| ConnectorError::SerializationError(e.to_string()));
        }

        Err(ConnectorError::SerializationError(
            "Invalid response".to_string(),
        ))
    }

    /// List deals.
    pub async fn list_deals(&self) -> Result<Vec<HubSpotDeal>, ConnectorError> {
        let result: crate::connector::ExecutionResult<serde_json::Value> = self
            .connector
            .get("/crm/v3/objects/deals?properties=dealname,amount,dealstage,closedate,pipeline")
            .await?;

        if let Some(data) = result.data {
            if let Some(results) = data.get("results") {
                return serde_json::from_value(results.clone())
                    .map_err(|e| ConnectorError::SerializationError(e.to_string()));
            }
        }

        Ok(vec![])
    }

    /// Create deal.
    pub async fn create_deal(&self, deal: &HubSpotDeal) -> Result<HubSpotDeal, ConnectorError> {
        let body = serde_json::json!({
            "properties": deal.properties
        });

        let result: crate::connector::ExecutionResult<serde_json::Value> =
            self.connector.post("/crm/v3/objects/deals", &body).await?;

        if let Some(data) = result.data {
            return serde_json::from_value(data)
                .map_err(|e| ConnectorError::SerializationError(e.to_string()));
        }

        Err(ConnectorError::SerializationError(
            "Invalid response".to_string(),
        ))
    }
}

// ============================================
// Sync Results
// ============================================

/// Integration sync result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationSyncResult {
    pub integration_type: String,
    pub records_created: i32,
    pub records_updated: i32,
    pub records_deleted: i32,
    pub records_skipped: i32,
    pub errors: Vec<SyncError>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: i64,
}

/// Sync error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncError {
    pub record_id: Option<String>,
    pub error_type: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl IntegrationSyncResult {
    /// Create a new sync result.
    pub fn new(integration_type: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            integration_type: integration_type.into(),
            records_created: 0,
            records_updated: 0,
            records_deleted: 0,
            records_skipped: 0,
            errors: vec![],
            started_at: now,
            completed_at: now,
            duration_ms: 0,
        }
    }

    /// Complete the sync result.
    pub fn complete(mut self) -> Self {
        self.completed_at = Utc::now();
        self.duration_ms = (self.completed_at - self.started_at).num_milliseconds();
        self
    }

    /// Add an error.
    pub fn add_error(&mut self, error: SyncError) {
        self.errors.push(error);
    }

    /// Check if sync was successful.
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get total records processed.
    pub fn total_processed(&self) -> i32 {
        self.records_created + self.records_updated + self.records_deleted + self.records_skipped
    }
}
