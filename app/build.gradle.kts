plugins {
    alias(libs.plugins.android.application)
}

android {
    namespace = "com.shininggrimace.shiningemulator"
    compileSdk = 37

    defaultConfig {
        applicationId = "com.shininggrimace.shiningemulator"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    buildFeatures {
        prefab = true
    }

    sourceSets {
        getByName("main") {
            assets.srcDirs("../assets")
        }
    }

    ndkVersion = "29.0.14206865"
}

tasks.preBuild {
    dependsOn(":rustBuild")
}

dependencies {
    implementation(libs.androidx.appcompat)
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.games.activity)
    implementation(libs.material)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(libs.androidx.junit)
}
