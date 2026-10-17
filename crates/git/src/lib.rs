//! Native Git object access through gitoxide.
use gg_engine::{Commit, HISTORY_LIMIT, History, HistoryRequest, HistorySource};

pub struct Git;

impl HistorySource for Git {
    fn history(&self, request: &HistoryRequest) -> Result<History, String> {
        read_history(request)
            .map_err(|error| format!("Could not read {}: {error}", request.path.display()))
    }
}

fn read_history(request: &HistoryRequest) -> Result<History, Box<dyn std::error::Error>> {
    let repo = gix::discover(&request.path)?;
    let head = repo.head()?;
    let label = head
        .referent_name()
        .map(|name| name.shorten().to_string())
        .unwrap_or_else(|| "Detached HEAD".into());
    let mut history = History {
        path: repo.workdir().unwrap_or(repo.path()).to_path_buf(),
        head: label,
        commits: Vec::new(),
        truncated: false,
    };
    if head.is_unborn() {
        return Ok(history);
    }
    let commit = repo.head_commit()?;
    for entry in commit.ancestors().all()?.take(HISTORY_LIMIT + 1) {
        let entry = entry?;
        if history.commits.len() == HISTORY_LIMIT {
            history.truncated = true;
            break;
        }
        let commit = repo.find_object(entry.id)?.try_into_commit()?;
        let decoded = commit.decode()?;
        let author = decoded.author()?;
        history.commits.push(Commit {
            id: commit.id.to_string(),
            parents: decoded.parents().map(|id| id.to_string()).collect(),
            author: author.name.to_string(),
            email: author.email.to_string(),
            time: author.time()?.format(gix::date::time::format::ISO8601)?,
            message: decoded.message.to_string(),
        });
    }
    Ok(history)
}
