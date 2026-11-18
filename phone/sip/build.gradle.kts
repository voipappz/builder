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
            implementation("io.ktor:ktor-client-core:3.1.3")
            implementation("io.ktor:ktor-client-websockets:3.1.3")
        }
        jvmMain.dependencies {
            implementation("io.ktor:ktor-client-okhttp:3.1.3")
        }
        commonTest.dependencies {
            implementation(kotlin("test"))
            implementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.10.1")
        }
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
}
