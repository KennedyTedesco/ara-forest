use ara_parser::tree::Tree;
use ara_source::source::Source;
use ara_source::source::SourceKind;
use bincode::config;
use bincode::Decode;
use bincode::Encode;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

const ARA_SOURCE_EXTENSION: &str = "ara";
const ARA_DEFINITION_EXTENSION: &str = "d.ara";
const ARA_CACHED_SOURCE_EXTENSION: &str = "ara.cache";

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Debug, Hash, Encode, Decode)]
pub struct SignedTree {
    pub signature: u64,
    pub tree: Tree,
}

fn main() {
    let source_dir = PathBuf::from(format!("{MANIFEST_DIR}/examples/project/src"));
    let cache_dir = PathBuf::from(format!("{MANIFEST_DIR}/examples/project/.cache"));

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let sources = collect_sources(source_dir).await;
        let mut trees = Vec::with_capacity(sources.len());

        let mut handles = Vec::with_capacity(sources.len());
        for source_path in sources {
            handles.push(tokio::spawn(build_tree(source_path, cache_dir.clone())));
        }

        for handle in handles {
            let (source, tree) = handle.await.unwrap();
            trees.push(tree);
        }
    })
}

async fn build_tree(source_path: PathBuf, cache_dir: PathBuf) -> (Source, Tree) {
    let contents = fs::read_to_string(&source_path).await.unwrap();

    let kind = match source_path.extension() {
        Some(extension) if extension == ARA_DEFINITION_EXTENSION => SourceKind::Definition,
        _ => SourceKind::Script,
    };

    let origin = strip_root(&source_path);
    let source = Source::new(kind, origin, contents);
    let tree = get_from_cache(&source, &cache_dir).await;

    (source, tree)
}

fn strip_root(path: &Path) -> String {
    let root = PathBuf::from(format!("{MANIFEST_DIR}/examples/project"));

    path.strip_prefix(root)
        .map(|path| path.to_string_lossy())
        .unwrap()
        .to_string()
}

async fn get_from_cache(source: &Source, cache_dir: &PathBuf) -> Tree {
    let file_path = cache_dir
        .join(hash(source.origin.as_ref().unwrap()).to_string())
        .with_extension(ARA_CACHED_SOURCE_EXTENSION);

    let data = fs::read(&file_path).await.unwrap();

    let (signed_tree, _): (SignedTree, _) =
        bincode::decode_from_slice(&data, config::standard()).unwrap();

    signed_tree.tree
}

async fn collect_sources(dir: PathBuf) -> Vec<PathBuf> {
    let mut sources = vec![];
    for entry in WalkDir::new(dir) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            sources.push(entry.path().to_owned());
        }
    }
    sources
}

fn hash(content: &str) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    hasher.write(content.as_bytes());
    hasher.finish()
}
