//! Which paths `.gitattributes` marks as having no text diff, read in
//! process as Git reads them for `git diff`: the worktree's attribute
//! files (the index's in a bare repository), `info/attributes`, and
//! `core.attributesFile`.
use gix::bstr::BStr;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The note a comparison carries when attributes made it binary.
pub const NOTE: &str = "Binary content (.gitattributes)";

/// Attribute lookups for one repository, kept across paths.
pub struct DiffAttributes {
    stack: gix::worktree::Stack,
    outcome: gix::attrs::search::Outcome,
}

impl DiffAttributes {
    pub fn open(repo: &gix::Repository) -> Result<Self> {
        let index = repo.index_or_empty()?;
        let source = if repo.workdir().is_some() {
            gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping
        } else {
            gix::worktree::stack::state::attributes::Source::IdMapping
        };
        let stack = repo.attributes_only(&index, source)?;
        let outcome = stack.selected_attribute_matches(["diff"]);
        Ok(Self {
            stack: stack.detach(),
            outcome,
        })
    }

    /// Whether `path` (repository-relative) has `diff` unset, by `-diff` or
    /// the `binary` macro, so Git shows "Binary files differ" for it.
    pub fn binary(&mut self, repo: &gix::Repository, path: &[u8]) -> Result<bool> {
        let platform = self.stack.at_entry(BStr::new(path), None, &repo.objects)?;
        platform.matching_attributes(&mut self.outcome);
        Ok(self
            .outcome
            .iter_selected()
            .any(|matched| matched.assignment.state.is_unset()))
    }
}
