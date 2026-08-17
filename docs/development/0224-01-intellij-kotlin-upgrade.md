# Задача 0224-01: Подъём Kotlin в плагине intellij-takt снимет ограничение на пусковой JDK

> Фича: [../features/0224-intellij-kotlin-upgrade.md](../features/0224-intellij-kotlin-upgrade.md) · ADR: [../adr/0224-intellij-kotlin-upgrade.md](../adr/0224-intellij-kotlin-upgrade.md) · анализ: [../analyze/0224-intellij-kotlin-upgrade.md](../analyze/0224-intellij-kotlin-upgrade.md)

## Что было

Сборка плагина падала под пусковым JDK новее 21 сообщением из одного номера
версии (`* What went wrong: 26.0.2`). Преflight-проверка фичи 0159 объясняла
отказ таблицей версий в `kotlin-gradle-plugin` 2.0.21 и требовала запускать
Gradle под старым JDK. ADR 0224 показал пробой, что причина названа неверно:
падает **встроенный в Gradle** компилятор Kotlin-скриптов, а подъём одного
Kotlin JDK 26 не покрыл бы вовсе (таблица даже у 2.4.10 обрывается на 24).

## Что сделано

**1. Сборочная связка поднята** (`extensions/intellij-takt`):

| Компонент | Было | Стало |
|---|---|---|
| Gradle (wrapper) | 8.10.2 | **9.7.0** |
| `org.jetbrains.intellij.platform` | 2.1.0 | **2.18.1** |
| Kotlin (`kotlin("jvm")`) | 2.0.21 | **2.4.10** |

`platformVersion` (2024.2.5), `platformType` (IC), `pluginSinceBuild` (242) и
открытый `pluginUntilBuild` **не тронуты** — круг совместимых IDE прежний.
Wrapper обновлён задачей `./gradlew wrapper --gradle-version 9.7.0`, поэтому
`gradlew`/`gradlew.bat` переписаны самим Gradle (у `.bat` вернулись CRLF).

**2. Форма списка IDE верификатора заменена вынужденно.** В 2.18.1 метода
`ide("IC-2024.3")` нет; `create(type, version)` заводит одну конфигурацию на все
версии, и резолв Gradle сводит их к старшей (`idea:ideaIC:2024.3 -> 2025.2`).
Работает форма с одним провайдером:

```kotlin
ides {
    create(providers.provider { listOf("IC-2024.3", "IC-2025.1", "IC-2025.2") })
}
```

Прогон `verifyPlugin` подтверждает: **три** верификации, все `Compatible`.

**3. Преflight-проверка перезамерена.** Жёсткий `error` снят — измеренной
«плохой» версии больше нет (17 и 26 проверены прогоном); осталось
предупреждение о непроверенных 27+, называющее лечение (поднять Gradle).
Комментарий переписан: он больше не винит Kotlin, а приводит стектрейс с
`org.gradle.kotlin.dsl.support.KotlinCompiler`.

**4. Скрипт установки перестал искать именно JDK 17**
(`extensions/install-rustrover-plugin.sh`). Теперь он следит за **диапазоном**
`JDK_MIN=17`…`JDK_MAX=26`: текущий JDK не трогается, если годится; иначе ищется
подходящий; не нашли — предупреждение, а не отказ. Границы вынесены в
переменные, чтобы проза сообщений и проверка брались из одного значения.
PowerShell-версия (`.ps1`) JDK не выбирает вовсе — правок не требует.

**5. Документация приведена в соответствие:** README плагина (требования,
раздел `verifyPlugin`, описание скрипта) и живой контекст `CLAUDE.md`.

**Попутно** (найдено при правке, не предмет фичи): в `README.md` и `CLAUDE.md`
каталог плагина назывался `extensions/intellij-lam` — имя, отменённое
переименованием Lam → Takt (фича 0100). Три вхождения исправлены.

Обратная функциональность (правило 11): исходники плагина, язык, компилятор и
симулятор не менялись; версия языка не поднимается.

## Проверки

```sh
cd extensions/intellij-takt
JAVA_HOME=/opt/homebrew/opt/openjdk/libexec/openjdk.jdk/Contents/Home ./gradlew --warning-mode all test
JAVA_HOME=$(/usr/libexec/java_home -v 17) ./gradlew --rerun-tasks test
./gradlew buildPlugin
./gradlew verifyPlugin
sh -n ../install-rustrover-plugin.sh
cd ../.. && ./scripts/precheck.sh
```

Результаты — в отчёте [../reports/0224-intellij-kotlin-upgrade.md](../reports/0224-intellij-kotlin-upgrade.md).
