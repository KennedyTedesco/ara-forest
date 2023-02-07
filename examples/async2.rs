use ara_parser::tree::Tree;
use ara_source::source::Source;
use ara_source::source::SourceKind;
use std::path::PathBuf;
use std::thread;
use tokio::fs;
use tokio::runtime::Builder;
use walkdir::WalkDir;

const ARA_SOURCE_EXTENSION: &str = "ara";
const ARA_DEFINITION_EXTENSION: &str = "d.ara";
const ARA_CACHED_SOURCE_EXTENSION: &str = "ara.cache";

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

const NUM_EXECUTORS: usize = 6;

fn main() {
    let source_dir = PathBuf::from(format!("{MANIFEST_DIR}/examples/project/src"));

    let sources = collect_sources(source_dir);
    let chunks = sources
        .chunks(NUM_EXECUTORS)
        .map(Vec::from)
        .collect::<Vec<Vec<PathBuf>>>();

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_EXECUTORS)
        .spawn_handler(|thread| {
            thread::spawn(|| thread.run());
            Ok(())
        })
        .build()
        .unwrap();

    let mut trees = Vec::with_capacity(sources.len());
    for chunk in chunks {
        pool.install(|| {
            let rt = Builder::new_multi_thread()
                .enable_all()
                .worker_threads(NUM_EXECUTORS)
                .build()
                .unwrap();

            rt.block_on(async {
                let mut handles = Vec::with_capacity(chunk.len());
                for source_path in chunk {
                    handles.push(tokio::spawn(build_tree(source_path)));
                }

                for handle in handles {
                    let (_source, tree) = handle.await.unwrap();
                    trees.push(tree);
                }
            });
        });
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

fn collect_sources(dir: PathBuf) -> Vec<PathBuf> {
    let mut sources = vec![];
    for entry in WalkDir::new(dir) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            sources.push(entry.path().to_owned());
        }
    }
    sources
}
