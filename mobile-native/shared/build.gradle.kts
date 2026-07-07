plugins {
    alias(libs.plugins.kotlin.multiplatform)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.android.kotlin.multiplatform.library)
}

kotlin {
    // AGP 9 migration: the new com.android.kotlin.multiplatform.library plugin replaces
    // `com.android.library` + `androidTarget`. It is variant-agnostic: no build types,
    // no product flavors, no BuildConfig generation.
    // See https://kotlinlang.org/docs/multiplatform/multiplatform-project-agp-9-migration.html
    androidLibrary {
        namespace = "three.two.bit.ppt.reality.shared"
        compileSdk = libs.versions.compileSdk.get().toInt()
        minSdk = libs.versions.minSdk.get().toInt()

        // Preserve the JVM target migrated in PR #378 (compilerOptions DSL).
        compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) }

        // Run `commonTest` on the JVM host. Without this the AGP-9 KMP library plugin creates
        // NO host test compilation at all, so `:shared:build` / `allTests` on Linux CI silently
        // executed zero tests and the whole commonTest suite was dormant (surfaced by #2125).
        withHostTest {}
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

        val commonTest by getting {
            dependencies {
                implementation(libs.kotlin.test)
                implementation(libs.ktor.client.mock)
                implementation(libs.ktor.client.content.negotiation)
                implementation(libs.ktor.serialization.kotlinx.json)
                implementation(libs.kotlinx.coroutines.test)
            }
        }

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
