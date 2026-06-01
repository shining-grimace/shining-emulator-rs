
// Required for building dependencies that need CC, etc. to be set; else all of that can go
val NDK_TOOLCHAIN_PATH = "/home/thomas/Android/Sdk/ndk/29.0.14206865/toolchains/llvm/prebuilt/linux-x86_64"
val TARGET_SDK = 35

tasks.register<Exec>("cargoBuildAarch64") {
    outputs.upToDateWhen { false }

    // Environment needed by Bevy app
    environment("AR", "$NDK_TOOLCHAIN_PATH/bin/llvm-ar")
    environment("CC", "$NDK_TOOLCHAIN_PATH/bin/aarch64-linux-android${TARGET_SDK}-clang")
    environment("CXX", "$NDK_TOOLCHAIN_PATH/bin/aarch64-linux-android${TARGET_SDK}-clang++")
    environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", "$NDK_TOOLCHAIN_PATH/bin/aarch64-linux-android${TARGET_SDK}-clang")

    commandLine(
        "cargo",
        "build",
        "--package",
        "android",
        "--target",
        "aarch64-linux-android",
        "--release"
    )
}

tasks.register<Exec>("cargoBuildX86_64") {
    outputs.upToDateWhen { false }

    // Environment needed by Bevy app
    environment("AR", "$NDK_TOOLCHAIN_PATH/bin/llvm-ar")
    environment("CC", "$NDK_TOOLCHAIN_PATH/bin/x86_64-linux-android${TARGET_SDK}-clang")
    environment("CXX", "$NDK_TOOLCHAIN_PATH/bin/x86_64-linux-android${TARGET_SDK}-clang++")
    environment("CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER", "$NDK_TOOLCHAIN_PATH/bin/x86_64-linux-android${TARGET_SDK}-clang")

    commandLine(
        "cargo",
        "build",
        "--package",
        "android",
        "--target",
        "x86_64-linux-android",
        "--release",
    )
}

tasks.register("copyAarch64Lib") {
    dependsOn("cargoBuildAarch64")
    doLast {
        project.copy {
            from("target/aarch64-linux-android/release/libshiningemulator.so")
            into("app/src/main/jniLibs/arm64-v8a")
        }
    }
    outputs.upToDateWhen { false }
}

tasks.register("copyX86_64Lib") {
    dependsOn("cargoBuildX86_64")
    doLast {
        project.copy {
            from("target/x86_64-linux-android/release/libshiningemulator.so")
            into("app/src/main/jniLibs/x86_64")
        }
    }
    outputs.upToDateWhen { false }
}

tasks.register("rustBuild") {
    dependsOn("copyAarch64Lib", "copyX86_64Lib")
    doLast {
        file("app/src/main/jniLibs").setLastModified(System.currentTimeMillis())
    }
}
