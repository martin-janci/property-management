# Invoicing & Accounting Product — Use Cases

> **Product code:** `ACC` (standalone online invoicing + light-accounting SaaS).
> **Provenance:** Generalized from competitor analysis of a CZ/SK online invoicing SaaS. **Vendor-neutral**, functionality only.
> **Companion:** epics in [`epics.md`](epics.md). IDs follow the repo convention `UC-ACC-XX.Y` (XX = category, Y = sequence).

## Actors

### Account Level
- **Owner / Admin** — Account owner; manages company/agenda, subscription, users, global settings.
- **User (Staff)** — Team member with role-based access (issue documents, manage contacts, etc.).
- **Accountant** — External collaborator (often read/export); may manage multiple client companies; prepares filings.

### Counterparties
- **Customer** — Recipient of issued documents; limited self-service (view/pay an invoice via shared link).
- **Supplier / Vendor** — Counterparty on purchase/expense documents.

### System & External
- **System / Automation** — Scheduled, non-interactive actor (recurring invoices, reminders, bank sync, OCR/AI, FX refresh).
- **Bank** — Statement feed / payment data (external).
- **Payment Gateway** — Online card/bank payments (external).
- **Tax Authority** — Recipient of VAT/tax filings via export/e-file (external).
- **Business / VAT Registry** — Company autocomplete & VAT verification (external).
- **E-shop / External System** — Integrates via public API / webhooks (external).

### Actor Hierarchy
```
Owner / Admin
├── User (Staff)              [role-based: Standard / Read-only]
├── Accountant                [cross-company, read/export]
└── Company (Agenda)          [one Owner operates many]
        ├── Customer          [via shared link / email]
        ├── Supplier
        └── System / Automation
                ├── Bank (statements, payments)
                ├── Payment Gateway
                ├── Tax Authority (filings)
                ├── Business / VAT Registry (lookup)
                └── E-shop / External System (API)
```

---

## UC-ACC-01: Accounts, Organizations & Access (web, mobile)

### UC-ACC-01.1: Register Account
**Actor:** Owner/Admin
**Description:** A new user signs up and a first company/agenda is created.

### UC-ACC-01.2: Create Additional Company (Agenda)
**Actor:** Owner/Admin, Accountant
**Description:** Add another company under the same login, fully isolated from existing ones.

### UC-ACC-01.3: Switch Active Company
**Actor:** Owner/Admin, User, Accountant
**Description:** Change the active company context; all data and documents scope to it.

### UC-ACC-01.4: Invite User
**Actor:** Owner/Admin
**Description:** Invite a team member by email to join a company.

### UC-ACC-01.5: Assign Role & Permissions
**Actor:** Owner/Admin
**Description:** Grant a role (Admin, Standard, Read-only) controlling visible modules and actions.

### UC-ACC-01.6: Deactivate / Remove User
**Actor:** Owner/Admin
**Description:** Revoke a user's access while preserving their authored documents.

### UC-ACC-01.7: Grant Accountant Access
**Actor:** Owner/Admin
**Description:** Give an external accountant remote, ongoing access to documents and exports.

### UC-ACC-01.8: Manage Subscription / Plan
**Actor:** Owner/Admin
**Description:** View, upgrade, downgrade, or cancel the subscription tier and see usage limits.

### UC-ACC-01.9: Enable Two-Factor Authentication
**Actor:** Owner/Admin, User
**Description:** Add a second authentication factor to the account.

### UC-ACC-01.10: View Audit Log / Activity History
**Actor:** Owner/Admin
**Description:** Review who changed what and when across the company.

### UC-ACC-01.11: Export Account Data (GDPR)
**Actor:** Owner/Admin
**Description:** Export all company/personal data in a portable format.

### UC-ACC-01.12: Delete Account / Company (GDPR)
**Actor:** Owner/Admin
**Description:** Permanently delete a company/account with required confirmations and retention rules.

### UC-ACC-01.13: Log In / Single Sign-On
**Actor:** Owner/Admin, User, Accountant
**Description:** Authenticate via credentials (or SSO) and resume the last active company.

