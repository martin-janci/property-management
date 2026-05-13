package three.two.bit.ppt.reality.ui.auth

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R

/**
 * New-password screen — KMP / Compose M3 redesign matching the design
 * (`KmpResetScreen`). Either the entry form (with inline strength meter)
 * or a "Password changed" success state.
 *
 * UC-47.4 — Password reset confirmation.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ResetPasswordScreen(
    token: String,
    onBackClick: () -> Unit,
    onSubmit: suspend (token: String, newPassword: String) -> Result<Unit>,
    onSuccess: () -> Unit,
) {
    var password by remember { mutableStateOf("") }
    var confirmPassword by remember { mutableStateOf("") }
    var passwordError by remember { mutableStateOf<String?>(null) }
    var confirmError by remember { mutableStateOf<String?>(null) }
    var generalError by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    var success by remember { mutableStateOf(false) }
    val scope = rememberAuthScope()

    Scaffold(
        topBar = {
            CenterAlignedTopAppBar(
                title = {
                    Text(
                        text = stringResource(R.string.auth_reset_title),
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.SemiBold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.Default.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                },
                colors = TopAppBarDefaults.centerAlignedTopAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                ),
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
        ) {
            if (token.isBlank()) {
                AuthIconHero(
                    iconBg = MaterialTheme.colorScheme.errorContainer,
                    iconContent = {
                        Icon(
                            Icons.Default.Lock,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onErrorContainer,
                            modifier = Modifier.size(32.dp),
                        )
                    },
                    title = stringResource(R.string.auth_reset_invalid_title),
                    subtitle = stringResource(R.string.auth_reset_invalid_body),
                )
                return@Column
            }

            if (success) {
                AuthIconHero(
                    iconBg = Color(0xFFD1FAE5),
                    iconContent = {
                        Icon(
                            Icons.Default.Check,
                            contentDescription = null,
                            tint = Color(0xFF065F46),
                            modifier = Modifier.size(36.dp),
                        )
                    },
                    title = stringResource(R.string.auth_reset_success_title),
                    subtitle = stringResource(R.string.auth_reset_success_body),
                )
                Spacer(modifier = Modifier.height(28.dp))
                Box(modifier = Modifier.padding(horizontal = 24.dp)) {
                    Button(
                        onClick = onSuccess,
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(52.dp),
                        shape = RoundedCornerShape(14.dp),
                    ) { Text(stringResource(R.string.sign_in)) }
                }
                return@Column
            }

            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 20.dp, vertical = 24.dp),
            ) {
                Text(
                    text = stringResource(R.string.auth_reset_body),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(modifier = Modifier.height(24.dp))
                generalError?.let {
                    ErrorBanner(it)
                    Spacer(modifier = Modifier.height(14.dp))
                }

                AuthFieldLabel(stringResource(R.string.auth_reset_new_password))
                Spacer(modifier = Modifier.height(6.dp))
                OutlinedTextField(
                    value = password,
                    onValueChange = { password = it; passwordError = null },
                    leadingIcon = { Icon(Icons.Default.Lock, contentDescription = null) },
                    singleLine = true,
                    isError = passwordError != null,
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                )
                Spacer(modifier = Modifier.height(8.dp))
                StrengthBars(password = password)
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = passwordError ?: strengthLabel(password),
                    style = MaterialTheme.typography.labelSmall,
                    color = if (passwordError != null) MaterialTheme.colorScheme.error
                    else MaterialTheme.colorScheme.onSurfaceVariant,
                )

                Spacer(modifier = Modifier.height(18.dp))
                AuthFieldLabel(stringResource(R.string.auth_register_confirm_label))
                Spacer(modifier = Modifier.height(6.dp))
                OutlinedTextField(
                    value = confirmPassword,
                    onValueChange = { confirmPassword = it; confirmError = null },
                    leadingIcon = { Icon(Icons.Default.Lock, contentDescription = null) },
                    singleLine = true,
                    isError = confirmError != null,
                    supportingText = { confirmError?.let { Text(it) } },
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                )

                Spacer(modifier = Modifier.height(24.dp))
                Button(
                    onClick = {
                        passwordError = when {
                            password.isEmpty() -> "Password is required"
                            password.length < MIN_PASSWORD_LENGTH ->
                                "Password must be at least $MIN_PASSWORD_LENGTH characters"
                            else -> null
                        }
                        confirmError =
                            if (confirmPassword != password) "Passwords do not match" else null
                        if (passwordError != null || confirmError != null) return@Button
                        isSubmitting = true
                        generalError = null
                        scope.launch {
                            val result = onSubmit(token, password)
                            isSubmitting = false
                            result.fold(
                                onSuccess = { success = true },
                                onFailure = {
                                    generalError = it.message ?: "Could not reset password."
                                },
                            )
                        }
                    },
                    enabled = !isSubmitting,
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(52.dp),
                    shape = RoundedCornerShape(14.dp),
                ) {
                    if (isSubmitting) {
                        CircularProgressIndicator(
                            modifier = Modifier
                                .padding(end = 8.dp)
                                .size(18.dp),
                            strokeWidth = 2.dp,
                            color = MaterialTheme.colorScheme.onPrimary,
                        )
                    }
                    Text(
                        text = stringResource(R.string.auth_reset_submit),
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.SemiBold,
                    )
                }
            }
        }
    }
}

@Composable
private fun StrengthBars(password: String) {
    val strength = passwordStrength(password)
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        repeat(4) { idx ->
            val fillColor = when {
                idx < strength && strength >= 3 -> Color(0xFF10B981)
                idx < strength && strength == 2 -> Color(0xFFF59E0B)
                idx < strength -> Color(0xFFEF4444)
                else -> MaterialTheme.colorScheme.outlineVariant
            }
            Box(
                modifier = Modifier
                    .weight(1f)
                    .height(4.dp)
                    .clip(RoundedCornerShape(2.dp))
                    .background(fillColor),
            )
        }
    }
}

private fun passwordStrength(password: String): Int = when {
    password.isEmpty() -> 0
    password.length < 6 -> 1
    password.length < 10 -> 2
    password.any { !it.isLetterOrDigit() } && password.any { it.isDigit() } -> 4
    else -> 3
}

@Composable
private fun strengthLabel(password: String): String {
    if (password.isEmpty()) return stringResource(R.string.auth_register_password_hint)
    return when (passwordStrength(password)) {
        1 -> stringResource(R.string.auth_register_strength_weak)
        2 -> stringResource(R.string.auth_register_strength_medium)
        3 -> stringResource(R.string.auth_register_strength_good)
        else -> stringResource(R.string.auth_register_strength_strong)
    }
}
