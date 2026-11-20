# English translations for Property Management System

# Common errors
error-generic = An error occurred. Please try again.
error-not-found = The requested resource was not found.
error-unauthorized = You are not authorized to perform this action.
error-forbidden = Access denied.
error-bad-request = Invalid request.
error-conflict = A conflict occurred with the current state.
error-validation = Please correct the errors in your request.
error-rate-limited = Too many requests. Please try again later.
error-internal = An internal error occurred.
error-database = A database error occurred.
error-external-service = An external service error occurred.

# Auth errors
auth-invalid-credentials = Invalid email or password.
auth-email-required = Email is required.
auth-password-required = Password is required.
auth-invalid-email = Please enter a valid email address.
auth-weak-password = Password is too weak. Use at least 8 characters with letters and numbers.
auth-email-exists = An account with this email already exists.
auth-account-locked = Your account has been locked due to too many failed attempts.
auth-token-expired = Your session has expired. Please log in again.
auth-token-invalid = Invalid authentication token.
auth-session-expired = Your session has expired. Please log in again.

# Auth success
auth-registration-success = Account created successfully. Please check your email to verify your account.
auth-login-success = Successfully logged in.
auth-logout-success = Successfully logged out.
auth-password-reset-sent = Password reset instructions have been sent to your email.
auth-password-reset-success = Your password has been reset successfully.
auth-email-verified = Your email has been verified successfully.
auth-email-verification-sent = Check your email to verify your account.
auth-email-verification-success = Your email has been verified. You can now log in.
auth-password-reset-email-sent = If an account exists with this email, a password reset link has been sent.
auth-session-revoked = Session revoked successfully.

# Validation
validation-required = This field is required.
validation-invalid-format = Invalid format.
validation-too-short = Value is too short.
validation-too-long = Value is too long.
validation-out-of-range = Value is out of the allowed range.
validation-invalid-value = Invalid value.
validation-street-required = Street address is required.
validation-city-required = City is required.
validation-title-required = Title is required.
validation-question-text-required = Question text is required.
validation-comment-required = Comment is required.

# Resources
resource-not-found = Resource not found.
resource-already-exists = Resource already exists.
resource-access-denied = You do not have permission to access this resource.

# Faults
fault-created = Fault report has been created successfully.
fault-updated = Fault report has been updated.
fault-assigned = Fault has been assigned.
fault-resolved = Fault has been resolved.
fault-closed = Fault has been closed.
fault-created-success = Fault report has been created successfully.
fault-updated-success = Fault report has been updated.
fault-triaged-success = Fault has been triaged successfully.
fault-assigned-success = Fault has been assigned.
fault-status-updated = Status updated successfully.
fault-resolved-success = Fault has been resolved.
fault-confirmed-success = Resolution confirmed successfully.
fault-reopened-success = Fault has been reopened.

# Notifications
notification-sent = Notification sent successfully.
notification-failed = Failed to send notification.

# Documents
document-uploaded = Document uploaded successfully.
document-deleted = Document deleted successfully.
document-not-found = Document not found.
document-created-success = Document created successfully.
document-updated-success = Document updated successfully.
document-moved-success = Document moved successfully.
document-access-updated = Document access updated.
document-folder-created-success = Folder created successfully.
document-folder-updated-success = Folder updated successfully.
document-share-created-success = Share link created successfully.

# Voting
vote-submitted = Your vote has been submitted.
vote-already-cast = You have already voted in this poll.
voting-closed = This poll is closed for voting.
voting-end-date-must-be-future = End date must be in the future.
voting-start-before-end = Start date must be before end date.
voting-quorum-range-invalid = Quorum must be between 0 and 100.
voting-choices-required = At least two choices are required.
voting-hide-reason-required = A reason is required when hiding a vote.

# Organizations
organization-created = Organization created successfully.
organization-updated = Organization updated successfully.
organization-member-added = Member added to organization.
organization-member-removed = Member removed from organization.
organization-deleted-success = Organization deleted successfully.
organization-member-added-success = Member added to organization successfully.
organization-role-updated-success = Role updated successfully.
organization-member-removed-success = Member removed from organization successfully.
organization-role-deleted-success = Role deleted successfully.

# Announcements
announcement-created-success = Announcement created successfully.
announcement-updated-success = Announcement updated successfully.
announcement-published-success = Announcement published successfully.
announcement-scheduled-success = Announcement scheduled successfully.
announcement-archived-success = Announcement archived successfully.

# Forms
form-created-success = Form created successfully.
form-updated-success = Form updated successfully.
form-published-success = Form published successfully.
form-archived-success = Form archived successfully.
form-field-added-success = Field added successfully.
form-field-updated-success = Field updated successfully.
form-submitted-success = Form submitted successfully.

# Messaging
message-sent-success = Message sent successfully.
user-blocked-success = User blocked successfully.
user-unblocked-success = User unblocked successfully.

# Package & Visitor
package-registered-success = Package registered successfully.
package-updated-success = Package updated successfully.
package-marked-received = Package marked as received.
package-picked-up-success = Package picked up successfully.
visitor-updated-success = Visitor updated successfully.
visitor-checked-in-success = Visitor checked in successfully.
visitor-checked-out-success = Visitor checked out successfully.
visitor-registration-cancelled = Visitor registration cancelled.

# Saved-search alerts (reality-server drainer — BIT-139)
# $count = number of new matching listings (>= 1), $name = saved-search name.
search-alert-subject = { $count ->
    [one] { $count } new listing matches your saved search "{ $name }"
   *[other] { $count } new listings match your saved search "{ $name }"
}
search-alert-body = { $count ->
    [one] Your saved search "{ $name }" has { $count } new matching listing. Open the app to view it.
   *[other] Your saved search "{ $name }" has { $count } new matching listings. Open the app to view them.
}
