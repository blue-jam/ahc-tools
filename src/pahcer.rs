use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub(crate) struct ExecResult {
    pub(crate) case_count: usize,
    pub(crate) total_score: usize,
}
