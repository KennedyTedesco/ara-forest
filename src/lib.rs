use std::fs;
use std::thread;

use ara_parser::tree::Tree;
use ara_parser::tree::TreeMap;
use ara_reporting::Report;
use ara_source::source::Source;
use ara_source::SourceMap;

use crate::config::Config;
use crate::error::Error;
use crate::source::SourcesBuilder;
use crate::tree::TreeBuilder;

pub mod config;
pub mod error;
pub(crate) mod hash;
pub mod logger;
pub(crate) mod serializer;
pub mod source;
pub(crate) mod tree;

pub(crate) const ARA_SOURCE_EXTENSION: &str = "ara";
pub(crate) const ARA_DEFINITION_EXTENSION: &str = "d.ara";
pub(crate) const ARA_CACHED_SOURCE_EXTENSION: &str = "ara.cache";

#[derive(Debug)]
pub struct Forest {
    pub source: SourceMap,
    pub tree: TreeMap,
}

impl Forest {
    pub fn new(source: SourceMap, tree: TreeMap) -> Self {
        Self { source, tree }
    }
}

pub struct Parser<'a> {
    pub config: &'a Config,
    tree_builder: TreeBuilder<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(config: &'a Config) -> Self {
        Parser {
            config,
            tree_builder: TreeBuilder::new(config),
        }
    }

    pub fn parse(&self) -> Result<Forest, Box<Report>> {
        self.init_logger().map_err(|error| Box::new(error.into()))?;

        let sources = SourcesBuilder::new(self.config)
            .build()
            .map_err(|error| Box::new(error.into()))?;

        if sources.is_empty() {
            return Ok(Forest::new(
                SourceMap::new(Vec::new()),
                TreeMap::new(Vec::new()),
            ));
        }

        let threads_count = self.threads_count(sources.len());
        let chunks = sources
            .chunks(sources.len() / threads_count)
            .map(|chunk| chunk.iter().collect::<Vec<&Source>>())
            .collect::<Vec<Vec<&Source>>>();

        let trees = thread::scope(|scope| -> Result<Vec<Tree>, Box<Report>> {
            self.create_cache_dir()
                .map_err(|error| Box::new(error.into()))?;

            let mut threads = Vec::with_capacity(threads_count);
            for sources_chunk in chunks.into_iter() {
                threads.push(scope.spawn(move || -> Result<Vec<Tree>, Box<Report>> {
                    let mut trees = Vec::with_capacity(sources_chunk.len());
                    for source in sources_chunk {
                        let tree =
                            self.tree_builder
                                .build(source)
                                .map_err(|error| match error {
                                    Error::ParseError(report) => report,
                                    _ => Box::new(error.into()),
                                })?;
                        trees.push(tree);
                    }

                    Ok(trees)
                }));
            }

            let mut trees = Vec::with_capacity(sources.len());
            for handle in threads {
                trees.extend(handle.join().unwrap()?);
            }

            Ok(trees)
        })?;

        Ok(Forest::new(SourceMap::new(sources), TreeMap::new(trees)))
    }

    fn threads_count(&self, files_len: usize) -> usize {
        if self.config.threads > files_len {
            files_len
        } else {
            self.config.threads
        }
    }

    fn create_cache_dir(&self) -> Result<(), Error> {
        if self.config.cache.is_some() {
            fs::create_dir_all(self.config.cache.as_ref().unwrap())?;
        }

        Ok(())
    }

    fn init_logger(&self) -> Result<(), Error> {
        if self.config.logger.is_some() {
            self.config.logger.as_ref().unwrap().init()?
        }

        Ok(())
    }
}
