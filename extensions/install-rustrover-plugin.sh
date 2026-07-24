#!/bin/sh
# install-rustrover-plugin.sh — сборка плагина Takt для IntelliJ Platform и его
# установка/обновление в найденные инсталляции JetBrains RustRover.
#
# Что делает:
#   1. Собирает плагин `extensions/intellij-takt` (его же gradlew → buildPlugin),
#      получая build/distributions/intellij-takt-<версия>.zip.
#   2. Находит каталоги плагинов RustRover в стандартном месте JetBrains
#      (macOS/Linux; в т.ч. установки через Toolbox — конфиг там стандартный).
#   3. Ставит плагин, а при наличии прежней версии — обновляет (удаляет старую
#      папку плагина и распаковывает свежую).
#
# Использование:
#   extensions/install-rustrover-plugin.sh [--skip-build] [-h|--help]
#
#   --skip-build   не пересобирать — взять готовый zip из build/distributions.
#
# После установки RustRover нужно перезапустить (плагины подхватываются при
# старте). Если IDE запущена, изменения применятся после перезапуска.
set -eu

SKIP_BUILD=0
while [ $# -gt 0 ]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1; shift ;;
    -h|--help)
      sed -n '2,26p' "$0" | sed 's/^# \{0,1\}//'
      exit 0 ;;
    *) echo "Неизвестный аргумент: $1" >&2; exit 2 ;;
  esac
done

# Корень скрипта = extensions/; проект плагина рядом.
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PLUGIN_DIR="$SCRIPT_DIR/intellij-takt"

[ -d "$PLUGIN_DIR" ] || { echo "Не найден проект плагина: $PLUGIN_DIR" >&2; exit 1; }
[ -x "$PLUGIN_DIR/gradlew" ] || { echo "Не найден $PLUGIN_DIR/gradlew" >&2; exit 1; }

# --- 1. Сборка -------------------------------------------------------------
if [ "$SKIP_BUILD" -eq 0 ]; then
  echo "==> Сборка плагина (buildPlugin)…"
  "$PLUGIN_DIR/gradlew" -p "$PLUGIN_DIR" --console=plain buildPlugin
else
  echo "==> Сборка пропущена (--skip-build)."
fi

# --- 2. Поиск собранного zip ----------------------------------------------
DIST_DIR="$PLUGIN_DIR/build/distributions"
VERSION=$(sed -n 's/^pluginVersion[[:space:]]*=[[:space:]]*//p' "$PLUGIN_DIR/gradle.properties" | tr -d '[:space:]')
ZIP="$DIST_DIR/intellij-takt-$VERSION.zip"
if [ ! -f "$ZIP" ]; then
  # Резерв: самый свежий zip в distributions.
  ZIP=$(ls -t "$DIST_DIR"/*.zip 2>/dev/null | head -1 || true)
fi
[ -n "${ZIP:-}" ] && [ -f "$ZIP" ] || {
  echo "Не найден собранный плагин в $DIST_DIR (запустите без --skip-build)." >&2
  exit 1
}

# Имя папки плагина внутри zip (верхний компонент пути первого элемента архива).
PLUGIN_NAME=$(unzip -Z1 "$ZIP" 2>/dev/null | sed -n '1p' | cut -d/ -f1)
[ -n "$PLUGIN_NAME" ] || { echo "Не удалось определить имя плагина из $ZIP" >&2; exit 1; }
echo "==> Плагин: $PLUGIN_NAME (из $(basename "$ZIP"))"

# --- 3. Каталоги плагинов RustRover ---------------------------------------
# На macOS плагины лежат в <config>/RustRover<ver>/plugins; на Linux — прямо в
# <data>/RustRover<ver>. Пробегаем все найденные версии.
OS=$(uname -s)
case "$OS" in
  Darwin) JB_BASE="$HOME/Library/Application Support/JetBrains"; MAC=1 ;;
  Linux)  JB_BASE="$HOME/.local/share/JetBrains";                MAC=0 ;;
  *) echo "ОС '$OS' не поддерживается этим скриптом (macOS/Linux)." >&2; exit 1 ;;
esac

[ -d "$JB_BASE" ] || { echo "Каталог JetBrains не найден: $JB_BASE" >&2; exit 1; }

INSTALLED=0
RUNNING=0
for RR in "$JB_BASE"/RustRover*; do
  [ -d "$RR" ] || continue   # шаблон без совпадений → пропускаем литерал
  if [ "$MAC" -eq 1 ]; then
    PLUGINS_DIR="$RR/plugins"
    LOCK="$RR/.lock"
  else
    PLUGINS_DIR="$RR"
    LOCK="$HOME/.config/JetBrains/$(basename "$RR")/.lock"
  fi

  mkdir -p "$PLUGINS_DIR"
  TARGET="$PLUGINS_DIR/$PLUGIN_NAME"
  ACTION="установлен"
  [ -d "$TARGET" ] && ACTION="обновлён"
  rm -rf "$TARGET"
  unzip -q -o "$ZIP" -d "$PLUGINS_DIR"

  echo "==> [$( basename "$RR")] плагин $ACTION → $TARGET"
  INSTALLED=$((INSTALLED + 1))
  [ -f "$LOCK" ] && RUNNING=1
done

if [ "$INSTALLED" -eq 0 ]; then
  echo "RustRover не найден в $JB_BASE (нет каталогов RustRover*)." >&2
  echo "Установите RustRover и запустите его хотя бы раз, затем повторите." >&2
  exit 1
fi

echo "==> Готово: установок обновлено — $INSTALLED."
if [ "$RUNNING" -eq 1 ]; then
  echo "!!  RustRover, похоже, запущен — перезапустите IDE, чтобы применить плагин."
else
  echo "    Запустите/перезапустите RustRover, чтобы плагин загрузился."
fi
