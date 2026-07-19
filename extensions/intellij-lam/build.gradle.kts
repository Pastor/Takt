import org.jetbrains.intellij.platform.gradle.IntelliJPlatformType
import org.jetbrains.intellij.platform.gradle.TestFrameworkType

// Плагин IntelliJ IDEA для языка Lam — подсветка синтаксиса (фича 0022).
// Задача 0022-01 — каркас: FileType/Language, регистрация `.lam`.
// Лексер/highlighter (0022-02) и настройка цветов (0022-03) добавляются поверх.
plugins {
    id("java")
    kotlin("jvm") version "2.0.21"
    id("org.jetbrains.intellij.platform") version "2.1.0"
}

group = providers.gradleProperty("pluginGroup").get()
version = providers.gradleProperty("pluginVersion").get()

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

dependencies {
    intellijPlatform {
        create(
            IntelliJPlatformType.fromCode(providers.gradleProperty("platformType").get()),
            providers.gradleProperty("platformVersion").get(),
        )
        testFramework(TestFrameworkType.Platform)
        // CLI IntelliJ Plugin Verifier — для задачи verifyPlugin (проверка
        // бинарной совместимости с новыми IDE).
        pluginVerifier()
        // LSP4IJ (фича 0038): зависимость на клиент LSP от Red Hat. Версия
        // фиксируется явно (API LSP4IJ моложе платформенных, риск ADR).
        // Опциональность объявляется в plugin.xml (`<depends optional="true"
        // config-file="lam-lsp4ij.xml">`): плагин ставится и работает без LSP4IJ
        // (лексический слой 0022), семантический слой включается лишь при её наличии.
        plugin("com.redhat.devtools.lsp4ij:0.20.1")
    }
    testImplementation("junit:junit:4.13.2")
    // Тест-фреймворк платформы 2024.2 ожидает opentest4j на classpath
    // (`org.opentest4j.AssertionFailedError`) — при подъёме с 2024.1 (фича 0038)
    // его перестало подтягивать транзитивно; добавляем явно.
    testImplementation("org.opentest4j:opentest4j:1.3.0")
}

intellijPlatform {
    // Инструментация кода (формы/NLS) не нужна: у плагина нет Java-исходников и форм.
    instrumentCode = false

    pluginConfiguration {
        ideaVersion {
            sinceBuild = providers.gradleProperty("pluginSinceBuild")
            // Открытый верхний диапазон: при пустом `pluginUntilBuild` провайдер
            // становится «отсутствующим» (filter отбрасывает пустое значение), и
            // атрибут until-build в дескрипторе НЕ эмитится вовсе. Прежний
            // `.orElse("")` давал невалидный `until-build=""` (ловится Plugin
            // Verifier). Пустой until-build ⇒ плагин ставится в любые новые IDE.
            untilBuild = providers.gradleProperty("pluginUntilBuild").filter { it.isNotBlank() }
        }
    }

    // Проверка бинарной совместимости с новыми IDE (IntelliJ Plugin Verifier).
    // Открытый until-build обещает работу в свежих сборках — verifier это проверяет
    // на явном спреде релизов IC, которые новее сборочной платформы (2024.1.7).
    // `recommended()` не используем: он подтягивает ещё не вышедшую сборку (2025.3),
    // не резолвимую в релизном репозитории. Список правится по мере выхода IDE.
    pluginVerification {
        // Гасим косметический гайдлайн Marketplace «слово intellij в id плагина»
        // (org.lam.intellij): не влияет на совместимость, переименование id
        // сломало бы идентичность плагина.
        freeArgs = listOf("-mute", "TemplateWordInPluginId")
        ides {
            ide("IC-2024.3")
            ide("IC-2025.1")
            ide("IC-2025.2")
        }
    }
}

kotlin {
    jvmToolchain(21)
}

tasks.test {
    useJUnit()
}