---

## UC-ACC-02: Company & Document Configuration (web)

### UC-ACC-02.1: Set Company Profile
**Actor:** Owner/Admin
**Description:** Maintain legal name, registration IDs, VAT status, addresses, contact details, and logo.

### UC-ACC-02.2: Configure Tax / VAT Mode
**Actor:** Owner/Admin, Accountant
**Description:** Choose VAT-payer vs non-payer and tax-records vs full-accounting mode; drives all document tax behavior.

### UC-ACC-02.3: Define Numbering Series
**Actor:** Owner/Admin
**Description:** Configure gapless numbering sequences per document type and period/year.

### UC-ACC-02.4: Customize Document Design
**Actor:** Owner/Admin
**Description:** Choose a layout/template and apply branding (logo, colors) to generated documents.

### UC-ACC-02.5: Configure Default Texts & Terms
**Actor:** Owner/Admin
**Description:** Set default payment terms, footers, headers, and standard notes for documents.

### UC-ACC-02.6: Manage Email Templates
**Actor:** Owner/Admin
**Description:** Edit subject/body templates used when sending documents and reminders.

### UC-ACC-02.7: Configure Languages & Localization
**Actor:** Owner/Admin
**Description:** Select document/UI languages, number/date formats, and per-document language.

### UC-ACC-02.8: Manage Units of Measure
**Actor:** Owner/Admin
**Description:** Define and edit units used on catalog items and document lines.

### UC-ACC-02.9: Configure VAT Rates
**Actor:** Owner/Admin
**Description:** Maintain applicable VAT rates and defaults.

### UC-ACC-02.10: Configure Bank Accounts on Documents
**Actor:** Owner/Admin
**Description:** Set which bank account(s) and payment details appear on issued documents.

### UC-ACC-02.11: Set Default Currency & Rounding
**Actor:** Owner/Admin
**Description:** Choose base currency and rounding rules (total and per-line precision).

---

## UC-ACC-03: Contacts & CRM (web, mobile)

### UC-ACC-03.1: Create Contact
**Actor:** User, Owner/Admin
**Description:** Add a customer or supplier with identifiers, addresses, and tax details.

### UC-ACC-03.2: Autocomplete from Business Registry
**Actor:** User; Business/VAT Registry
**Description:** Look up a company by name/ID and auto-fill official details.

### UC-ACC-03.3: Verify VAT Payer Status
**Actor:** User; Business/VAT Registry
**Description:** Validate a counterparty's VAT number and registration status.

### UC-ACC-03.4: Manage Multiple Addresses
**Actor:** User
**Description:** Keep separate billing and one or more delivery addresses per contact.

### UC-ACC-03.5: Edit / Merge / Deactivate Contact
**Actor:** User, Owner/Admin
**Description:** Maintain contact data, merge duplicates, or archive an inactive contact.

### UC-ACC-03.6: Tag / Segment Contacts
**Actor:** User
**Description:** Apply labels/segments for filtering and bulk operations.

### UC-ACC-03.7: View Contact Transaction History
**Actor:** User, Owner/Admin, Accountant
**Description:** See all issued/received documents and payments for a contact.

### UC-ACC-03.8: View Communication History
**Actor:** User, Owner/Admin
**Description:** Review emails and documents sent to the contact.

### UC-ACC-03.9: Set Per-Contact Defaults
**Actor:** User, Owner/Admin
**Description:** Define default currency, price level, payment terms, and discount per contact.

### UC-ACC-03.10: Import / Export Contacts
**Actor:** Owner/Admin
**Description:** Bulk-import contacts from a file and export the directory.

### UC-ACC-03.11: Track Receivables / Credit per Contact
**Actor:** Owner/Admin, Accountant
**Description:** Monitor outstanding balance and optional credit limit per contact.

---

## UC-ACC-04: Product & Price-List Catalog (web)

### UC-ACC-04.1: Create Catalog Item
**Actor:** User, Owner/Admin
**Description:** Add a good or service with name, code, description, unit, and VAT rate.

