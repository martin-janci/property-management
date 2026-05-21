plugins {
    alias(libs.plugins.kotlin.multiplatform)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.android.library)
}

kotlin {
    androidTarget {
        compilerOptions {
            jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
        }
    }

    listOf(iosX64(), iosArm64(), iosSimulatorArm64()).forEach {
        it.binaries.framework {
            baseName = "shared"
            isStatic = true
        }
    }

    sourceSets {
        val commonMain by getting {
            dependencies {
                // Ktor
                implementation(libs.ktor.client.core)
                implementation(libs.ktor.client.content.negotiation)
                implementation(libs.ktor.serialization.kotlinx.json)
                implementation(libs.ktor.client.logging)

                // Kotlin
                implementation(libs.kotlinx.coroutines.core)
                implementation(libs.kotlinx.serialization.json)
                implementation(libs.kotlinx.datetime)
            }
        }

        val commonTest by getting { dependencies { implementation(libs.kotlin.test) } }

        val androidMain by getting { dependencies { implementation(libs.ktor.client.android) } }

        val iosX64Main by getting
        val iosArm64Main by getting
        val iosSimulatorArm64Main by getting
        val iosMain by creating {
            dependsOn(commonMain)
            iosX64Main.dependsOn(this)
            iosArm64Main.dependsOn(this)
            iosSimulatorArm64Main.dependsOn(this)
            dependencies { implementation(libs.ktor.client.darwin) }
        }
    }
}

android {
    namespace = "three.two.bit.ppt.reality.shared"
    compileSdk = libs.versions.compileSdk.get().toInt()

    defaultConfig { minSdk = libs.versions.minSdk.get().toInt() }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // Epic 85 - Story 85.2: Build Configuration by Environment
    // Enable BuildConfig generation for shared module
    buildFeatures { buildConfig = true }

    // Epic 85 - Story 85.2: Build Configuration by Environment
    // Product flavors matching the app module
    flavorDimensions += "environment"
    productFlavors {
        create("development") {
            dimension = "environment"
            // Android emulator uses 10.0.2.2 to reach host localhost
            buildConfigField("String", "API_BASE_URL", "\"http://10.0.2.2:8081\"")
            buildConfigField("String", "ENVIRONMENT", "\"development\"")
            buildConfigField("Boolean", "ENABLE_LOGGING", "true")
        }
        create("staging") {
            dimension = "environment"
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
            buildConfigField("String", "API_BASE_URL", "\"https://reality.ppt.example.com\"")
            buildConfigField("String", "ENVIRONMENT", "\"production\"")
            buildConfigField("Boolean", "ENABLE_LOGGING", "false")
        }
    }
}
