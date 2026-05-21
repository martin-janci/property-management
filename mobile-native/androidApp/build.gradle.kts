plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

// Read version from VERSION file (single source of truth)
val versionFile = rootProject.file("../VERSION")
val appVersion =
    if (versionFile.exists()) {
        versionFile.readText().trim()
    } else {
        "0.1.0" // Fallback version
    }

// Calculate versionCode: MAJOR * 10000 + MINOR * 100 + PATCH
val versionParts = appVersion.split(".")
val calculatedVersionCode =
    versionParts[0].toInt() * 10000 + versionParts[1].toInt() * 100 + versionParts[2].toInt()

android {
    namespace = "three.two.bit.ppt.reality"
    compileSdk = libs.versions.compileSdk.get().toInt()

    defaultConfig {
        applicationId = "three.two.bit.ppt.reality"
        minSdk = libs.versions.minSdk.get().toInt()
        targetSdk = libs.versions.targetSdk.get().toInt()
        versionCode = calculatedVersionCode
        versionName = appVersion
    }

    // Epic 85 - Story 85.2: Build Configuration by Environment
    // Signing configuration for release builds
    signingConfigs {
        create("release") {
            storeFile = file(System.getenv("KEYSTORE_FILE") ?: "../keystore/release.jks")
            storePassword = System.getenv("KEYSTORE_PASSWORD") ?: ""
            keyAlias = System.getenv("KEY_ALIAS") ?: "release"
            keyPassword = System.getenv("KEY_PASSWORD") ?: ""
        }
    }

    // Epic 85 - Story 85.2: Build Configuration by Environment
    // Product flavors for different environments
    flavorDimensions += "environment"
    productFlavors {
        create("development") {
            dimension = "environment"
            applicationIdSuffix = ".dev"
            versionNameSuffix = "-dev"
            resValue("string", "app_name", "Reality (Dev)")
            // Android emulator uses 10.0.2.2 to reach host localhost
            buildConfigField("String", "API_BASE_URL", "\"http://10.0.2.2:8081\"")
            buildConfigField("String", "ENVIRONMENT", "\"development\"")
            buildConfigField("Boolean", "ENABLE_LOGGING", "true")
        }
        create("staging") {
            dimension = "environment"
            applicationIdSuffix = ".staging"
            versionNameSuffix = "-staging"
            resValue("string", "app_name", "Reality (Staging)")
            buildConfigField(
                "String",
                "API_BASE_URL",
                "\"https://staging-reality.ppt.example.com\""
            )
            buildConfigField("String", "ENVIRONMENT", "\"staging\"")
            buildConfigField("Boolean", "ENABLE_LOGGING", "true")
        }
        create("production") {
            dimension = "environment"
            resValue("string", "app_name", "Reality Portal")
            buildConfigField("String", "API_BASE_URL", "\"https://reality.ppt.example.com\"")
            buildConfigField("String", "ENVIRONMENT", "\"production\"")
            buildConfigField("Boolean", "ENABLE_LOGGING", "false")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            // Use release signing config if keystore exists
            val keystoreFile = file(System.getenv("KEYSTORE_FILE") ?: "../keystore/release.jks")
            if (keystoreFile.exists()) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
        debug {
            isDebuggable = true
            applicationIdSuffix = ".debug"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }
}

kotlin { compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) } }

dependencies {
    implementation(project(":shared"))

    // Compose BOM
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.ui)
    implementation(libs.compose.material3)
    implementation(libs.compose.material.icons.extended)
    implementation(libs.compose.ui.tooling.preview)
    debugImplementation(libs.compose.ui.tooling)

    // AndroidX
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.navigation.compose)

    // Coroutines
    implementation(libs.kotlinx.coroutines.android)

    // Ktor (needed for repository default parameters)
    implementation(libs.ktor.client.core)
    implementation(libs.ktor.client.android)

    // Image loading
    implementation(libs.coil.compose)

    // Location services
    implementation(libs.play.services.location)

    // DataStore for preferences
    implementation(libs.datastore.preferences)
}