### UC-ACC-04.2: Set Prices (Levels, With/Without VAT)
**Actor:** User, Owner/Admin
**Description:** Define one or more price levels and net/gross pricing.

### UC-ACC-04.3: Assign VAT Rate & Unit
**Actor:** User
**Description:** Attach the correct VAT rate and unit of measure to an item.

### UC-ACC-04.4: Categorize / Tag Items
**Actor:** User
**Description:** Organize the catalog with categories and tags.

### UC-ACC-04.5: Manage Discounts
**Actor:** User, Owner/Admin
**Description:** Define item- or contact-level discounts applied on documents.

### UC-ACC-04.6: Import / Export Price List
**Actor:** Owner/Admin
**Description:** Bulk-import catalog items and export the price list.

### UC-ACC-04.7: Link Item to Stock
**Actor:** User
**Description:** Connect a catalog item to an inventory record for stock tracking.

### UC-ACC-04.8: Quick-Add Item from Document Line
**Actor:** User
**Description:** Create a new catalog item inline while editing a document line.

---

## UC-ACC-05: Sales Invoicing (web, mobile)

### UC-ACC-05.1: Create Issued Invoice
**Actor:** User, Owner/Admin
**Description:** Issue an invoice to a customer with header, due date, and payment details.

### UC-ACC-05.2: Add Line Items
**Actor:** User
**Description:** Add lines from the catalog or ad-hoc, with quantity, price, and VAT.

### UC-ACC-05.3: Apply Discounts, VAT & Rounding
**Actor:** User
**Description:** Apply line/document discounts and compute VAT and rounded totals.

### UC-ACC-05.4: Issue in Foreign Currency
**Actor:** User
**Description:** Create a document in another currency with an exchange rate and dual totals.

### UC-ACC-05.5: Generate Proforma / Advance Invoice
**Actor:** User
**Description:** Issue a request-for-payment (proforma/advance) before the taxable supply.

### UC-ACC-05.6: Create Tax / Settlement Document from Advance
**Actor:** User, Accountant
**Description:** Convert a received advance into a tax/settlement document and final invoice.

### UC-ACC-05.7: Issue Credit Note / Corrective Document
**Actor:** User, Owner/Admin
**Description:** Correct or cancel a prior invoice with a linked corrective document.

### UC-ACC-05.8: Generate Payment QR / Code
**Actor:** User, System
**Description:** Embed a scannable payment code with amount and reference on the document.

### UC-ACC-05.9: Preview & Generate PDF
**Actor:** User
**Description:** Render a print-ready PDF of the document.

### UC-ACC-05.10: Send Invoice by Email
**Actor:** User, System
**Description:** Email the document (PDF/link) to the customer using a template.

### UC-ACC-05.11: Send Invoice via Shareable Link
**Actor:** User
**Description:** Share a secure link where the customer can view and pay the invoice.

### UC-ACC-05.12: Export Invoice (PDF / ISDOC / XML)
**Actor:** User, Accountant, External System
**Description:** Export a document in a standard electronic format.

### UC-ACC-05.13: Attach Files to Document
**Actor:** User
**Description:** Add supporting attachments (contracts, photos) to a document.

### UC-ACC-05.14: Tag Documents
**Actor:** User
**Description:** Label documents for filtering and reporting.

### UC-ACC-05.15: Link Related Documents
**Actor:** User, System
**Description:** Maintain links across proforma ↔ invoice ↔ credit note ↔ delivery note.

### UC-ACC-05.16: Duplicate Invoice
**Actor:** User
**Description:** Create a new document by copying an existing one.

### UC-ACC-05.17: Track Invoice Status
**Actor:** User, System
**Description:** Reflect lifecycle: draft → issued → sent → paid / overdue / cancelled.

### UC-ACC-05.18: Bulk Document Actions
**Actor:** User, Owner/Admin
**Description:** Send, export, print, or tag multiple documents at once.

### UC-ACC-05.19: Set Decimal Precision per Line
**Actor:** User
**Description:** Control per-line decimal precision (e.g., up to 4 places).

