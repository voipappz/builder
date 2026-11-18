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

// The SIP-over-WebSocket client, in common Kotlin (Android and iOS share it).
include(":sip")
