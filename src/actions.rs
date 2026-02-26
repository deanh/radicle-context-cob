//! Context actions that can be applied to the COB state.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use radicle::cob::common::Uri;
use radicle::cob::store::CobAction;
use radicle::cob::{Embed, ObjectId};

use crate::state::{LearningsSummary, VerificationResult};

/// Context action. Represents all possible mutations to a context's state.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    /// Open a new context (initial action). Sets all immutable core fields.
    #[serde(rename = "open")]
    Open {
        /// Brief identifier for the session.
        title: String,
        /// Free-form description (for standalone contexts without a plan).
        #[serde(default, skip_serializing_if = "String::is_empty")]
        description: String,
        /// What was tried, why the chosen path won, deliberate non-decisions.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        approach: String,
        /// Assumptions the work depends on.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        constraints: Vec<String>,
        /// What was discovered about the codebase.
        #[serde(default)]
        learnings: LearningsSummary,
        /// Problems encountered during the session.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        friction: Vec<String>,
        /// Unfinished work, tech debt introduced.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        open_items: Vec<String>,
        /// Which files were actually modified.
        #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
        files_touched: BTreeSet<String>,
        /// Structured verification results from the session.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        verification: Vec<VerificationResult>,
        /// Plan task ID that produced this context.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_id: Option<String>,
        /// Embedded content (e.g. session transcripts as git blobs).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        embeds: Vec<Embed<Uri>>,
    },

    /// Link a git commit SHA to this context.
    #[serde(rename = "link.commit")]
    LinkCommit {
        /// Git commit SHA.
        sha: String,
    },

    /// Unlink a git commit SHA from this context.
    #[serde(rename = "unlink.commit")]
    UnlinkCommit {
        /// Git commit SHA.
        sha: String,
    },

    /// Link a Radicle issue to this context.
    #[serde(rename = "link.issue")]
    LinkIssue {
        /// Issue object ID.
        issue_id: ObjectId,
    },

    /// Unlink a Radicle issue from this context.
    #[serde(rename = "unlink.issue")]
    UnlinkIssue {
        /// Issue object ID.
        issue_id: ObjectId,
    },

    /// Link a Radicle patch to this context.
    #[serde(rename = "link.patch")]
    LinkPatch {
        /// Patch object ID.
        patch_id: ObjectId,
    },

    /// Unlink a Radicle patch from this context.
    #[serde(rename = "unlink.patch")]
    UnlinkPatch {
        /// Patch object ID.
        patch_id: ObjectId,
    },

    /// Link a Radicle plan to this context.
    #[serde(rename = "link.plan")]
    LinkPlan {
        /// Plan object ID.
        plan_id: ObjectId,
    },

    /// Unlink a Radicle plan from this context.
    #[serde(rename = "unlink.plan")]
    UnlinkPlan {
        /// Plan object ID.
        plan_id: ObjectId,
    },
}

impl CobAction for Action {
    fn produces_identifier(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_open_action_serialization() {
        let action = Action::Open {
            title: "Implement auth flow".to_string(),
            description: "Session to add OAuth support".to_string(),
            approach: "Used passport.js for OAuth, rejected manual implementation".to_string(),
            constraints: vec!["Assumes Redis is available for sessions".to_string()],
            learnings: LearningsSummary::default(),
            friction: vec!["Type errors with async middleware".to_string()],
            open_items: vec![],
            files_touched: BTreeSet::from(["src/auth.rs".to_string()]),
            verification: vec![],
            task_id: None,
            embeds: vec![],
        };

        let json = serde_json::to_string(&action).expect("serialization failed");
        assert!(json.contains("\"type\":\"open\""));

        let deserialized: Action = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_link_commit_serialization() {
        let action = Action::LinkCommit {
            sha: "abc1234def5678".to_string(),
        };

        let json = serde_json::to_string(&action).expect("serialization failed");
        assert!(json.contains("\"type\":\"link.commit\""));

        let deserialized: Action = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_link_issue_serialization() {
        let id = ObjectId::from_str("d96f02665bf896324e2f0a4c18d08a768135ef2e").unwrap();
        let action = Action::LinkIssue { issue_id: id };

        let json = serde_json::to_string(&action).expect("serialization failed");
        assert!(json.contains("\"type\":\"link.issue\""));

        let deserialized: Action = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_link_plan_serialization() {
        let id = ObjectId::from_str("d96f02665bf896324e2f0a4c18d08a768135ef2e").unwrap();
        let action = Action::LinkPlan { plan_id: id };

        let json = serde_json::to_string(&action).expect("serialization failed");
        assert!(json.contains("\"type\":\"link.plan\""));

        let deserialized: Action = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_produces_identifier() {
        let open = Action::Open {
            title: "test".to_string(),
            description: String::new(),
            approach: String::new(),
            constraints: vec![],
            learnings: LearningsSummary::default(),
            friction: vec![],
            open_items: vec![],
            files_touched: BTreeSet::new(),
            verification: vec![],
            task_id: None,
            embeds: vec![],
        };
        assert!(!open.produces_identifier());

        let link = Action::LinkCommit {
            sha: "abc".to_string(),
        };
        assert!(!link.produces_identifier());
    }

    #[test]
    fn test_open_action_backward_compat() {
        // Old JSON without verification/taskId should deserialize with defaults
        let json = r#"{"type":"open","title":"test"}"#;
        let action: Action = serde_json::from_str(json).expect("deserialization failed");
        match action {
            Action::Open {
                verification,
                task_id,
                ..
            } => {
                assert!(verification.is_empty());
                assert!(task_id.is_none());
            }
            _ => panic!("expected Open action"),
        }
    }
}
