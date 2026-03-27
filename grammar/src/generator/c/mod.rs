use crate::diagnostics::Diagnostic;
use crate::generator::{Generator as AsGenerator, Source};
use crate::semantic::ModelNode;

pub struct Generator {}

impl AsGenerator for Generator {
    fn generate(&self, model: &ModelNode) -> Result<Source, Diagnostic> {
        todo!()
    }
}
