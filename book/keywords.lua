-- Подсветка ключевых слов языка Takt в тексте (не в блоках кода).
--
-- Инлайн-код (`model`, `state`, `if`, …), совпадающий с ключевым словом языка,
-- печатается моноширинным + жирным + цветом (как keyword в подсветке), чтобы в
-- прозе было видно: это ключевое слово Takt, а не постороннее слово. Обычный
-- инлайн-код (имена, типы, операторы) и англ. слова не затрагиваются.
--
-- Цвет — tango KeywordTok (#204A87), тот же, что у ключевых слов в блоках кода.
-- Блоки кода (CodeBlock) не трогаются: они уже подсвечены skylighting.
--
-- Одиночные заглавные X/F/G/U/R (операторы LTL) и `_` (wildcard) НЕ подсвечиваются:
-- слишком высок риск ложных срабатываний в прозе.

local keywords = {
  ["address"]=true, ["as"]=true, ["assembly"]=true, ["break"]=true,
  ["cond"]=true, ["const"]=true, ["continue"]=true, ["else"]=true,
  ["enum"]=true, ["extern"]=true, ["false"]=true, ["fn"]=true, ["for"]=true,
  ["formula"]=true, ["from"]=true, ["if"]=true, ["import"]=true, ["in"]=true,
  ["inout"]=true, ["invariant"]=true, ["loop"]=true, ["match"]=true,
  ["model"]=true, ["next"]=true, ["out"]=true, ["ref"]=true, ["return"]=true,
  ["start"]=true, ["state"]=true, ["string"]=true, ["struct"]=true,
  ["template"]=true, ["true"]=true, ["type"]=true, ["var"]=true,
  ["while"]=true, ["LTL"]=true, ["Guard"]=true,
}

function Code(el)
  if keywords[el.text] and FORMAT:match('latex') then
    return pandoc.RawInline(
      'latex',
      '\\textbf{\\textcolor[HTML]{204A87}{\\texttt{' .. el.text .. '}}}'
    )
  end
end
