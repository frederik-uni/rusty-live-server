use std::{
    future::Future,
    path::{Path, PathBuf},
};

use tokio::{
    fs::{read_dir, File as TokioFile, ReadDir},
    io::AsyncReadExt as _,
};

use crate::Error;

pub trait FileSystemInterface: Clone + Send {
    fn get_file(&self, path: &Path) -> impl Future<Output = crate::Result<impl File>> + Send;
    fn get_dir(&self, path: &Path) -> impl Future<Output = crate::Result<impl Dir>> + Send;
}

pub trait Dir: Send {
    fn get_next(&mut self) -> impl Future<Output = crate::Result<Option<PathBuf>>> + Send;
}

pub trait File: Send {
    fn read_to_end(&mut self) -> impl Future<Output = Vec<u8>> + Send;
}

#[derive(Default, Clone, Copy)]
pub struct AsyncFileSystem;

struct AsyncDir {
    dir: ReadDir,
}

struct AsyncFile {
    file: TokioFile,
}

impl AsyncFile {
    async fn new(path: &Path) -> crate::Result<Self> {
        let file = TokioFile::open(path).await?;
        Ok(Self { file })
    }
}

impl File for AsyncFile {
    async fn read_to_end(&mut self) -> Vec<u8> {
        let mut buffer = vec![];
        self.file.read_to_end(&mut buffer).await;
        buffer
    }
}

impl AsyncDir {
    async fn new(path: &Path) -> Result<Self, Error> {
        let dir = read_dir(path).await?;
        Ok(Self { dir })
    }
}

impl Dir for AsyncDir {
    async fn get_next(&mut self) -> crate::Result<Option<PathBuf>> {
        Ok(self.dir.next_entry().await?.map(|v| v.path()))
    }
}

impl FileSystemInterface for AsyncFileSystem {
    async fn get_dir(&self, path: &Path) -> crate::Result<impl Dir> {
        AsyncDir::new(path).await
    }

    async fn get_file(&self, path: &Path) -> crate::Result<impl File> {
        AsyncFile::new(path).await
    }
}
