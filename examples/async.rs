use ara_parser::tree::Tree;
use ara_source::source::Source;
use ara_source::source::SourceKind;
use std::path::PathBuf;


use tokio::fs;
use tokio::sync::mpsc;
use walkdir::WalkDir;

const ARA_SOURCE_EXTENSION: &str = "ara";
const ARA_DEFINITION_EXTENSION: &str = "d.ara";
const ARA_CACHED_SOURCE_EXTENSION: &str = "ara.cache";

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(parse());
}

async fn parse() {
    let source_dir = PathBuf::from(format!("{MANIFEST_DIR}/examples/project/src"));
    let sources = collect_sources(source_dir).await;
    let chunk_size = sources.len() / num_cpus::get();

    let (tx, mut rx) = mpsc::channel(sources.len());

    let mut handles = Vec::new();
    for chunk in sources.chunks(chunk_size) {
        let chunk = chunk.to_vec();
        let tx = tx.clone();

        let handle = tokio::spawn(async move {
            let mut sub_handles = Vec::with_capacity(chunk.len());
            for source_path in chunk {
                sub_handles.push(tokio::spawn(build_tree(source_path)));
            }

            for sub_handle in sub_handles {
                let (source, tree) = sub_handle.await.unwrap();

                tx.send((source, tree));
            }
        });
        handles.push(handle);
    }

    drop(tx);

    for handle in handles {
        handle.await.unwrap();
    }

    let mut results = vec![];
    while let Some(result) = rx.recv().await {
        results.push(result);
    }

    for (source, _tree) in results {
        println!("Source: {}", &source.origin.clone().unwrap());
    }
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