### UC-ACC-05.20: Round Document Total
**Actor:** User, System
**Description:** Apply configured total rounding and show the rounding line.

---

## UC-ACC-06: Quotes, Orders & Delivery (web)

### UC-ACC-06.1: Create Price Quote / Offer
**Actor:** User
**Description:** Issue a quotation to a prospective customer.

### UC-ACC-06.2: Send Quote & Track Acceptance
**Actor:** User, Customer
**Description:** Send a quote and record its accepted/rejected status.

### UC-ACC-06.3: Convert Quote → Order / Invoice
**Actor:** User
**Description:** Generate an order or invoice from an accepted quote without re-keying.

### UC-ACC-06.4: Create Sales Order
**Actor:** User
**Description:** Record a received order to be fulfilled and invoiced.

### UC-ACC-06.5: Create Delivery Note
**Actor:** User
**Description:** Issue a delivery/dispatch note for goods, optionally drawing from stock.

### UC-ACC-06.6: Convert Order → Delivery / Invoice
**Actor:** User
**Description:** Progress an order into a delivery note and/or invoice.

### UC-ACC-06.7: Track Fulfillment Status
**Actor:** User
**Description:** Monitor order/delivery progress (open, partially fulfilled, complete).

---

## UC-ACC-07: Purchases & Expenses (web, mobile)

### UC-ACC-07.1: Record Received Invoice / Bill
**Actor:** User, Accountant
**Description:** Enter a supplier invoice with amounts, VAT, and due date.

### UC-ACC-07.2: Capture Expense
**Actor:** User
**Description:** Record a cost/expense document (receipt) against a category.

### UC-ACC-07.3: Upload Document to Inbox
**Actor:** User
**Description:** Add a scan/PDF/photo to a processing inbox for later posting.

### UC-ACC-07.4: AI / OCR Extract Document Data
**Actor:** System
**Description:** Automatically read supplier, amounts, VAT, and dates from an uploaded document.

### UC-ACC-07.5: Review & Post Extracted Document
**Actor:** User, Accountant
**Description:** Confirm or correct extracted data and post it as a received document.

### UC-ACC-07.6: Categorize Expense
**Actor:** User, Accountant
**Description:** Assign an expense category/cost type for reporting and tax.

### UC-ACC-07.7: Link Expense to Supplier
**Actor:** User
**Description:** Associate the document with a supplier contact.

### UC-ACC-07.8: Attach Scan / Receipt
**Actor:** User
**Description:** Keep the source image/PDF attached to the posted document.

### UC-ACC-07.9: Track Payable Status & Due Date
**Actor:** User, System
**Description:** Monitor unpaid/paid/overdue status of payables.

---

## UC-ACC-08: Cash & Bank (web)

### UC-ACC-08.1: Create Cash Receipt (Income)
**Actor:** User
**Description:** Record cash received with an income cash-register document.

### UC-ACC-08.2: Create Cash Payment (Expense)
**Actor:** User
**Description:** Record cash paid out with an expense cash-register document.

### UC-ACC-08.3: Manage Cash Registers
**Actor:** Owner/Admin
**Description:** Maintain one or more cash registers and their balances.

### UC-ACC-08.4: Add Bank Account
**Actor:** Owner/Admin
**Description:** Register a bank account for documents and reconciliation.

### UC-ACC-08.5: Import Bank Statement
**Actor:** User, System; Bank
**Description:** Import a statement (file or feed) of bank transactions.

### UC-ACC-08.6: Auto-Match Payments
**Actor:** System
**Description:** Automatically pair statement transactions to open documents by reference/amount.

### UC-ACC-08.7: Manually Match / Unmatch Payment
**Actor:** User
**Description:** Override matching to pair or unpair a transaction and document.

### UC-ACC-08.8: Manage Multiple Accounts / Registers
**Actor:** Owner/Admin
**Description:** Operate several bank accounts and cash registers in one company.

### UC-ACC-08.9: Integrate POS / Fiscal Receipts
**Actor:** System; POS/fiscal device
**Description:** Ingest point-of-sale / fiscal receipts into the books.

