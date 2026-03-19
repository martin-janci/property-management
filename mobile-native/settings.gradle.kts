// JDK version check - AGP 8.x requires JDK 17-21 (JDK 25+ is not supported)
val javaVersion = System.getProperty("java.version")
val javaMajor = javaVersion.split(".").first().toIntOrNull() ?: 0
if (javaMajor < 17 || javaMajor > 21) {
    throw GradleException(
        """
        |
        | JDK version mismatch!
        | Required: JDK 17-21 (recommended: 17)
        | Found: JDK $javaVersion
        |
        | Fix: Set JAVA_HOME to JDK 17 before running Gradle:
        |   export JAVA_HOME=/path/to/jdk-17
        |   ./gradlew assembleDebug
        |
        | macOS example:
        |   export JAVA_HOME=${'$'}(/usr/libexec/java_home -v 17)
        |
        """.trimMargin()
    )
}

pluginManagement {
    repositories {
        google()
        gradlePluginPortal()
        mavenCentral()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "RealityPortal"

include(":shared")

include(":androidApp")
