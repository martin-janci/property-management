import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Auth;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("User account")
model User {
  @doc("Unique identifier")
  id: uuid;

  @doc("Email address")
  email: email;

  @doc("Display name")
  displayName: string;

  @doc("Phone number")
  phone?: phoneNumber;

  @doc("Profile picture URL")
  avatarUrl?: url;

  @doc("Whether email is verified")
  emailVerified: boolean;

  @doc("Whether 2FA is enabled")
  twoFactorEnabled: boolean;

  @doc("Last login timestamp")
  lastLoginAt?: dateTime;

  @doc("Account status")
  status: UserStatus;

  ...AuditableEntity;
}

@doc("User account status")
enum UserStatus {
  Active: "active",
  Inactive: "inactive",
  Suspended: "suspended",
  PendingVerification: "pending_verification",
}

@doc("Login request")
model LoginRequest {
  @doc("Email address")
  email: email;

  @doc("Password")
  @secret
  password: string;

  @doc("2FA code (if enabled)")
  twoFactorCode?: string;
}

@doc("Login response")
model LoginResponse {
  @doc("Access token (JWT)")
  accessToken: string;

  @doc("Refresh token")
  refreshToken: string;

  @doc("Token expiration in seconds")
  expiresIn: int32;

  @doc("User info")
  user: User;

  @doc("Available tenants for this user")
  tenants: TenantMembership[];
}

@doc("User's membership in a tenant")
model TenantMembership {
  @doc("Tenant ID")
  tenantId: uuid;

  @doc("Tenant/Organization name")
  tenantName: string;

  @doc("User's role in this tenant")
  role: TenantRole;
}

@doc("Registration request")
model RegisterRequest {
  @doc("Email address")
  email: email;

  @doc("Password")
  @secret
  @minLength(8)
  password: string;

  @doc("Display name")
  displayName: string;

  @doc("Phone number")
  phone?: phoneNumber;

  @doc("Invitation code (if required)")
  invitationCode?: string;
}

@doc("Password reset request")
model PasswordResetRequest {
  @doc("Email address")
  email: email;
}

@doc("Password change request")
model PasswordChangeRequest {
  @doc("Current password")
  @secret
  currentPassword: string;

  @doc("New password")
  @secret
  @minLength(8)
  newPassword: string;
}

@doc("2FA setup response")
model TwoFactorSetupResponse {
  @doc("Secret key for authenticator app")
  secret: string;

  @doc("QR code data URL")
  qrCodeUrl: string;

  @doc("Backup codes")
  backupCodes: string[];
}

@route("/api/v1/auth")
@tag("Authentication")
interface AuthApi {
  @doc("Login with email and password")
  @post
  @route("/login")
  login(@body body: LoginRequest): LoginResponse | UnauthorizedError;

  @doc("Register new user account")
  @post
  @route("/register")
  register(@body body: RegisterRequest): {
    @statusCode statusCode: 201;
    @body body: User;
  } | BadRequestError | ConflictError;

  @doc("Refresh access token")
  @post
  @route("/refresh")
  refresh(@header("Authorization") refreshToken: string): LoginResponse | UnauthorizedError;

  @doc("Logout (invalidate tokens)")
  @post
  @route("/logout")
  logout(@header("Authorization") token?: string): {
    @statusCode statusCode: 204;
  };

  @doc("Request password reset")
  @post
  @route("/password-reset")
  requestPasswordReset(@body body: PasswordResetRequest): {
    @statusCode statusCode: 202;
  };

  @doc("Get current user profile")
  @get
  @route("/me")
  getMe(@header("Authorization") token?: string): User | UnauthorizedError;

  @doc("Update current user profile")
  @patch
  @route("/me")
  updateMe(
    @header("Authorization") token?: string,
    @body body: {
      displayName?: string;
      phone?: phoneNumber;
      avatarUrl?: url;
    }
  ): User | UnauthorizedError | BadRequestError;

  @doc("Change password")
  @post
  @route("/me/password")
  changePassword(
    @header("Authorization") token?: string,
    @body body: PasswordChangeRequest
  ): {
    @statusCode statusCode: 204;
  } | UnauthorizedError | BadRequestError;

  @doc("Setup 2FA")
  @post
  @route("/me/2fa/setup")
  setup2FA(@header("Authorization") token?: string): TwoFactorSetupResponse | UnauthorizedError;

  @doc("Enable 2FA")
  @post
  @route("/me/2fa/enable")
  enable2FA(
    @header("Authorization") token?: string,
    @body body: { code: string }
  ): {
    @statusCode statusCode: 204;
  } | UnauthorizedError | BadRequestError;

  @doc("Disable 2FA")
  @post
  @route("/me/2fa/disable")
  disable2FA(
    @header("Authorization") token?: string,
    @body body: { code: string }
  ): {
    @statusCode statusCode: 204;
  } | UnauthorizedError | BadRequestError;
}
