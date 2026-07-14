//! Переассоциация комментариев по байтовым смещениям (задача 0024-02).
//!
//! # Почему это вообще работает
//!
//! Лексер складывает комментарии в отдельный `Vec<Comment>` и **не** включает их
//! в поток токенов, но каждый несёт `Location::Source(файл, начало, конец)` —
//! ровно как и каждый узел АСД. Значит их можно сшить обратно по смещениям, не
//! строя lossless-дерево (решение ADR 0024, Option A).
//!
//! Текст комментария лексер хранит **сырым** (`input[start..end]`, вместе с `//`,
//! `///` и `/* */`), поэтому печатается дословно — форматтер не переписывает
//! содержимое комментариев пользователя.
//!
//! # Классификация
//!
//! - **Ведущий** — начинается раньше узла: печатается на своих строках перед ним.
//! - **Хвостовой** — начинается после конца узла и на **той же** строке
//!   исходника: приклеивается к строке узла (`var x: u8 := 0; // счётчик`).
//! - **Остаточные** — всё, что осталось после последнего узла (хвост файла).

use crate::diagnostics::Location;
use crate::parser::ast;

/// Комментарий, приведённый к смещениям и сырому тексту.
struct Item {
    start: usize,
    end: usize,
    text: String,
}

/// Курсор по комментариям исходника.
///
/// Комментарии выдаются **строго по одному разу**: каждый либо напечатан как
/// ведущий/хвостовой, либо попадёт в [`Comments::rest`]. Потерять комментарий
/// пользователя нельзя — это главное требование R2.
pub(crate) struct Comments<'a> {
    source: &'a str,
    items: Vec<Item>,
    next: usize,
}

impl<'a> Comments<'a> {
    pub(crate) fn new(source: &'a str, comments: &[ast::Comment]) -> Self {
        let mut items: Vec<Item> = comments
            .iter()
            .filter_map(|c| {
                let (Location::Source(_, start, end)) = *location(c) else {
                    return None;
                };
                Some(Item {
                    start,
                    end,
                    text: c.value().trim_end().to_string(),
                })
            })
            .collect();
        // Лексер выдаёт комментарии по порядку, но полагаться на это не будем.
        items.sort_by_key(|i| i.start);
        Self {
            source,
            items,
            next: 0,
        }
    }

    /// Ведущие комментарии: все, что начинаются раньше `offset`.
    pub(crate) fn leading(&mut self, offset: usize) -> Vec<String> {
        let mut out = Vec::new();
        while self.next < self.items.len() && self.items[self.next].start < offset {
            out.push(self.items[self.next].text.clone());
            self.next += 1;
        }
        out
    }

    /// Хвостовой комментарий: начинается после `end` и на той же строке.
    pub(crate) fn trailing(&mut self, end: usize) -> Option<String> {
        let item = self.items.get(self.next)?;
        if item.start < end {
            return None;
        }
        // Между концом узла и началом комментария не должно быть перевода строки.
        let gap = self.source.get(end..item.start)?;
        if gap.contains('\n') {
            return None;
        }
        let text = item.text.clone();
        self.next += 1;
        Some(text)
    }

    /// Всё, что осталось (хвост файла).
    pub(crate) fn rest(&mut self) -> Vec<String> {
        let out: Vec<String> = self.items[self.next..]
            .iter()
            .map(|i| i.text.clone())
            .collect();
        self.next = self.items.len();
        out
    }

    /// Остались ли непогашенные комментарии.
    ///
    /// Страховка требования R2: если после печати что-то осталось необработанным,
    /// это дефект привязки, а не норма.
    pub(crate) fn is_exhausted(&self) -> bool {
        self.next >= self.items.len()
    }
}

fn location(c: &ast::Comment) -> &Location {
    match c {
        ast::Comment::Line(loc, _)
        | ast::Comment::DocLine(loc, _)
        | ast::Comment::Block(loc, _) => loc,
    }
}

/// Достаёт байтовые смещения из `Location`.
pub(crate) fn span(loc: &Location) -> Option<(usize, usize)> {
    match loc {
        Location::Source(_, start, end) => Some((*start, *end)),
        Location::Builtin | Location::CommandLine | Location::Implicit | Location::Codegen => None,
    }
}
