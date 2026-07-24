#[cfg(test)]
pub mod tests {
    pub const SRC: &str = r#"
model A {
    start Start;
}
model B {
    start Start;
}
start Entry = A | B | (A + B) {
    next Next1;
}
state Next1 = A + B + (A | B) {
    next Next2;
}
state Next2 = A + (B | A) + B {
    next Next3;
}
state Next3 = A + (B + A) + B {
    next Next4;
}
state Next4 = A + (B + A) + (B | A) {
    next Next5;
}
state Next5 = (A | B) + (A + B) {
    next Next6;
}
state Next6 = (A | B) + (A + B) + (A | B) {
    next Next7;
}
state Next7 = (A | B) + (A + B) + (A | B) + (A + B) {
    next Next8;
}
state Next8 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) {
    next Next9;
}
state Next9 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B) {
    next Next10;
}
state Next10 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B) + (A + B);
"#;
}
