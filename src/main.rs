use std::path::PathBuf;

#[tokio::main]
async fn main() {
    serve_dir::serve(PathBuf::from("./html"), 8081, true, None)
        .await
        .unwrap();
}
