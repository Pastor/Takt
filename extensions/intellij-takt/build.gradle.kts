import org.jetbrains.intellij.platform.gradle.IntelliJPlatformType
import org.jetbrains.intellij.platform.gradle.TestFrameworkType

// Плагин IntelliJ IDEA для языка Takt — подсветка синтаксиса (фича 0022).
// Задача 0022-01 — каркас: FileType/Language, регистрация `.takt`.
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
        // config-file="takt-lsp4ij.xml">`): плагин ставится и работает без LSP4IJ
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
        // (org.takt.intellij): не влияет на совместимость, переименование id
        // сломало бы идентичность плагина.
        freeArgs = listOf("-mute", "TemplateWordInPluginId")
        ides {
            ide("IC-2024.3")
            ide("IC-2025.1")
            ide("IC-2025.2")
        }
    }
}

// ── Пусковой JDK: преflight-проверка (фича 0159) ──────────────────────────────
//
// ⚠️ ЗДЕСЬ ДВА РАЗНЫХ JDK, И ПУТАЮТ ИМЕННО ИХ.
//
//  * JDK КОМПИЛЯЦИИ задаёт `jvmToolchain(21)` ниже. Gradle скачивает его сам
//    (foojay-резолвер в settings.gradle.kts) — об этом заботиться не нужно.
//  * JDK ПУСКОВОЙ — тот, на котором работает демон Gradle и Kotlin-плагин.
//    Его берут из окружения, и ломается именно он: `jvmToolchain` его НЕ меняет.
//
// Таблица версий Java внутри `kotlin-gradle-plugin` 2.0.21 обрывается на
// `JAVA_21` (замер: класс commons-lang3 `JavaVersion` в jar плагина). Пусковой
// JDK новее приводит к отказу вида `IllegalArgumentException: 25.0.2` на
// `compileKotlin` — сообщение не называет ни причины, ни лечения, и час на его
// разгадывание уже был потрачен однажды.
//
// ГРАНИЦЫ НИЖЕ — ЗАМЕР, А НЕ ДОГАДКА:
//   ≤ 21  — работает (17 проверен прогоном; 21 им же компилирует toolchain);
//   25    — ломается (замер при работе над плагином);
//   22–24 — НЕ ПРОВЕРЕНО: таблица Kotlin даёт основание ожидать отказа, но JDK
//           этих версий под рукой не было. Поэтому предупреждение, а не отказ:
//           запрещать неизмеренное — та же догадка, только с другим знаком.
//
// ⚠️ Числа привязаны к Kotlin 2.0.21. Подняли Kotlin — ПЕРЕЗАМЕРЬТЕ границы, а
// не подвиньте их «на глаз».
run {
    val launcher = JavaVersion.current()
    val lastKnownGood = JavaVersion.VERSION_21
    val firstMeasuredBad = JavaVersion.VERSION_25
    // Номер версии подставляется и в прозу, и в команду из ОДНОГО значения:
    // зашитое числом «21» в примере разъехалось бы с границей при её правке
    // (мутация порога это и показала).
    val howToFix =
        "Запустите Gradle под JDK $lastKnownGood или ниже, например:\n" +
            "    JAVA_HOME=\$(/usr/libexec/java_home -v $lastKnownGood) ./gradlew <задача>\n" +
            "  `jvmToolchain` трогать не нужно: JDK компиляции Gradle скачивает сам."

    if (launcher >= firstMeasuredBad) {
        error(
            "Пусковой JDK $launcher не поддерживается сборкой плагина.\n" +
                "  Причина: таблица версий Java в kotlin-gradle-plugin 2.0.21 обрывается на " +
                "$lastKnownGood, и на более новом пусковом JDK `compileKotlin` падает с " +
                "IllegalArgumentException.\n  $howToFix",
        )
    }
    if (launcher > lastKnownGood) {
        logger.warn(
            "ВНИМАНИЕ: пусковой JDK $launcher не проверялся с этой сборкой.\n" +
                "  Известно: до $lastKnownGood включительно работает, $firstMeasuredBad ломается; " +
                "диапазон между ними не измерен.\n" +
                "  Если сборка упадёт с IllegalArgumentException на compileKotlin — причина эта.\n" +
                "  $howToFix\n" +
                "  Сработало или нет — стоит записать: тогда границу можно будет сузить фактом.",
        )
    }
}

kotlin {
    jvmToolchain(21)
}

tasks.test {
    useJUnit()
}
