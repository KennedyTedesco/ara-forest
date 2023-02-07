use ara_parser::tree::Tree;
use ara_source::source::Source;
use ara_source::source::SourceKind;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::{fs, task};
use walkdir::WalkDir;

const ARA_SOURCE_EXTENSION: &str = "ara";
const ARA_DEFINITION_EXTENSION: &str = "d.ara";
const ARA_CACHED_SOURCE_EXTENSION: &str = "ara.cache";

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

const NUM_EXECUTORS: usize = 6;

fn main() {
    let source_dir = PathBuf::from(format!("{MANIFEST_DIR}/examples/project/src"));
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        parse(source_dir).await;
    });
}

async fn parse(source_dir: PathBuf) {
    let sources = collect_sources(source_dir).await;
    let sources_len = sources.len();
    let chunked_sources = chunk_sources(sources, NUM_EXECUTORS);

    let (tx, mut rx) = mpsc::channel(sources_len);
    let mut handles = Vec::with_capacity(NUM_EXECUTORS);
    for chunk in chunked_sources {
        let tx = tx.clone();
        handles.push(task::spawn(async move {
            for source in chunk {
                let result = build_tree(source).await;
                tx.send(result).await.unwrap();
            }
        }));
    }

    let mut results = Vec::with_capacity(sources_len);
    for _ in 0..sources_len {
        if let Some(result) = rx.recv().await {
            results.push(result);
        }
    }

    // for (source, _tree) in results {
    //     println!("Source: {}", source.origin.unwrap());
    // }
}

fn chunk_sources(sources: Vec<PathBuf>, chunk_size: usize) -> Vec<Vec<PathBuf>> {
    let mut chunked_sources = vec![];
    let chunk_count = (sources.len() + chunk_size - 1) / chunk_size;
    for i in 0..chunk_count {
        let start = i * chunk_size;
        let end = std::cmp::min(start + chunk_size, sources.len());
        chunked_sources.push(sources[start..end].to_vec());
    }
    chunked_sources
}

async fn build_tree(source_path: PathBuf) -> (Source, Tree) {
    let contents = fs::read_to_string(&source_path).await.unwrap();

    let kind = match source_path.extension() {
        Some(extension) if extension == ARA_DEFINITION_EXTENSION => SourceKind::Definition,
        _ => SourceKind::Script,
    };

    let origin = &source_path
        .strip_prefix("")
        .map(|path| path.to_string_lossy())
        .unwrap()
        .to_string();

    let source = Source::new(kind, origin, contents);
    let tree = ara_parser::parser::parse(&source).unwrap();

    (source, tree)
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
