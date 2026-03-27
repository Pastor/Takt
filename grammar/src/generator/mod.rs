mod c;
mod indent;

use crate::diagnostics::Diagnostic;
use crate::semantic::ModelNode;

pub type Source = (Option<String>, Option<String>);

pub enum Language {
    C,
}

pub trait Generator {
    fn generate(&self, model: &ModelNode) -> Result<Source, Diagnostic>;
}

pub fn generate(l: Language, model: &ModelNode) -> Result<Source, Diagnostic> {
    match l {
        Language::C => {
            let generator = c::Generator {};
            generator.generate(model)
        }
    }
}
