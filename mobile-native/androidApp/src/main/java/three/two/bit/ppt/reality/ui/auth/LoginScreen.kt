package three.two.bit.ppt.reality.ui.auth

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Login
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.util.isNetworkError

/**
 * Email/password login screen — KMP / Compose M3 redesign matching the design bundle
 * (`guest-registration-v2-design-system/project/ui_kits/ mobile-native/screens-extension.jsx` →
 * `KmpLoginScreen`).
 *
 * Layout:
 * 1. Gradient brand hero (logo + "Reality Portal" + subtitle).
 * 2. Email + password fields with leading icons, password reveal icon preserved via the M3
 *    OutlinedTextField visual transformation.
 * 3. Right-aligned "Forgot password?" link.
 * 4. Primary "Sign in" button (filled).
 * 5. "OR" divider + footer "Don't have an account? Sign up".
 *
 * The primary "Sign in via PM App" action mirrors iOS `LoginView.loginWithSso()`: it hands the flow
 * to the Property Management app over the `propertymanagement://sso` deep link after minting the
 * per-flow CSRF nonce (see [onSsoLoginClick] / `SsoInitiation.begin`). Google / Apple social-login
 * buttons from the mock stay unwired — that flow isn't on the roadmap yet.
 *
 * UC-47.2 — Email/password login.
 *
 * @param onSsoLoginClick launches the SSO hop to the PM app. The caller mints the CSRF nonce via
 *   `SsoInitiation.begin()` and fires the outbound `ACTION_VIEW` intent; this screen only surfaces
 *   the affordance.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LoginScreen(
    onBackClick: () -> Unit,
    onSubmit: suspend (email: String, password: String) -> Result<Unit>,
    onForgotPasswordClick: () -> Unit,
    onRegisterClick: () -> Unit,
    onLoginSuccess: () -> Unit,
    onSsoLoginClick: () -> Unit = {},
) {
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var emailError by remember { mutableStateOf<String?>(null) }
    var passwordError by remember { mutableStateOf<String?>(null) }
    var generalError by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    val scope = rememberAuthScope()

    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Column(modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
            AuthBrandHero(
                title = stringResource(R.string.app_name),
                subtitle = stringResource(R.string.auth_login_subtitle),
            )
            Column(
                modifier =
                    Modifier.fillMaxWidth()
                        .padding(start = 20.dp, end = 20.dp, top = 32.dp, bottom = 24.dp)
            ) {
                generalError?.let {
                    ErrorBanner(it)
                    Spacer(modifier = Modifier.height(14.dp))
                }

                // Primary SSO affordance — parity with iOS `LoginView.ssoLoginSection`. Tapping this
                // mints a CSRF nonce and opens the Property Management app; the email/password form
                // below is the fallback.
                Button(
                    onClick = onSsoLoginClick,
                    modifier = Modifier.fillMaxWidth().height(52.dp),
                    shape = RoundedCornerShape(14.dp),
                ) {
                    Icon(
                        Icons.AutoMirrored.Filled.Login,
                        contentDescription = null,
                        modifier = Modifier.padding(end = 8.dp).size(18.dp),
                    )
                    Text(
                        text = stringResource(R.string.sign_in_pm_app),
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.SemiBold,
                    )
                }
                Spacer(modifier = Modifier.height(6.dp))
                Text(
                    text = stringResource(R.string.sign_in_redirect_notice),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OrDivider(stringResource(R.string.auth_or))

                AuthFieldLabel(stringResource(R.string.email))
                Spacer(modifier = Modifier.height(6.dp))
                OutlinedTextField(
                    value = email,
                    onValueChange = {
                        email = it
                        emailError = null
                    },
                    placeholder = { Text(stringResource(R.string.auth_login_email_placeholder)) },
                    leadingIcon = { Icon(Icons.Default.Email, contentDescription = null) },
                    singleLine = true,
                    isError = emailError != null,
                    supportingText = { emailError?.let { Text(it) } },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email),
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                )
                Spacer(modifier = Modifier.height(14.dp))

                AuthFieldLabel(stringResource(R.string.password))
                Spacer(modifier = Modifier.height(6.dp))
                OutlinedTextField(
                    value = password,
                    onValueChange = {
                        password = it
                        passwordError = null
                    },
                    placeholder = { Text("••••••••") },
                    leadingIcon = { Icon(Icons.Default.Lock, contentDescription = null) },
                    singleLine = true,
                    isError = passwordError != null,
                    supportingText = { passwordError?.let { Text(it) } },
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                )

                Spacer(modifier = Modifier.height(10.dp))
                Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                    TextButton(onClick = onForgotPasswordClick) {
                        Text(
                            text = stringResource(R.string.forgot_password),
                            style = MaterialTheme.typography.labelLarge,
                            fontWeight = FontWeight.Medium,
                        )
                    }
                }

                Spacer(modifier = Modifier.height(12.dp))
                val emailRequiredMsg = stringResource(R.string.auth_validation_email_required)
                val emailInvalidMsg = stringResource(R.string.auth_validation_email_invalid)
                val passwordRequiredMsg = stringResource(R.string.auth_validation_password_required)
                val signInFailedMsg = stringResource(R.string.auth_validation_sign_in_failed)
                val networkErrorMsg = stringResource(R.string.error_network)
                Button(
                    onClick = {
                        emailError =
                            when {
                                email.isBlank() -> emailRequiredMsg
                                !emailRegex.matches(email.trim()) -> emailInvalidMsg
                                else -> null
                            }
                        passwordError = if (password.isEmpty()) passwordRequiredMsg else null
                        if (emailError != null || passwordError != null) return@Button
                        isSubmitting = true
                        generalError = null
                        scope.launch {
                            val result = onSubmit(email.trim(), password)
                            isSubmitting = false
                            result.fold(
                                onSuccess = { onLoginSuccess() },
                                onFailure = {
                                    generalError =
                                        if (it.isNetworkError()) networkErrorMsg
                                        else signInFailedMsg
                                },
                            )
                        }
                    },
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth().height(52.dp),
                    shape = RoundedCornerShape(14.dp),
                ) {
                    if (isSubmitting) {
                        CircularProgressIndicator(
                            modifier = Modifier.padding(end = 8.dp).size(18.dp),
                            strokeWidth = 2.dp,
                            color = MaterialTheme.colorScheme.onPrimary,
                        )
                    }
                    Text(
                        text = stringResource(R.string.sign_in),
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.SemiBold,
                    )
                }

                Spacer(modifier = Modifier.height(24.dp))
                Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            text = stringResource(R.string.auth_login_no_account),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        TextButton(onClick = onRegisterClick) {
                            Text(
                                text = stringResource(R.string.create_account),
                                style = MaterialTheme.typography.bodyMedium,
                                fontWeight = FontWeight.SemiBold,
                            )
                        }
                    }
                }
            }
        }
    }
}
