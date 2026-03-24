use crate::diagnostics::Diagnostic;
use crate::parser::ast::ImportPath;
use itertools::Itertools;
use std::fs::{exists, read_to_string};
pub fn read_import_file(
    search_paths: &[String],
    path: &ImportPath,
) -> Result<(String, String), Diagnostic> {
    let files = match path {
        ImportPath::Filename(filename) => search_paths
            .iter()
            .map(|path| format!("{}/{}", &path, filename.string))
            .collect::<Vec<String>>(),
        ImportPath::Path(path) => {
            let inner_path = path.identifiers.iter().map(|i| i.name.clone()).join("/");
            search_paths
                .iter()
                .map(|path| format!("{}/{}", path, inner_path))
                .collect::<Vec<String>>()
        }
    };

    let files = files
        .iter()
        .filter(|filename| exists(filename).is_ok_and(|r| r))
        .cloned()
        .collect::<Vec<String>>();
    if files.is_empty() {
        return Err(format!("Import {:?} not found", path).as_str().into());
    }
    let filename = files.first().unwrap();
    if filename.ends_with("\\.but") {
        return Err(format!("Illegal file extension: {:?}", path)
            .as_str()
            .into());
    }
    let content = read_to_string(filename).map_err(|e| e.to_string().as_str().into())?;
    Ok((content, filename.clone()))
}
