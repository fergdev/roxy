package io.fergdev.roxy

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform