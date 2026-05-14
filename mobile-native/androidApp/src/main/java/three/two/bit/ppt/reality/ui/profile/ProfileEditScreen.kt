package three.two.bit.ppt.reality.ui.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.CameraAlt
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Phone
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.ui.auth.AuthFieldLabel
import three.two.bit.ppt.reality.ui.auth.ErrorBanner
import three.two.bit.ppt.reality.ui.auth.SuccessBanner
import three.two.bit.ppt.reality.ui.auth.rememberAuthScope
import three.two.bit.ppt.reality.ui.theme.Brand500
import three.two.bit.ppt.reality.ui.theme.Brand800
import three.two.bit.ppt.reality.util.isNetworkError

/**
 * Edit-profile screen — KMP / Compose M3 redesign matching the design (`KmpEditProfileScreen`).
 * Sticky top bar with back + "Upraviť profil" title; 96dp gradient avatar with camera-edit overlay;
 * field labels + leading-icon text fields (Name, Phone); 2-col radio-card grid of supported
 * languages; "About me" multi-line text area with char counter; sticky bottom bar with "Cancel"
 * ghost + "Save changes" primary button.
 *
 * Note: phone, language, and "About me" don't have backing endpoints yet (`PUT /api/v1/users/me`
 * only accepts name). The fields render for design fidelity and remember local state, but only
 * `displayName` is propagated to `onSubmit`. Email field stays disabled per the existing rule
 * (support-driven).
 *
 * UC-47.7 — Edit profile.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProfileEditScreen(
    ssoService: SsoService,
    onBackClick: () -> Unit,
    onSubmit: suspend (displayName: String) -> Result<Unit>,
) {
    val authState by ssoService.authState.collectAsState()
    val authenticated = authState as? AuthState.Authenticated

    var displayName by
        remember(authenticated?.user?.userId) {
            mutableStateOf(authenticated?.user?.name.orEmpty())
        }
    var phone by remember { mutableStateOf("") }
    var aboutMe by remember { mutableStateOf("") }
    var selectedLang by remember { mutableStateOf("SK") }
    var displayNameError by remember { mutableStateOf<String?>(null) }
    var generalError by remember { mutableStateOf<String?>(null) }
    var savedMessage by remember { mutableStateOf<String?>(null) }
    var isSaving by remember { mutableStateOf(false) }

    val networkErrorMsg = stringResource(R.string.error_network)
    val saveFailedMsg = stringResource(R.string.error_generic)
    val scope = rememberAuthScope()

    Scaffold(
        topBar = {
            CenterAlignedTopAppBar(
                title = {
                    Text(
                        text = stringResource(R.string.edit_profile),
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
        bottomBar = {
            authenticated?.let {
                EditProfileBottomBar(
                    onCancel = onBackClick,
                    onSave = {
                        if (displayName.isBlank()) {
                            displayNameError = "Display name is required"
                            return@EditProfileBottomBar
                        }
                        isSaving = true
                        generalError = null
                        savedMessage = null
                        scope.launch {
                            val result = onSubmit(displayName.trim())
                            isSaving = false
                            result.fold(
                                onSuccess = { savedMessage = "Profile updated." },
                                onFailure = {
                                    generalError =
                                        if (it.isNetworkError()) networkErrorMsg else saveFailedMsg
                                },
                            )
                        }
                    },
                    saving = isSaving,
                )
            }
        },
    ) { padding ->
        if (authenticated == null) {
            Column(
                modifier = Modifier.fillMaxSize().padding(padding).padding(32.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                Text(
                    text = stringResource(R.string.profile_edit_signin_required_title),
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = stringResource(R.string.profile_edit_signin_required_body),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            return@Scaffold
        }

        Column(
            modifier =
                Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            // Avatar editor
            Box(
                modifier = Modifier.padding(top = 24.dp, bottom = 8.dp),
                contentAlignment = Alignment.BottomEnd,
            ) {
                Box(
                    modifier =
                        Modifier.size(96.dp).clip(CircleShape).drawBehind {
                            drawRect(
                                brush =
                                    Brush.linearGradient(
                                        colors = listOf(Brand800, Brand500),
                                        start = Offset(0f, 0f),
                                        end = Offset(size.width, size.height),
                                    ),
                            )
                        },
                    contentAlignment = Alignment.Center,
                ) {
                    val initials =
                        remember(authenticated.user.name) {
                            authenticated.user.name
                                .split(" ")
                                .take(2)
                                .mapNotNull { it.firstOrNull()?.uppercase() }
                                .joinToString("")
                                .ifEmpty { authenticated.user.email.take(2).uppercase() }
                        }
                    Text(
                        text = initials,
                        style = MaterialTheme.typography.headlineSmall.copy(fontSize = 32.sp),
                        fontWeight = FontWeight.SemiBold,
                        color = Color.White,
                    )
                }
                Box(
                    modifier =
                        Modifier.size(32.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.primary),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        Icons.Default.CameraAlt,
                        contentDescription = null,
                        tint = Color.White,
                        modifier = Modifier.size(16.dp),
                    )
                }
            }
            Text(
                text = stringResource(R.string.profile_edit_change_photo),
                style = MaterialTheme.typography.bodySmall,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.primary,
            )

            Spacer(modifier = Modifier.height(16.dp))
            Column(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp),
            ) {
                generalError?.let {
                    ErrorBanner(it)
                    Spacer(modifier = Modifier.height(12.dp))
                }
                savedMessage?.let {
                    SuccessBanner(it)
                    Spacer(modifier = Modifier.height(12.dp))
                }

                AuthFieldLabel(stringResource(R.string.auth_register_name_label))
                Spacer(modifier = Modifier.height(6.dp))
                OutlinedTextField(
                    value = displayName,
                    onValueChange = {
                        displayName = it
                        displayNameError = null
                        savedMessage = null
                    },
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
                    value = authenticated.user.email,
                    onValueChange = {},
                    leadingIcon = { Icon(Icons.Default.Email, contentDescription = null) },
                    singleLine = true,
                    enabled = false,
                    supportingText = { Text(stringResource(R.string.profile_edit_email_locked)) },
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                )

                Spacer(modifier = Modifier.height(14.dp))
                AuthFieldLabel(stringResource(R.string.profile_edit_phone))
                Spacer(modifier = Modifier.height(6.dp))
                OutlinedTextField(
                    value = phone,
                    onValueChange = { phone = it },
                    leadingIcon = { Icon(Icons.Default.Phone, contentDescription = null) },
                    singleLine = true,
                    placeholder = { Text("+421 ··· ··· ···") },
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                )

                Spacer(modifier = Modifier.height(18.dp))
                AuthFieldLabel(stringResource(R.string.profile_edit_language))
                Spacer(modifier = Modifier.height(6.dp))
                LanguageGrid(
                    selected = selectedLang,
                    onSelect = { selectedLang = it },
                )

                Spacer(modifier = Modifier.height(18.dp))
                AuthFieldLabel(stringResource(R.string.profile_edit_about))
                Spacer(modifier = Modifier.height(6.dp))
                OutlinedTextField(
                    value = aboutMe,
                    onValueChange = { if (it.length <= 280) aboutMe = it },
                    placeholder = { Text(stringResource(R.string.profile_edit_about_placeholder)) },
                    minLines = 4,
                    maxLines = 6,
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                )
                Spacer(modifier = Modifier.height(4.dp))
                Row(modifier = Modifier.fillMaxWidth()) {
                    Spacer(modifier = Modifier.weight(1f))
                    Text(
                        text = "${aboutMe.length} / 280",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Spacer(modifier = Modifier.height(80.dp))
            }
        }
    }
}

@Composable
private fun LanguageGrid(selected: String, onSelect: (String) -> Unit) {
    val langs =
        listOf(
            "SK" to "Slovenčina",
            "CS" to "Čeština",
            "DE" to "Deutsch",
            "EN" to "English",
            "PL" to "Polski",
            "HU" to "Magyar",
        )
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        langs.chunked(2).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { (code, label) ->
                    LanguageCard(
                        code = code,
                        label = label,
                        selected = selected == code,
                        onClick = { onSelect(code) },
                        modifier = Modifier.weight(1f),
                    )
                }
                if (row.size == 1) Spacer(modifier = Modifier.weight(1f))
            }
        }
    }
}

@Composable
private fun LanguageCard(
    code: String,
    label: String,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        onClick = onClick,
        modifier = modifier,
        shape = RoundedCornerShape(8.dp),
        color =
            if (selected) MaterialTheme.colorScheme.primaryContainer
            else MaterialTheme.colorScheme.surface,
        border =
            androidx.compose.foundation.BorderStroke(
                width = 1.5.dp,
                color =
                    if (selected) MaterialTheme.colorScheme.primary
                    else MaterialTheme.colorScheme.outline,
            ),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Surface(
                shape = CircleShape,
                color = Color.Transparent,
                border =
                    androidx.compose.foundation.BorderStroke(
                        width = 2.dp,
                        color =
                            if (selected) MaterialTheme.colorScheme.primary
                            else MaterialTheme.colorScheme.outline,
                    ),
                modifier = Modifier.size(18.dp),
            ) {
                if (selected) {
                    Box(
                        modifier =
                            Modifier.padding(4.dp)
                                .fillMaxSize()
                                .clip(CircleShape)
                                .background(MaterialTheme.colorScheme.primary),
                    )
                }
            }
            Spacer(modifier = Modifier.width(10.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = code,
                    style =
                        MaterialTheme.typography.labelSmall.copy(
                            fontWeight = FontWeight.Bold,
                            letterSpacing = 0.04.sp,
                        ),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    text = label,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.onSurface,
                )
            }
        }
    }
}

@Composable
private fun EditProfileBottomBar(
    onCancel: () -> Unit,
    onSave: () -> Unit,
    saving: Boolean,
) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surface,
        shadowElevation = 8.dp,
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            OutlinedButton(
                onClick = onCancel,
                modifier = Modifier.weight(1f).height(48.dp),
                shape = RoundedCornerShape(14.dp),
                enabled = !saving,
            ) {
                Text(
                    text = stringResource(R.string.cancel),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                )
            }
            Button(
                onClick = onSave,
                modifier = Modifier.weight(1f).height(48.dp),
                shape = RoundedCornerShape(14.dp),
                enabled = !saving,
            ) {
                if (saving) {
                    CircularProgressIndicator(
                        modifier = Modifier.padding(end = 8.dp).size(16.dp),
                        strokeWidth = 2.dp,
                        color = MaterialTheme.colorScheme.onPrimary,
                    )
                }
                Text(
                    text = stringResource(R.string.profile_edit_save),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                )
            }
        }
    }
}
