use std::cell::RefCell;
use std::fmt::Write;

pub struct Printer<'a> {
    indent_size: usize,
    indent: usize,
    writer: &'a mut dyn Write,
    padding: RefCell<String>,
}

impl<'a> Printer<'a> {
    pub fn new(indent_size: usize, writer: &'a mut dyn Write) -> Self {
        Self {
            indent: 0,
            indent_size,
            padding: RefCell::new(String::new()),
            writer,
        }
    }

    pub fn up(&mut self) -> &mut Self {
        self.indent += 1;
        self.calculate_padding();
        self
    }

    pub fn down(&mut self) -> &mut Self {
        self.indent -= 1;
        self.calculate_padding();
        self
    }

    pub fn nl(&mut self) -> &mut Self {
        self.writer.write_char('\n').unwrap();
        self
    }

    fn calculate_padding(&mut self) {
        let mut padding = String::new();
        for _ in 0..self.indent {
            for _ in 0..self.indent_size {
                padding.push(' ');
            }
        }
        self.padding.replace(padding);
    }

    pub fn print(&mut self, message: &str) -> &mut Self {
        self.writer.write_fmt(format_args!("{}", message)).unwrap();
        self
    }

    pub fn ident(&mut self, message: &str) -> &mut Self {
        self.writer
            .write_fmt(format_args!("{}{}", &*self.padding.borrow(), message))
            .unwrap();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn print_padding() {
        let mut output = String::new();
        let mut printer = Printer::new(4, &mut output);
        printer
            .print("struct A {")
            .nl()
            .up()
            .ident("value: u8;")
            .nl()
            .down()
            .print("}");
        assert_eq!(
            output,
            r#"struct A {
    value: u8;
}"#
        );
    }
}
