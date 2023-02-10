use std::path::Path;
use walkdir::WalkDir;

use ara_source::source::Source;
use ara_source::source::SourceKind;

use crate::config::Config;
use crate::error::Error;
use crate::ARA_DEFINITION_EXTENSION;
use crate::ARA_SOURCE_EXTENSION;

pub struct SourcesBuilder<'a> {
    config: &'a Config,
}

impl<'a> SourcesBuilder<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    pub fn build(&self) -> Result<Vec<Source>, Error> {
        let mut paths = vec![&self.config.source];
        paths.extend(&self.config.definitions);

        let mut sources = Vec::new();
        for path in paths {
            let path = &self.config.root.join(path);
            if !path.is_dir() {
                return Err(Error::InvalidPath(format!(
                    "{} must be a directory and be relative to the project root directory.",
                    path.display(),
                )));
            }
            for entry in WalkDir::new(path) {
                let entry = entry?;
                if entry.file_type().is_file()
                    && entry.path().extension() == Some(ARA_SOURCE_EXTENSION.as_ref())
                {
                    sources.push(self.build_source(entry.path()));
                }
            }
        }

        Ok(sources)
    }

    fn build_source(&self, source_path: &Path) -> Source {
        let origin = source_path
            .strip_prefix(&self.config.root)
            .map(|path| path.to_string_lossy())
            .unwrap()
            .to_string();

        let kind = match source_path.extension() {
            Some(extension) if extension == ARA_DEFINITION_EXTENSION => SourceKind::Definition,
            _ => SourceKind::Script,
        };

        Source::new(kind, &self.config.root, origin)
    }
}
