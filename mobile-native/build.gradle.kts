plugins {
    // AGP 9: Kotlin support is built into com.android.application / .library — the
    // org.jetbrains.kotlin.android plugin is no longer declared. See
    // https://developer.android.com/build/migrate-to-built-in-kotlin
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.android.kotlin.multiplatform.library) apply false
    alias(libs.plugins.kotlin.multiplatform) apply false
    alias(libs.plugins.kotlin.serialization) apply false
    alias(libs.plugins.kotlin.compose) apply false
    // Spotless 8.5.0+ is required for Gradle 9 support.
    id("com.diffplug.spotless") version "8.9.0"
}

spotless {
    kotlin {
        target("**/*.kt")
        targetExclude("**/build/**")
        ktfmt().kotlinlangStyle()
    }
    kotlinGradle {
        target("**/*.gradle.kts")
        ktfmt().kotlinlangStyle()
    }
}
