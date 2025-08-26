@file:Suppress(
    "MemberVisibilityCanBePrivate",
    "MissingPackageDeclaration",
    "UndocumentedPublicClass",
    "UndocumentedPublicProperty",
    "MaxLineLength"
)
import org.gradle.api.JavaVersion
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

public object Config {
    const val group = "io.fergdev"
    const val artifact = "roxy"
    const val artifactId = "$group.$artifact"

    const val name = "Roxy"

    const val majorRelease = 1
    const val minorRelease = 0
    const val patch = 0
    const val postfix = "-alpha01"
    const val versionCode = 1

    const val majorVersionName = "$majorRelease.$minorRelease.$patch"
    const val versionName = "$majorVersionName$postfix"

    const val description =
        """Roxy - rust proxy, but with KMP front end"""

    const val url = "https://github.com/fergdev/roxy"


    const val licenseName = "The Apache License, Version 2.0"
    const val licenseUrl = "https://www.apache.org/licenses/LICENSE-2.0.txt"
    const val licenseDistribution = "https://www.apache.org/licenses/LICENSE-2.0.txt"

    val jvmTarget = JvmTarget.JVM_23
    val javaVersion = JavaVersion.VERSION_23

    object Android {
        const val compileSdk = 36
        const val targetSdk = compileSdk
        const val minSdk = 21
        const val isMinifyEnabledRelease = false
        const val isMinifyEnabledDebug = false
    }

    val optIns = listOf(
        "kotlin.RequiresOptIn",
        "kotlin.experimental.ExperimentalTypeInference",
        "kotlin.uuid.ExperimentalUuidApi",
        "kotlin.time.ExperimentalTime",
        "kotlin.contracts.ExperimentalContracts",
    )
    val compilerArgs = listOf(
        "-Xexpect-actual-classes",
        "-Xconsistent-data-class-copy-visibility",
        "-Xwarning-level=NOTHING_TO_INLINE:disabled",
        "-Xwarning-level=UNUSED_ANONYMOUS_PARAMETER:disabled",
    )
    val jvmCompilerArgs = buildList {
        add("-Xjvm-default=all") // enable all jvm optimizations
        add("-Xcontext-parameters")
        add("-Xstring-concat=inline")
        add("-Xlambdas=indy")
        add("-Xjdk-release=${jvmTarget.target}")
    }

    val wasmCompilerArgs = buildList {
        add("-Xwasm-use-new-exception-proposal")
        add("-Xwasm-debugger-custom-formatters")
    }
    object Detekt {
        const val configFile = "detekt.yml"
        val includedFiles = listOf("**/*.kt", "**/*.kts")
        val excludedFiles = listOf("**/resources/**", "**/build/**", "**/.idea/**", "**/**Test/**")
    }
}
