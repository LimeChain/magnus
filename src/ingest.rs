pub trait Ingest {
    fn spawn<T: State>(state: T) -> eyre::Result<()>;
}
