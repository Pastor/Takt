# Грамматика (EBNF)

Формальный синтаксис языка Takt в нотации EBNF. Раздел — нормативный источник по
структуре программы; текстовые разделы описывают смысл конструкций, а грамматика —
их допустимую форму.

> **Раздел в разработке.** Полная грамматика приводится здесь в соответствии с
> эталоном `Takt.ebnf`. Ниже — иллюстративный фрагмент; он не полон.

```ebnf
model        = "model" , [ identifier ] , "{" , { model-elem } , "}" ;
model-elem   = state | var-decl | port-decl | fn-decl | cond-decl | named-block ;
state        = [ "start" ] , "state" , identifier , "{" , { state-elem } , "}" ;
state-elem   = named-block | transition ;
transition   = "ref" , identifier , ":" , condition , ";" ;
port-decl    = ( "in" | "out" | "inout" ) , identifier , ":" , type ,
               [ ":=" , expression ] , ";" ;
```

<!-- Источник истины формы — Takt.ebnf; при изменении синтаксиса языка правятся
     оба (грамматика-эталон и этот раздел). -->
