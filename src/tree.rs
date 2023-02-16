use bincode::Decode;
use bincode::Encode;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use ara_parser::tree::Tree;
use ara_source::source::Source;
use ara_source::source::SourceTrait;

use crate::config::Config;
use crate::error::Error;
use crate::ARA_CACHED_SOURCE_EXTENSION;

#[derive(Debug, Hash, Encode, Decode)]
pub struct SignedTree {
    pub signature: u64,
    pub tree: Tree,
}

pub struct TreeBuilder<'a> {
    config: &'a Config,
}

impl<'a> TreeBuilder<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    pub fn build(&self, source: &Source) -> Result<Tree, Error> {
        if self.config.cache.is_none() {
            return ara_parser::parser::parse(source).map_err(Error::ParseError);
        }

        let cached_file_path = self.get_cached_file_path(source);
        let tree = self.get_from_cache(source, &cached_file_path).or_else(
            |error| -> Result<Tree, Error> {
                if let Error::DeserializeError(_) = error {
                    log::error!(
                        "error while deserializing cached file ({}) for source ({}): {}",
                        &cached_file_path.to_string_lossy(),
                        source.origin.as_ref().unwrap(),
                        error
                    );
                }

                let tree = ara_parser::parser::parse(source).map_err(Error::ParseError)?;
                self.save_to_cache(source, tree, &cached_file_path)
            },
        )?;

        Ok(tree)
    }

    fn get_from_cache(&self, source: &Source, cached_file_path: &PathBuf) -> Result<Tree, Error> {
        let signed_tree = self
            .config
            .serializer
            .deserialize(&fs::read(cached_file_path)?)?;

        if signed_tree.signature != source.hash()? {
            log::warn!(
                "cache miss due to source change ({}).",
                source.origin.as_ref().unwrap(),
            );

            return Err(Error::CacheMiss);
        }

        log::info!(
            "loaded ({}) parsed source from cache ({}).",
            source.origin.as_ref().unwrap(),
            &cached_file_path.to_string_lossy(),
        );

        Ok(signed_tree.tree)
    }

    fn save_to_cache(
        &self,
        source: &Source,
        tree: Tree,
        cached_file_path: &PathBuf,
    ) -> Result<Tree, Error> {
        let signed_tree = SignedTree::new(source.hash()?, tree);

        let serialized = self.config.serializer.serialize(&signed_tree)?;
        let mut file = File::create(cached_file_path)?;
        file.write_all(&serialized)?;

        log::info!(
            "saved ({}) parsed source to cache ({}).",
            &signed_tree.tree.source,
            &cached_file_path.to_string_lossy(),
        );

        Ok(signed_tree.tree)
    }

    fn get_cached_file_path(&self, source: &Source) -> PathBuf {
        let cache_path = self.config.cache.as_ref().unwrap();
        cache_path
            .join(
                self.config
                    .hasher
                    .hash(source.origin.as_ref().unwrap())
                    .to_string(),
            )
            .with_extension(ARA_CACHED_SOURCE_EXTENSION)
    }
}

impl SignedTree {
    pub fn new(signature: u64, tree: Tree) -> Self {
        Self { signature, tree }
    }
}