### UC-ACC-08.10: Reconcile Balances
**Actor:** User, Accountant
**Description:** Verify recorded balances against bank/cash actuals.

---

## UC-ACC-09: Payments & Collections (web, mobile)

### UC-ACC-09.1: Record Manual Payment
**Actor:** User
**Description:** Mark a document fully or partially paid by hand.

### UC-ACC-09.2: Enable Online Payment (Pay-by-Link)
**Actor:** Owner/Admin; Payment Gateway
**Description:** Configure a gateway so customers can pay an invoice online.

### UC-ACC-09.3: Receive Online Payment & Auto-Mark Paid
**Actor:** System; Customer, Payment Gateway
**Description:** Capture a gateway payment and automatically settle the document.

### UC-ACC-09.4: Send Payment Confirmation / Thank-You
**Actor:** System
**Description:** Automatically email a confirmation when a document is paid.

### UC-ACC-09.5: Configure & Send Overdue Reminders
**Actor:** Owner/Admin, System
**Description:** Define dunning rules and send reminders for overdue documents.

### UC-ACC-09.6: Escalate Reminder Levels
**Actor:** System
**Description:** Send progressively firmer reminders as overdue age increases.

### UC-ACC-09.7: Track Receivables Aging
**Actor:** Owner/Admin, Accountant
**Description:** View outstanding receivables bucketed by overdue age.

### UC-ACC-09.8: Handle Partial Payments & Overpayments
**Actor:** User, System
**Description:** Record partial settlements and manage overpayment balances.

### UC-ACC-09.9: Issue Refund
**Actor:** User, Owner/Admin
**Description:** Record a refund against a paid document.

---

## UC-ACC-10: VAT & Tax Compliance (web)

### UC-ACC-10.1: Maintain VAT Records
**Actor:** System, Accountant
**Description:** Accumulate output/input VAT entries from issued and received documents.

### UC-ACC-10.2: Generate VAT Return
**Actor:** Owner/Admin, Accountant
**Description:** Produce the periodic VAT return from recorded entries.

### UC-ACC-10.3: Generate Control / Recapitulative Statement
**Actor:** Owner/Admin, Accountant
**Description:** Produce the detailed VAT control/recapitulative statement for the period.

### UC-ACC-10.4: Generate EC Sales List (Intra-EU)
**Actor:** Owner/Admin, Accountant
**Description:** Produce the summary of intra-community supplies.

### UC-ACC-10.5: Handle Reverse-Charge Transactions
**Actor:** User, Accountant
**Description:** Apply reverse-charge VAT treatment on qualifying documents.

### UC-ACC-10.6: Handle Cross-Border / OSS VAT
**Actor:** Accountant
**Description:** Apply destination-country VAT for cross-border B2C (one-stop-shop) sales.

### UC-ACC-10.7: Apply VAT Regime per Document
**Actor:** User, System
**Description:** Resolve the correct VAT regime (domestic, EU, export, exempt) per document.

### UC-ACC-10.8: Export Filings (XML / E-File)
**Actor:** Owner/Admin, Accountant; Tax Authority
**Description:** Export statutory filings in the authority's electronic format.

### UC-ACC-10.9: View VAT Summary per Period
**Actor:** Owner/Admin, Accountant
**Description:** Review VAT liability/credit summarized by period.

### UC-ACC-10.10: Operate in Non-VAT-Payer Mode
**Actor:** Owner/Admin
**Description:** Run documents and reports without VAT for non-registered businesses.

### UC-ACC-10.11: Round & Reconcile VAT
**Actor:** System, Accountant
**Description:** Apply VAT rounding rules and reconcile to document totals.

---

## UC-ACC-11: Inventory / Stock (web)

### UC-ACC-11.1: Create Stock Item / Warehouse
**Actor:** Owner/Admin, User
**Description:** Define a stock-tracked item and the warehouse(s) it lives in.

### UC-ACC-11.2: Receive Stock (Stock-In)
**Actor:** User
**Description:** Increase stock from a purchase or manual receipt.

