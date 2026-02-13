//! Context state structures.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use radicle::cob::common::{Author, Timestamp};
use radicle::cob::ObjectId;

/// A file-specific code learning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeLearning {
    /// File path relative to repo root.
    pub path: String,
    /// Optional start line number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// Optional end line number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    /// What was discovered about this code.
    pub finding: String,
}

/// Summary of learnings from a development session.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningsSummary {
    /// Repository-level patterns and conventions discovered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repo: Vec<String>,
    /// File-specific code findings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub code: Vec<CodeLearning>,
}

/// Context state. Accumulates [`Action`](crate::Action).
///
/// Represents an observation record from an AI-assisted development session.
/// Core fields are immutable (set at creation). Only link fields support
/// mutation via set operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    /// Brief identifier for the session.
    pub(crate) title: String,
    /// Free-form description (for standalone contexts without a plan).
    pub(crate) description: String,
    /// What was tried, why the chosen path won, what was deliberately not done and why.
    pub(crate) approach: String,
    /// Assumptions the work depends on — "valid as long as X remains true".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) constraints: Vec<String>,
    /// What was discovered about the codebase.
    #[serde(default)]
    pub(crate) learnings: LearningsSummary,
    /// Problems encountered — prevents future agents from repeating mistakes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) friction: Vec<String>,
    /// Unfinished work, tech debt introduced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) open_items: Vec<String>,
    /// Which files were actually modified.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub(crate) files_touched: BTreeSet<String>,
    /// Git commits this context produced.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub(crate) related_commits: BTreeSet<String>,
    /// Linked Radicle issues.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub(crate) related_issues: BTreeSet<ObjectId>,
    /// Linked Radicle patches.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub(crate) related_patches: BTreeSet<ObjectId>,
    /// Linked Radicle plans.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub(crate) related_plans: BTreeSet<ObjectId>,
    /// The Radicle identity who ran the session.
    pub(crate) author: Author,
    /// When the context was created.
    pub(crate) created_at: Timestamp,
}

impl Context {
    /// Create a new context.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        title: String,
        description: String,
        approach: String,
        constraints: Vec<String>,
        learnings: LearningsSummary,
        friction: Vec<String>,
        open_items: Vec<String>,
        files_touched: BTreeSet<String>,
        author: Author,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            title,
            description,
            approach,
            constraints,
            learnings,
            friction,
            open_items,
            files_touched,
            related_commits: BTreeSet::new(),
            related_issues: BTreeSet::new(),
            related_patches: BTreeSet::new(),
            related_plans: BTreeSet::new(),
            author,
            created_at: timestamp,
        }
    }

    /// Get the context title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the context description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the approach taken.
    pub fn approach(&self) -> &str {
        &self.approach
    }

    /// Get the constraints.
    pub fn constraints(&self) -> &[String] {
        &self.constraints
    }

    /// Get the learnings summary.
    pub fn learnings(&self) -> &LearningsSummary {
        &self.learnings
    }

    /// Get the friction points.
    pub fn friction(&self) -> &[String] {
        &self.friction
    }

    /// Get the open items.
    pub fn open_items(&self) -> &[String] {
        &self.open_items
    }

    /// Get the files touched.
    pub fn files_touched(&self) -> &BTreeSet<String> {
        &self.files_touched
    }

    /// Get the related commits.
    pub fn related_commits(&self) -> &BTreeSet<String> {
        &self.related_commits
    }

    /// Get the related issues.
    pub fn related_issues(&self) -> &BTreeSet<ObjectId> {
        &self.related_issues
    }

    /// Get the related patches.
    pub fn related_patches(&self) -> &BTreeSet<ObjectId> {
        &self.related_patches
    }

    /// Get the related plans.
    pub fn related_plans(&self) -> &BTreeSet<ObjectId> {
        &self.related_plans
    }

    /// Get the context author.
    pub fn author(&self) -> &Author {
        &self.author
    }

    /// Get when the context was created.
    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_learning_serde() {
        let learning = CodeLearning {
            path: "src/lib.rs".to_string(),
            line: Some(42),
            end_line: Some(50),
            finding: "Uses builder pattern for COB construction".to_string(),
        };

        let json = serde_json::to_string(&learning).expect("serialization failed");
        let deserialized: CodeLearning =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(learning, deserialized);
    }

    #[test]
    fn test_code_learning_optional_lines() {
        let learning = CodeLearning {
            path: "src/state.rs".to_string(),
            line: None,
            end_line: None,
            finding: "State uses BTreeSet for sorted collections".to_string(),
        };

        let json = serde_json::to_string(&learning).expect("serialization failed");
        assert!(!json.contains("line"));
        assert!(!json.contains("endLine"));

        let deserialized: CodeLearning =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(learning, deserialized);
    }

    #[test]
    fn test_learnings_summary_serde() {
        let summary = LearningsSummary {
            repo: vec!["Uses conventional commits".to_string()],
            code: vec![CodeLearning {
                path: "src/main.rs".to_string(),
                line: Some(10),
                end_line: None,
                finding: "Entry point delegates to run()".to_string(),
            }],
        };

        let json = serde_json::to_string(&summary).expect("serialization failed");
        let deserialized: LearningsSummary =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(summary, deserialized);
    }

    #[test]
    fn test_learnings_summary_default() {
        let summary = LearningsSummary::default();
        assert!(summary.repo.is_empty());
        assert!(summary.code.is_empty());

        // Empty summary should round-trip through JSON
        let json = serde_json::to_string(&summary).expect("serialization failed");
        let deserialized: LearningsSummary =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(summary, deserialized);
    }
}
