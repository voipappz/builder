plugins {
    kotlin("multiplatform")
}

kotlin {
    // JVM for now, so the SIP core is tested on a desktop; Android and iOS
    // targets are added with the app.
    jvm()

    sourceSets {
        commonMain.dependencies {
            implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.1")
            implementation("com.squareup.okio:okio:3.15.0")
        }
        commonTest.dependencies {
            implementation(kotlin("test"))
            implementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.10.1")
        }
    }
}
