use std::cell::RefCell;
use std::fmt::Write;

pub struct Printer<'a> {
    indent_size: usize,
    indent: usize,
    writer: &'a mut dyn Write,
    padding: RefCell<String>,
}

impl<'a> Printer<'a> {
    pub fn new(writer: &'a mut dyn Write) -> Self {
        Self {
            indent: 0,
            indent_size: 4,
            padding: RefCell::new(String::new()),
            writer,
        }
    }

    pub fn up(&mut self) -> &Self {
        self.indent += 1;
        self.calculate_padding();
        self
    }

    pub fn down(&mut self) -> &Self {
        self.indent -= 1;
        self.calculate_padding();
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

    pub fn print(&mut self, message: &str) -> &Self {
        self.writer
            .write_fmt(format_args!("{}{}", &*self.padding.borrow(), message))
            .unwrap();
        self
    }
}
