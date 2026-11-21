namespace PropertyManagement.Shared;

using TypeSpec.Http;

@doc("Standard error response")
model ErrorResponse {
  @doc("Error code for programmatic handling")
  code: string;

  @doc("Human-readable error message")
  message: string;

  @doc("Request ID for support/debugging")
  requestId?: string;

  @doc("Detailed validation errors")
  details?: ValidationError[];

  @doc("ISO 8601 timestamp")
  timestamp: dateTime;
}

@doc("Validation error detail")
model ValidationError {
  @doc("Field path (e.g., 'address.city')")
  field: string;

  @doc("Error message for this field")
  message: string;

  @doc("Error code")
  code: string;
}

// Common error responses
@error
@doc("Bad Request - Invalid input")
model BadRequestError {
  @statusCode statusCode: 400;
  @body body: ErrorResponse;
}

@error
@doc("Unauthorized - Authentication required")
model UnauthorizedError {
  @statusCode statusCode: 401;
  @body body: ErrorResponse;
}

@error
@doc("Forbidden - Insufficient permissions")
model ForbiddenError {
  @statusCode statusCode: 403;
  @body body: ErrorResponse;
}

@error
@doc("Not Found - Resource does not exist")
model NotFoundError {
  @statusCode statusCode: 404;
  @body body: ErrorResponse;
}

@error
@doc("Conflict - Resource already exists or version mismatch")
model ConflictError {
  @statusCode statusCode: 409;
  @body body: ErrorResponse;
}

@error
@doc("Unprocessable Entity - Business rule violation")
model UnprocessableEntityError {
  @statusCode statusCode: 422;
  @body body: ErrorResponse;
}

@error
@doc("Too Many Requests - Rate limit exceeded")
model RateLimitError {
  @statusCode statusCode: 429;
  @header("Retry-After") retryAfter: int32;
  @body body: ErrorResponse;
}

@error
@doc("Internal Server Error")
model InternalServerError {
  @statusCode statusCode: 500;
  @body body: ErrorResponse;
}

// Some legacy handlers return `(StatusCode, String)` (axum) rather than the
// JSON `ErrorResponse` envelope, so their 400 body is a plain-text,
// human-readable message with `Content-Type: text/plain`. This model documents
// that existing shape as-is; new handlers should prefer `BadRequestError`
// (JSON envelope). Converging the legacy plain-text handlers onto the envelope
// is a separate, breaking change tracked outside this contract fix.
@error
@doc("Bad Request with a plain-text body (legacy `(StatusCode, String)` handlers). The body is a human-readable message, e.g. `limit must not be negative: -1`.")
model PlainTextBadRequestError {
  @statusCode statusCode: 400;

  @doc("Human-readable error message, e.g. `offset must not be negative: -5`.")
  @body body: string;
}