### UC-ACC-11.3: Issue Stock (Stock-Out)
**Actor:** User, System
**Description:** Decrease stock via an invoice or delivery note.

### UC-ACC-11.4: View Stock Levels
**Actor:** User, Owner/Admin
**Description:** See current quantity on hand per item/warehouse.

### UC-ACC-11.5: View Stock Movement History
**Actor:** User, Accountant
**Description:** Audit all stock-in/out movements over time.

### UC-ACC-11.6: Stock Valuation
**Actor:** Accountant, Owner/Admin
**Description:** Value inventory on hand (e.g., average/FIFO cost).

### UC-ACC-11.7: Low-Stock Indicators
**Actor:** System, User
**Description:** Flag items below a configured reorder threshold.

### UC-ACC-11.8: Stock Adjustments / Corrections
**Actor:** User, Owner/Admin
**Description:** Correct stock levels for losses, counts, or errors.

---

## UC-ACC-12: Reporting, Dashboard & Cashflow (web, mobile)

### UC-ACC-12.1: View Dashboard / Overview
**Actor:** Owner/Admin
**Description:** See key indicators: revenue, receivables, overdue, recent activity.

### UC-ACC-12.2: Income & Expense (Cashflow) Report
**Actor:** Owner/Admin, Accountant
**Description:** Compare income vs expense over a period.

### UC-ACC-12.3: Sales Report
**Actor:** Owner/Admin, Accountant
**Description:** Break down sales by period, customer, or item.

### UC-ACC-12.4: Receivables / Payables Aging Report
**Actor:** Owner/Admin, Accountant
**Description:** List open receivables and payables by overdue age.

### UC-ACC-12.5: VAT Report
**Actor:** Owner/Admin, Accountant
**Description:** Summarize VAT by rate and period.

### UC-ACC-12.6: Export Lists (xlsx / csv)
**Actor:** Owner/Admin, Accountant, External System
**Description:** Export any document/contact/item list to spreadsheet formats.

### UC-ACC-12.7: Generate Accountant Export Package
**Actor:** Accountant, Owner/Admin
**Description:** Produce a consolidated export for import into accounting software.

### UC-ACC-12.8: Custom Date-Range Filtering
**Actor:** Owner/Admin, Accountant
**Description:** Filter reports/lists by arbitrary date ranges and dimensions.

### UC-ACC-12.9: Profit Overview
**Actor:** Owner/Admin
**Description:** View an approximate profit (income − expense) summary.

---

## UC-ACC-13: Automation & Recurring (web, system)

### UC-ACC-13.1: Configure Recurring Invoice
**Actor:** Owner/Admin, User
**Description:** Define a template, cadence, and recipients for a repeating invoice.

### UC-ACC-13.2: Auto-Generate & Send Recurring Invoices
**Actor:** System
**Description:** Create and dispatch recurring invoices on schedule, idempotently.

### UC-ACC-13.3: Schedule Automatic Reminders
**Actor:** System
**Description:** Send overdue reminders automatically per configured rules.

### UC-ACC-13.4: Auto-Match Incoming Payments
**Actor:** System
**Description:** Continuously pair imported bank transactions to documents.

### UC-ACC-13.5: Auto-Send Thank-You on Payment
**Actor:** System
**Description:** Email a confirmation automatically when payment is detected.

### UC-ACC-13.6: Rule-Based Defaults & Templates
**Actor:** Owner/Admin
**Description:** Apply automation rules for defaults (terms, numbering, texts) on new documents.

### UC-ACC-13.7: Auto-Download Exchange Rates
**Actor:** System
**Description:** Refresh currency exchange rates automatically for foreign-currency documents.

---

## UC-ACC-14: Integrations & API (web, system)

### UC-ACC-14.1: Authenticate & Call Public API
**Actor:** External System, Owner/Admin
**Description:** Use authenticated REST endpoints to read/write documents and data.

### UC-ACC-14.2: Manage API Keys / OAuth Clients
**Actor:** Owner/Admin
**Description:** Issue, scope, and revoke API credentials.

