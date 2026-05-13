package three.two.bit.ppt.reality.ui.auth

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Person
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
 * Account-creation screen — KMP / Compose M3 redesign matching the design
 * (`KmpRegisterScreen`). Sticky bar with back + "Vytvoriť účet" title,
 * fields with leading icons, inline password strength bars, terms
 * checkbox, primary "Create account" button.
 *
 * UC-47.1 — Registration.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RegisterScreen(
    onBackClick: () -> Unit,
    onSubmit: suspend (displayName: String, email: String, password: String) -> Result<Unit>,
    onSignInClick: () -> Unit,
) {
    var displayName by remember { mutableStateOf("") }
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var confirmPassword by remember { mutableStateOf("") }
    var displayNameError by remember { mutableStateOf<String?>(null) }
    var emailError by remember { mutableStateOf<String?>(null) }
    var passwordError by remember { mutableStateOf<String?>(null) }
    var confirmError by remember { mutableStateOf<String?>(null) }
    var generalError by remember { mutableStateOf<String?>(null) }
    var termsAccepted by remember { mutableStateOf(true) }
    var isSubmitting by remember { mutableStateOf(false) }
    var submitted by remember { mutableStateOf(false) }
    val scope = rememberAuthScope()

    Scaffold(
        topBar = {
            CenterAlignedTopAppBar(
                title = {
                    Text(
                        text = if (submitted) stringResource(R.string.auth_register_inbox_title)
                        else stringResource(R.string.create_account),
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.SemiBold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
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
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp, vertical = 24.dp),
        ) {
            if (submitted) {
                Text(
                    text = stringResource(R.string.auth_register_inbox_title),
                    style = MaterialTheme.typography.headlineSmall,
                    fontWeight = FontWeight.Bold,
                )
                Spacer(modifier = Modifier.height(10.dp))
                Text(
                    text = stringResource(R.string.auth_register_inbox_body, email),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(modifier = Modifier.height(24.dp))
                Button(
                    onClick = onSignInClick,
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(52.dp),
                    shape = RoundedCornerShape(14.dp),
                ) {
                    Text(stringResource(R.string.auth_back_to_sign_in))
                }
                return@Column
            }

            generalError?.let {
                ErrorBanner(it)
                Spacer(modifier = Modifier.height(14.dp))
            }

            AuthFieldLabel(stringResource(R.string.auth_register_name_label))
            Spacer(modifier = Modifier.height(6.dp))
            OutlinedTextField(
                value = displayName,
                onValueChange = { displayName = it; displayNameError = null },
                leadingIcon = { Icon(Icons.Default.Person, contentDescription = null) },
                singleLine = true,
                isError = displayNameError != null,
                supportingText = { displayNameError?.let { Text(it) } },
                shape = RoundedCornerShape(8.dp),
                modifier = Modifier.fillMaxWidth(),
            )

            Spacer(modifier = Modifier.height(14.dp))
            AuthFieldLabel(stringResource(R.string.email))
            Spacer(modifier = Modifier.height(6.dp))
            OutlinedTextField(
                value = email,
                onValueChange = { email = it; emailError = null },
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
            PasswordStrengthBars(strength = passwordStrength(password))
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = passwordError ?: passwordStrengthLabel(password),
                style = MaterialTheme.typography.labelSmall,
                color = if (passwordError != null) MaterialTheme.colorScheme.error
                else MaterialTheme.colorScheme.onSurfaceVariant,
            )

            Spacer(modifier = Modifier.height(14.dp))
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

            Spacer(modifier = Modifier.height(20.dp))
            Row(verticalAlignment = Alignment.Top) {
                Checkbox(
                    checked = termsAccepted,
                    onCheckedChange = { termsAccepted = it },
                )
                Spacer(modifier = Modifier.width(4.dp))
                Text(
                    text = stringResource(R.string.auth_register_terms),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurface,
                    modifier = Modifier.padding(top = 12.dp),
                )
            }

            Spacer(modifier = Modifier.height(20.dp))
            Button(
                onClick = {
                    displayNameError =
                        if (displayName.isBlank()) "Name is required" else null
                    emailError = when {
                        email.isBlank() -> "Email is required"
                        !emailRegex.matches(email.trim()) -> "Enter a valid email address"
                        else -> null
                    }
                    passwordError = when {
                        password.isEmpty() -> "Password is required"
                        password.length < MIN_PASSWORD_LENGTH ->
                            "Password must be at least $MIN_PASSWORD_LENGTH characters"
                        else -> null
                    }
                    confirmError =
                        if (confirmPassword != password) "Passwords do not match" else null
                    val errs = listOf(displayNameError, emailError, passwordError, confirmError)
                    if (errs.any { it != null }) return@Button
                    if (!termsAccepted) {
                        generalError = "Please accept the terms"
                        return@Button
                    }
                    isSubmitting = true
                    generalError = null
                    scope.launch {
                        val result = onSubmit(displayName.trim(), email.trim(), password)
                        isSubmitting = false
                        result.fold(
                            onSuccess = { submitted = true },
                            onFailure = { generalError = it.message ?: "Registration failed." },
                        )
                    }
                },
                enabled = !isSubmitting && termsAccepted,
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
                    text = stringResource(R.string.create_account),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                )
            }

            Spacer(modifier = Modifier.height(18.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = stringResource(R.string.auth_register_have_account),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                TextButton(onClick = onSignInClick) {
                    Text(
                        text = stringResource(R.string.sign_in),
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.SemiBold,
                    )
                }
            }
        }
    }
}

@Composable
private fun PasswordStrengthBars(strength: Int) {
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
    password.length < 6 -> 1
    password.length < 10 -> 2
    password.any { !it.isLetterOrDigit() } && password.any { it.isDigit() } -> 4
    else -> 3
}

@Composable
private fun passwordStrengthLabel(password: String): String {
    if (password.isEmpty()) return stringResource(R.string.auth_register_password_hint)
    return when (passwordStrength(password)) {
        1 -> stringResource(R.string.auth_register_strength_weak)
        2 -> stringResource(R.string.auth_register_strength_medium)
        3 -> stringResource(R.string.auth_register_strength_good)
        else -> stringResource(R.string.auth_register_strength_strong)
    }
}
