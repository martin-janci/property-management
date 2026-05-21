package three.two.bit.ppt.reality.api

/**
 * Android implementation of PlatformConfig.
 *
 * AGP 9 migration: the new `com.android.kotlin.multiplatform.library` plugin used by the `:shared`
 * module is variant-agnostic and no longer generates a `BuildConfig` class or supports product
 * flavors. The environment-specific values still live in the `:androidApp` module's product
 * flavors (`buildConfigField` in `androidApp/build.gradle.kts`), which DO generate a `BuildConfig`.
 *
 * This implementation reads those fields from the app's generated `BuildConfig` via reflection at
 * runtime — mirroring the iOS implementation, which reads `Info.plist` from `NSBundle` at runtime.
 * No compile-time dependency on the app module is introduced.
 *
 * Epic 85 - Story 85.1: Environment Variable Setup
 */
actual object PlatformConfig {
    /** Fully-qualified name of the app module's generated BuildConfig class. */
    private const val APP_BUILD_CONFIG = "three.two.bit.ppt.reality.BuildConfig"

    private val buildConfigFields: Map<String, Any?> by lazy {
        runCatching {
                val clazz = Class.forName(APP_BUILD_CONFIG)
                clazz.declaredFields.associate { field ->
                    field.isAccessible = true
                    field.name to field.get(null)
                }
            }
            .getOrDefault(emptyMap())
    }

    private fun stringField(name: String, default: String): String =
        buildConfigFields[name] as? String ?: default

    private fun booleanField(name: String, default: Boolean): Boolean =
        buildConfigFields[name] as? Boolean ?: default

    /**
     * Base URL for the Reality Portal API.
     *
     * Set by buildConfigField in androidApp/build.gradle.kts for each product flavor: -
     * development: http://10.0.2.2:8081 - staging: https://staging-reality.ppt.example.com -
     * production: https://reality.ppt.example.com
     */
    actual val baseUrl: String
        get() = stringField("API_BASE_URL", "https://reality.ppt.example.com")

    /**
     * Current environment: development, staging, or production.
     *
     * Set by buildConfigField in androidApp/build.gradle.kts for each product flavor.
     */
    actual val environment: String
        get() = stringField("ENVIRONMENT", "production")

    /**
     * Whether debug mode is enabled.
     *
     * Reads the app BuildConfig.DEBUG flag; falls back to environment-based detection.
     */
    actual val isDebug: Boolean
        get() = booleanField("DEBUG", environment != "production")

    /**
     * Whether logging is enabled.
     *
     * Set by buildConfigField in androidApp/build.gradle.kts for each product flavor.
     */
    actual val enableLogging: Boolean
        get() = booleanField("ENABLE_LOGGING", isDebug)
}
