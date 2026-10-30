//! `Ctrl+K` search: one query matched against grouped commands and refs.
//!
//! Matching is pure so it can be tested without a window; the browser builds
//! the items from the same command definitions and readiness as its menus.

/// Result groups, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    Filter,
    Actions,
    Branches,
    Remotes,
    Tags,
    Files,
}

impl Group {
    pub fn title(self) -> &'static str {
        match self {
            Self::Filter => "FILTER",
            Self::Actions => "ACTIONS",
            Self::Branches => "BRANCHES",
            Self::Remotes => "REMOTES",
            Self::Tags => "TAGS",
            Self::Files => "FILES",
        }
    }
}

/// One selectable result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item<C> {
    pub group: Group,
    pub label: String,
    /// Secondary text, such as the command's menu home or a ref's commit.
    pub context: String,
    /// Shortcut shown at the right edge, if any.
    pub key: &'static str,
    /// Why the command cannot run now; shown instead of running it.
    pub disabled: Option<String>,
    pub command: C,
}

/// Most results shown per group, so one large group cannot hide the others.
pub const GROUP_LIMIT: usize = 8;

/// Whether every whitespace-separated word of `query` occurs in `text`,
/// ignoring case.
pub fn matches(query: &str, text: &str) -> bool {
    let text = text.to_lowercase();
    query
        .split_whitespace()
        .all(|word| text.contains(&word.to_lowercase()))
}

/// Results for `query`: a leading Filter item for a non-empty query, then the
/// matching items in group order, labels starting with the query first, at
/// most [`GROUP_LIMIT`] per group. An empty query lists the actions.
pub fn results<C: Clone>(query: &str, items: &[Item<C>], filter: C) -> Vec<Item<C>> {
    let query = query.trim();
    let mut out = Vec::new();
    if !query.is_empty() {
        out.push(Item {
            group: Group::Filter,
            label: format!("Filter history by “{query}”"),
            context: "message, author, or ID".into(),
            key: "Enter",
            disabled: None,
            command: filter,
        });
    }
    let mut matched = items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            if query.is_empty() {
                item.group == Group::Actions
            } else {
                matches(query, &format!("{} {}", item.label, item.context))
            }
        })
        .map(|(index, item)| {
            let prefix =
                !query.is_empty() && item.label.to_lowercase().starts_with(&query.to_lowercase());
            (item.group, !prefix, index, item)
        })
        .collect::<Vec<_>>();
    matched.sort_by_key(|(group, not_prefix, index, _)| (*group, *not_prefix, *index));
    let mut shown = std::collections::HashMap::new();
    for (group, _, _, item) in matched {
        let count = shown.entry(group).or_insert(0);
        if *count < GROUP_LIMIT {
            *count += 1;
            out.push(item.clone());
        }
    }
    out
}

/// Up to [`GROUP_LIMIT`] tracked files matching `query`, file names that
/// start with the query first. An empty query lists none.
pub fn files<C>(query: &str, paths: &[String], command: impl Fn(&str) -> C) -> Vec<Item<C>> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let lower = query.to_lowercase();
    let mut matched = paths
        .iter()
        .filter(|path| matches(query, path))
        .map(|path| {
            let name = path.rsplit('/').next().unwrap_or(path);
            (!name.to_lowercase().starts_with(&lower), path.len(), path)
        })
        .collect::<Vec<_>>();
    matched.sort();
    matched
        .into_iter()
        .take(GROUP_LIMIT)
        .map(|(_, _, path)| Item {
            group: Group::Files,
            label: path.clone(),
            context: "open its latest change".into(),
            key: "",
            disabled: None,
            command: command(path),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(group: Group, label: &str, context: &str) -> Item<String> {
        Item {
            group,
            label: label.into(),
            context: context.into(),
            key: "",
            disabled: None,
            command: label.into(),
        }
    }

    #[test]
    fn every_word_must_match_ignoring_case() {
        assert!(matches("pull REB", "Pull (rebase) · Repository menu"));
        assert!(!matches("pull merge", "Pull (rebase) · Repository menu"));
        assert!(matches("", "anything"));
    }

    #[test]
    fn filter_comes_first_then_groups_in_order_with_prefix_matches_first() {
        let items = vec![
            item(Group::Branches, "rebase-ui", "branch"),
            item(Group::Actions, "Rebase current onto…", "Branch menu"),
            item(Group::Actions, "Pull (rebase)", "Repository menu"),
            item(Group::Tags, "v1", "tag"),
        ];
        let labels = results("reb", &items, "filter".to_string())
            .into_iter()
            .map(|item| (item.group, item.label))
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                (Group::Filter, "Filter history by “reb”".to_string()),
                (Group::Actions, "Rebase current onto…".to_string()),
                (Group::Actions, "Pull (rebase)".to_string()),
                (Group::Branches, "rebase-ui".to_string()),
            ]
        );
    }

    #[test]
    fn empty_query_lists_actions_without_a_filter_item() {
        let items = vec![
            item(Group::Actions, "Fetch", "Repository menu"),
            item(Group::Branches, "main", "branch"),
        ];
        let results = results("  ", &items, "filter".to_string());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, "Fetch");
    }

    #[test]
    fn files_prefer_names_that_start_with_the_query() {
        let paths = vec![
            "docs/plan.md".to_string(),
            "src/plan_view.rs".to_string(),
            "src/replan.rs".to_string(),
            "README.md".to_string(),
        ];
        let found = files("plan", &paths, str::to_string)
            .into_iter()
            .map(|item| item.label)
            .collect::<Vec<_>>();
        assert_eq!(found, ["docs/plan.md", "src/plan_view.rs", "src/replan.rs"]);
        assert!(files(" ", &paths, str::to_string).is_empty());
    }

    #[test]
    fn each_group_is_capped() {
        let items = (0..20)
            .map(|i| item(Group::Branches, &format!("feature-{i}"), "branch"))
            .collect::<Vec<_>>();
        let results = results("feature", &items, "filter".to_string());
        assert_eq!(results.len(), 1 + GROUP_LIMIT);
    }
}
