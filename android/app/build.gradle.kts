plugins {
    id("com.android.application")
}

android {
    namespace = "io.github.biraz_dev.zrpg"
    compileSdk = 35

    defaultConfig {
        applicationId = "io.github.biraz_dev.zrpg"
        // Android 10: covers current handhelds and guarantees Vulkan 1.1 or GLES 3.
        minSdk = 29
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        // The game library is built by cargo-ndk into src/main/jniLibs.
        ndk {
            abiFilters += listOf("arm64-v8a")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.core:core:1.13.1")
    // Must match the GameActivity version supported by the `android-activity`
    // crate that Bevy uses (0.6.x -> 4.4.0). Do not enable prefab.
    implementation("androidx.games:games-activity:4.4.0")
}