### UC-ACC-14.3: Subscribe to Webhooks / Events
**Actor:** External System, Owner/Admin
**Description:** Receive push notifications for events (document paid, created, etc.).

### UC-ACC-14.4: Connect E-shop / External System
**Actor:** Owner/Admin; E-shop
**Description:** Sync orders/invoices with an e-commerce or external platform.

### UC-ACC-14.5: Connect Bank
**Actor:** Owner/Admin; Bank
**Description:** Link a bank for automated statement/payment retrieval.

### UC-ACC-14.6: Connect Payment Gateway
**Actor:** Owner/Admin; Payment Gateway
**Description:** Link a gateway to accept online payments on invoices.

### UC-ACC-14.7: Export to Accounting Software
**Actor:** Accountant, External System
**Description:** Export documents in formats consumable by external accounting software.

### UC-ACC-14.8: Import Data
**Actor:** Owner/Admin, External System
**Description:** Import contacts, items, or documents from files/other systems.

### UC-ACC-14.9: Rate Limiting / Quotas
**Actor:** System
**Description:** Enforce per-plan API request quotas and rate limits.

---

## UC-ACC-15: Mobile & Notifications (mobile)

### UC-ACC-15.1: Use Mobile App
**Actor:** User, Owner/Admin
**Description:** Access core functionality from native iOS/Android apps.

### UC-ACC-15.2: Create / Send Invoice on Mobile
**Actor:** User
**Description:** Issue and send an invoice from a phone.

### UC-ACC-15.3: Scan Receipt / Document
**Actor:** User; System (OCR)
**Description:** Photograph a receipt and capture it into the inbox for processing.

### UC-ACC-15.4: Receive In-App Notifications
**Actor:** System, User
**Description:** Get alerts for paid, overdue, or new-document events.

### UC-ACC-15.5: Offline Draft & Sync
**Actor:** User, System
**Description:** Draft documents offline and sync when connectivity returns.

### UC-ACC-15.6: Mobile Dashboard
**Actor:** Owner/Admin
**Description:** View key indicators on mobile.

---

## UC-ACC-16: Platform — Security, Data & Compliance (web, system)

### UC-ACC-16.1: Two-Factor Authentication
**Actor:** Owner/Admin, User
**Description:** Require a second factor at login.

### UC-ACC-16.2: Role-Based Access Control
**Actor:** Owner/Admin, System
**Description:** Enforce per-role visibility and action permissions across modules.

### UC-ACC-16.3: Automatic Daily Backups
**Actor:** System
**Description:** Back up company data daily with restore capability.

### UC-ACC-16.4: Data Export / Migration
**Actor:** Owner/Admin
**Description:** Export complete data for portability or migration away.

### UC-ACC-16.5: Data Import from Other Systems
**Actor:** Owner/Admin
**Description:** Onboard by importing data from a previous system.

### UC-ACC-16.6: GDPR Data-Subject Requests
**Actor:** Owner/Admin, System
**Description:** Fulfill access/export and erasure requests for personal data.

### UC-ACC-16.7: Audit Trail / Change History
**Actor:** System, Owner/Admin
**Description:** Record an immutable history of document and setting changes.

### UC-ACC-16.8: Legislative / Template Updates
**Actor:** System
**Description:** Keep tax rules, statutory formats, and templates current.

### UC-ACC-16.9: Session Management & Login Security
**Actor:** System, Owner/Admin
**Description:** Manage active sessions, lockouts, and suspicious-login handling.

---

## Coverage Summary

- **16 categories**, **~140 use cases**, **11 actors**.
- **MVP (P1):** UC-ACC-01, -02, -03, -04, -05, -16 — register → configure → contacts + price list → issue/send/track invoices → secure, exportable data.
- **Differentiators to consider beyond parity:** AI/OCR document inbox (UC-ACC-07.4), automatic bank matching (UC-ACC-08.6), one-click statutory VAT filings (UC-ACC-10.2–10.4, 10.8), and recurring/dunning automation (UC-ACC-13).
