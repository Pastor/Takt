package org.takt.intellij.scheme

import com.google.gson.JsonObject
import com.google.gson.JsonParseException
import com.google.gson.JsonParser
import com.google.gson.JsonPrimitive
import java.io.File

/**
 * Разговор панели схемы с холстом: что уходит наружу и что приходит обратно.
 *
 * Чистая, тестируемая **без редактора** логика: панель строит сообщения и
 * разбирает ответы холста здесь, а редактор занимается окном и файлами. Форма
 * сообщений задана страницей (`web/static/scheme-host.js`) - второй её описи в
 * плагине нет.
 */
object TaktSchemeProtocol {

    /** Расширение файла раскладки: он лежит рядом с моделью. */
    const val LAYOUT_EXTENSION: String = ".takt-ui"

    /** Расширение файла модели. */
    const val MODEL_EXTENSION: String = ".takt"

    /**
     * Файл раскладки для файла модели; `null` - это не модель.
     *
     * Правило то же, что у страницы: имя модели плюс своё расширение. Разойдись
     * оно - редактор и страница писали бы раскладку в разные файлы.
     */
    fun layoutFileOf(model: File): File? {
        if (!model.name.endsWith(MODEL_EXTENSION)) return null
        return File(model.parentFile, model.name.removeSuffix(MODEL_EXTENSION) + LAYOUT_EXTENSION)
    }

    /** Сообщение холсту: граф модели. Тело графа - готовый JSON от сервера. */
    fun graphMessage(graphJson: String): String = """{"type":"graph","graph":$graphJson}"""

    /** Сообщение холсту: модель не разбирается, показать пустой лист. */
    fun noGraphMessage(): String = """{"type":"graph","graph":null}"""

    /** Сообщение холсту: содержимое файла раскладки. */
    fun layoutMessage(text: String): String = """{"type":"layout","text":${quote(text)}}"""

    /** Сообщение холсту: курсор в редакторе встал на строку и колонку. */
    fun cursorMessage(line: Int, character: Int): String =
        """{"type":"cursor","line":$line,"character":$character}"""

    /** Сообщение холсту: язык оболочки. */
    fun langMessage(lang: String): String = """{"type":"lang","lang":${quote(lang)}}"""

    /** Сообщение холсту: текущая тема IDE. */
    fun themeMessage(dark: Boolean): String =
        """{"type":"theme","theme":"${if (dark) "dark" else "light"}"}"""

    fun quote(text: String): String = JsonPrimitive(text).toString()

    fun graphObject(json: String): JsonObject? =
        parseObject(json)?.takeIf { it.get("sheets")?.isJsonArray == true }

    fun parseObject(json: String): JsonObject? = try {
        JsonParser.parseString(json).takeIf { it.isJsonObject }?.asJsonObject
    } catch (_: JsonParseException) {
        null
    }

    /** Разобранное сообщение холста. */
    sealed interface Incoming {
        /** Холст готов принимать данные. */
        data object Ready : Incoming

        /** Запрошен экспорт модели, привязанной к текущей сессии панели. */
        data object Export : Incoming

        /** Раскладка изменилась: записать текст в файл рядом с моделью. */
        data class Layout(val text: String) : Incoming

        /** Выбран узел: поставить курсор в редакторе. */
        data class Select(val line: Int, val character: Int) : Incoming
    }

    fun parse(message: JsonObject): Incoming? = when (string(message, "type")) {
        "ready" -> Incoming.Ready
        "export" -> Incoming.Export
        "layout" -> string(message, "text")?.let { Incoming.Layout(it) }
        "select" -> {
            val line = number(message, "line")
            val character = number(message, "character")
            if (line == null || character == null) null else Incoming.Select(line, character)
        }
        else -> null
    }

    fun string(message: JsonObject, name: String): String? =
        message.get(name)?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isString }?.asString

    private fun number(message: JsonObject, name: String): Int? =
        message.get(name)?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isNumber }
            ?.asString?.toIntOrNull()?.takeIf { it >= 0 }
}
