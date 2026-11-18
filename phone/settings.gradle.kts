rootProject.name = "voipappz-phone"

pluginManagement {
    repositories {
        gradlePluginPortal()
        google()
        mavenCentral()
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}

// The layout of JetBrains' Kotlin Multiplatform template: shared code and
// screens in `shared`, one small entry-point module per platform.
include(":sip")        // SIP over WebSocket, common Kotlin (a library)
include(":shared")     // the app: Compose Multiplatform screens and logic
include(":androidApp") // the Android entry point
// iosApp/ (the Xcode project) is added on a Mac.
