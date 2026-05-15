package three.two.bit.ppt.reality.ui.auth

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Email
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.util.isNetworkError

/**
 * Forgot-password screen — KMP / Compose M3 redesign matching the design (`KmpForgotScreen`).
 * Sticky top bar with back + title; either the email-collection form or a "Check your inbox"
 * success state with a 72dp green icon hero.
 *
 * UC-47.3 — Password reset request.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ForgotPasswordScreen(
    onBackClick: () -> Unit,
    onSubmit: suspend (email: String) -> Result<Unit>,
    onSignInClick: () -> Unit,
) {
    var email by remember { mutableStateOf("") }
    var emailError by remember { mutableStateOf<String?>(null) }
    var generalError by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    var submitted by remember { mutableStateOf(false) }
    val scope = rememberAuthScope()

    Scaffold(
        topBar = {
            CenterAlignedTopAppBar(
                title = {
                    Text(
                        text = stringResource(R.string.auth_forgot_title),
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.SemiBold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.back)
                        )
                    }
                },
                colors =
                    TopAppBarDefaults.centerAlignedTopAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surface,
                    ),
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding),
        ) {
            if (submitted) {
                AuthIconHero(
                    iconBg = Color(0xFFD1FAE5),
                    iconContent = {
                        Icon(
                            Icons.Default.Email,
                            contentDescription = null,
                            tint = Color(0xFF065F46),
                            modifier = Modifier.size(32.dp),
                        )
                    },
                    title = stringResource(R.string.auth_forgot_success_title),
                    subtitle = stringResource(R.string.auth_forgot_success_body, email),
                )
                Spacer(modifier = Modifier.height(28.dp))
                Box(modifier = Modifier.padding(horizontal = 24.dp)) {
                    Button(
                        onClick = onSignInClick,
                        modifier = Modifier.fillMaxWidth().height(52.dp),
                        shape = RoundedCornerShape(14.dp),
                    ) {
                        Text(stringResource(R.string.auth_back_to_sign_in))
                    }
                }
                return@Column
            }

            Column(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 24.dp),
            ) {
                Text(
                    text = stringResource(R.string.auth_forgot_body),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(modifier = Modifier.height(24.dp))
                generalError?.let {
                    ErrorBanner(it)
                    Spacer(modifier = Modifier.height(14.dp))
                }
                AuthFieldLabel(stringResource(R.string.email))
                Spacer(modifier = Modifier.height(6.dp))
                OutlinedTextField(
                    value = email,
                    onValueChange = {
                        email = it
                        emailError = null
                    },
                    leadingIcon = { Icon(Icons.Default.Email, contentDescription = null) },
                    singleLine = true,
                    isError = emailError != null,
                    supportingText = { emailError?.let { Text(it) } },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email),
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                )
                Spacer(modifier = Modifier.height(24.dp))
                val emailRequiredMsg = stringResource(R.string.auth_validation_email_required)
                val emailInvalidMsg = stringResource(R.string.auth_validation_email_invalid)
                val sendFailedMsg = stringResource(R.string.auth_validation_send_reset_failed)
                val networkErrorMsg = stringResource(R.string.error_network)
                Button(
                    onClick = {
                        val trimmed = email.trim()
                        emailError =
                            when {
                                trimmed.isEmpty() -> emailRequiredMsg
                                !emailRegex.matches(trimmed) -> emailInvalidMsg
                                else -> null
                            }
                        if (emailError != null) return@Button
                        isSubmitting = true
                        generalError = null
                        scope.launch {
                            val result = onSubmit(trimmed)
                            isSubmitting = false
                            result.fold(
                                onSuccess = { submitted = true },
                                onFailure = {
                                    generalError =
                                        if (it.isNetworkError()) networkErrorMsg else sendFailedMsg
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
                        text = stringResource(R.string.auth_forgot_submit),
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.SemiBold,
                    )
                }
                Spacer(modifier = Modifier.height(18.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.Center,
                ) {
                    TextButton(onClick = onSignInClick) {
                        Text(
                            text = stringResource(R.string.auth_back_to_sign_in),
                            style = MaterialTheme.typography.bodyMedium,
                            fontWeight = FontWeight.Medium,
                        )
                    }
                }
            }
        }
    }
}
