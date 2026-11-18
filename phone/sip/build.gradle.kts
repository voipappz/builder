import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidLibrary)
}

kotlin {
    jvmToolchain(17)

    // The phone: Android and iPhone share this code.
    androidTarget {
        compilerOptions { jvmTarget.set(JvmTarget.JVM_17) }
    }
    iosArm64()
    iosSimulatorArm64()
    iosX64()
    // Desktop JVM: fast tests and the live registerCheck, no phone needed.
    jvm()

    sourceSets {
        commonMain.dependencies {
            // Part of this library's public API (HttpClient, flows, Deferred), so `api`.
            api(libs.kotlinx.coroutines.core)
            api(libs.ktor.client.core)
            implementation(libs.ktor.client.websockets)
            implementation(libs.okio)
        }
        commonTest.dependencies {
            implementation(kotlin("test"))
            implementation(libs.kotlinx.coroutines.test)
        }
        androidMain.dependencies {
            implementation(libs.ktor.client.okhttp)
        }
        iosMain.dependencies {
            implementation(libs.ktor.client.darwin)
        }
        jvmMain.dependencies {
            implementation(libs.ktor.client.okhttp)
        }
    }
}

android {
    namespace = "io.voipappz.sip"
    compileSdk = libs.versions.android.compileSdk.get().toInt()
    defaultConfig {
        minSdk = libs.versions.android.minSdk.get().toInt()
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

// `./gradlew :sip:registerCheck` registers one account with a real switch and
// unregisters again. The account comes from the environment (never the repo):
// SIP_USER, SIP_PASSWORD, SIP_DOMAIN, and SIP_WSS (wss://host:port).
tasks.register<JavaExec>("registerCheck") {
    group = "verification"
    description = "REGISTER once against a real switch over WSS, then unregister"
    val jvm = kotlin.jvm()
    dependsOn(jvm.compilations["main"].compileTaskProvider)
    classpath = files(jvm.compilations["main"].output.allOutputs, jvm.compilations["main"].runtimeDependencyFiles)
    mainClass.set("io.voipappz.sip.RegisterCheckKt")
    notCompatibleWithConfigurationCache("reads the account from the environment at run time")
}
