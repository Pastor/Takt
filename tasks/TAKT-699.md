---
okf_version: 0.2
format: 2
type: Task
key: TAKT-699
list: ТАКТ
title: Кнопка экспорта в панели схемы IntelliJ
status: todo
priority: low
author: pastor
model: gpt-6
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:41:28Z
published: 2026-11-11T07:41:28Z
created: 2026-11-11T07:41:28Z
updated: 2026-11-11T07:41:28Z
anal:
  project: 22
  id: 1756
  version: 1
checksum: sha256:be3697fe6ac27e78c3d5bd03d98d33cfed63aa0b8575e550dc1c96b4cfe78a22
---

Панель схемы в редакторе
собирается из разметки страницы, но модуля в ней нет, и кнопку экспорта сборка
панели вырезает. Экспорт из панели — запуском `takt-sim export` командной
строкой плагина (`TaktCommandLine`, режим `EXPORT`) с проектом открытого
файла. Ждёт закрытия [панели схемы в webview](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0543-scheme-in-editor-webview.md). Tier 4 (новая возможность).
Вынесено из фичи [экспорт проекта: изображения схемы и видео прогона](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0590-project-export.md) (задача 09 плана) при её закрытии.
