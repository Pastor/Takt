-- Подсветка ключевых слов и предопределённых типов языка Takt в тексте (не в
-- блоках кода).
--
-- Инлайн-код (`model`, `state`, `if`, `u8`, `bit`, …), совпадающий с ключевым
-- словом или предопределённым типом языка, печатается цветом (как подсветка
-- синтаксиса), чтобы в прозе было видно: это элемент языка Takt, а не постороннее
-- слово. Обычный инлайн-код (имена, операторы) и англ. слова не затрагиваются.
--
-- Цвета и начертание совпадают с подсветкой БЛОКОВ кода (skylighting, tango):
--   • ключевые слова — синий #204A87 + ЖИРНЫЙ  (как \KeywordTok);
--   • типы           — синий #204A87 без жирного (как \DataTypeTok).
-- Блоки кода (CodeBlock) не трогаются: они уже подсвечены skylighting.
--
-- Одиночные заглавные X/F/G/U/R (операторы LTL) и `_` (wildcard) НЕ подсвечиваются:
-- слишком высок риск ложных срабатываний в прозе.

-- Ключевые слова (таблица KEYWORDS лексера, без X/F/G/U/R/_).
local keywords = {
  ["address"]=true, ["as"]=true, ["assembly"]=true, ["break"]=true,
  ["cond"]=true, ["const"]=true, ["continue"]=true, ["else"]=true,
  ["enum"]=true, ["extern"]=true, ["false"]=true, ["fn"]=true, ["for"]=true,
  ["formula"]=true, ["from"]=true, ["if"]=true, ["import"]=true, ["in"]=true,
  ["inout"]=true, ["invariant"]=true, ["loop"]=true, ["match"]=true,
  ["model"]=true, ["next"]=true, ["out"]=true, ["ref"]=true, ["return"]=true,
  ["start"]=true, ["state"]=true, ["struct"]=true,
  ["true"]=true, ["type"]=true, ["var"]=true,
  ["while"]=true, ["LTL"]=true, ["Guard"]=true,
}

-- Предопределённые (простые/встроенные) типы данных.
local types = {
  ["bit"]=true, ["bool"]=true, ["float"]=true, ["unit"]=true,
  ["u8"]=true, ["u16"]=true, ["u32"]=true, ["u64"]=true,
  ["i8"]=true, ["i16"]=true, ["i32"]=true, ["i64"]=true,
}

function Code(el)
  if not FORMAT:match('latex') then return nil end
  if keywords[el.text] then
    return pandoc.RawInline(
      'latex',
      '\\textbf{\\textcolor[HTML]{204A87}{\\texttt{' .. el.text .. '}}}'
    )
  elseif types[el.text] then
    return pandoc.RawInline(
      'latex',
      '\\textcolor[HTML]{204A87}{\\texttt{' .. el.text .. '}}'
    )
  end
end
