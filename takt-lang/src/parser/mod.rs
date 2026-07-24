/// Модуль абстрактного синтаксического дерева языка Lam.
pub mod ast;
pub mod ast_expr;

/// Модуль лексического анализатора Lam.
pub mod lexer;
pub mod token;

/// Разбирает строку адресного литерала вида `"0xNNNN:bit"` в пару `(адрес, бит)`.
///
/// Используется в правилах LALRPOP-грамматики для токена [`lexer::Token::AddressLiteral`].
/// Поддерживает шестнадцатеричную (`0x…`) и десятичную форму адреса.
///
/// # Примеры
///
/// ```
/// use takt_lang::parser::parse_address_literal;
/// assert_eq!(parse_address_literal("0x00200000:0"), (2097152, 0));
/// assert_eq!(parse_address_literal("0xFF:3"),       (255, 3));
/// ```
pub fn parse_address_literal(s: &str) -> (i64, i64) {
    if let Some(pos) = s.rfind(':') {
        let addr_part = &s[..pos];
        let bit_part = &s[pos + 1..];
        let addr = if addr_part.starts_with("0x") || addr_part.starts_with("0X") {
            i64::from_str_radix(&addr_part[2..], 16).unwrap_or(0)
        } else {
            addr_part.parse::<i64>().unwrap_or(0)
        };
        let bit = bit_part.parse::<i64>().unwrap_or(0);
        (addr, bit)
    } else {
        (0, 0)
    }
}
