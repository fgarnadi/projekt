use clap::ValueEnum;

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum SortBy {
    Name,
    LastSync,
    LastModified,
    Branch,
    Commit,
}
