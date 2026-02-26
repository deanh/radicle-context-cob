//! rad-context CLI tool for managing Context COBs.
//!
//! Usage:
//!   rad-context create <title> [--description <desc>] [--approach <approach>] [--json]
//!   rad-context list
//!   rad-context show <id> [--json]
//!   rad-context link <id> [--commit <sha>] [--issue <id>] [--patch <id>] [--plan <id>]
//!   rad-context unlink <id> [--commit <sha>] [--issue <id>] [--patch <id>] [--plan <id>]

use std::collections::BTreeSet;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Parser, Subcommand};
use serde::Deserialize;

use radicle::cob::{self, ObjectId, TypeName};
use radicle::prelude::ReadRepository;
use radicle::profile::Profile;
use radicle::rad;
use radicle::storage::git::Repository;
use radicle::storage::ReadStorage;

use radicle_context_cob::{Contexts, LearningsSummary, TYPENAME};

/// rad-context: Manage AI session context as Radicle COBs
#[derive(Parser)]
#[command(name = "rad-context")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the repository (defaults to current directory)
    #[arg(short, long, global = true)]
    repo: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new context
    Create {
        /// Context title
        title: Option<String>,

        /// Context description
        #[arg(short, long)]
        description: Option<String>,

        /// Approach taken
        #[arg(short, long)]
        approach: Option<String>,

        /// Constraints (can be specified multiple times)
        #[arg(long)]
        constraint: Vec<String>,

        /// Friction points (can be specified multiple times)
        #[arg(long)]
        friction: Vec<String>,

        /// Open items (can be specified multiple times)
        #[arg(long)]
        open_item: Vec<String>,

        /// Files touched (can be specified multiple times)
        #[arg(long)]
        file: Vec<String>,

        /// Read context as JSON from stdin
        #[arg(long)]
        json: bool,
    },

    /// List all contexts
    List,

    /// Show context details
    Show {
        /// Context ID (short form or full ID)
        id: String,

        /// Show in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Link a COB or commit to the context
    Link {
        /// Context ID
        id: String,

        /// Commit SHA to link
        #[arg(long)]
        commit: Option<String>,

        /// Issue ID to link
        #[arg(long)]
        issue: Option<String>,

        /// Patch ID to link
        #[arg(long)]
        patch: Option<String>,

        /// Plan ID to link
        #[arg(long)]
        plan: Option<String>,
    },

    /// Unlink a COB or commit from the context
    Unlink {
        /// Context ID
        id: String,

        /// Commit SHA to unlink
        #[arg(long)]
        commit: Option<String>,

        /// Issue ID to unlink
        #[arg(long)]
        issue: Option<String>,

        /// Patch ID to unlink
        #[arg(long)]
        patch: Option<String>,

        /// Plan ID to unlink
        #[arg(long)]
        plan: Option<String>,
    },
}

/// JSON input format for creating a context from stdin.
#[derive(Deserialize)]
struct JsonContextInput {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    approach: String,
    #[serde(default)]
    constraints: Vec<String>,
    #[serde(default)]
    learnings: LearningsSummary,
    #[serde(default)]
    friction: Vec<String>,
    #[serde(default)]
    open_items: Vec<String>,
    #[serde(default)]
    files_touched: BTreeSet<String>,
}

fn main() -> ExitCode {
    env_logger::init();

    let cli = Cli::parse();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let profile = Profile::load()?;

    let (_, rid) = if let Some(path) = cli.repo {
        rad::at(&path)?
    } else {
        rad::cwd()?
    };

    let repo = profile.storage.repository(rid)?;

    match cli.command {
        Commands::Create {
            title,
            description,
            approach,
            constraint,
            friction,
            open_item,
            file,
            json,
        } => {
            let mut contexts = Contexts::open(&repo)?;
            let signer = profile.signer()?;

            let (ctx_title, ctx_description, ctx_approach, ctx_constraints, ctx_learnings, ctx_friction, ctx_open_items, ctx_files) = if json {
                let mut input = String::new();
                std::io::stdin().read_to_string(&mut input)?;
                let parsed: JsonContextInput = serde_json::from_str(&input)?;
                (
                    parsed.title,
                    parsed.description,
                    parsed.approach,
                    parsed.constraints,
                    parsed.learnings,
                    parsed.friction,
                    parsed.open_items,
                    parsed.files_touched,
                )
            } else {
                let t = title.ok_or("title is required (provide as argument or use --json)")?;
                (
                    t,
                    description.unwrap_or_default(),
                    approach.unwrap_or_default(),
                    constraint,
                    LearningsSummary::default(),
                    friction,
                    open_item,
                    BTreeSet::from_iter(file),
                )
            };

            let (id, context) = contexts.create(
                ctx_title,
                ctx_description,
                ctx_approach,
                ctx_constraints,
                ctx_learnings,
                ctx_friction,
                ctx_open_items,
                ctx_files,
                vec![],
                &signer,
            )?;

            println!("Context created: {}", id);
            println!("  Title: {}", context.title());
        }
        Commands::List => {
            let contexts = Contexts::open(&repo)?;
            let mut count = 0;

            for result in contexts.all()? {
                let (id, context) = result?;
                count += 1;

                let commits = context.related_commits().len();
                let links = context.related_issues().len()
                    + context.related_patches().len()
                    + context.related_plans().len();

                print!("{} {}", short_id(&id), context.title());
                if commits > 0 || links > 0 {
                    print!(" [{}c {}l]", commits, links);
                }
                println!();
            }

            if count == 0 {
                println!("No contexts found.");
            } else {
                println!("\n{count} context(s)");
            }
        }
        Commands::Show { id, json } => {
            let contexts = Contexts::open(&repo)?;
            let context_id = resolve_cob_prefix(&id, &TYPENAME, &repo)?;

            let Some(context) = contexts.get(&context_id)? else {
                return Err(format!("Context not found: {id}").into());
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&context)?);
            } else {
                println!("# {}", context.title());
                println!();
                println!("ID: {}", context_id);
                println!("Author: {}", context.author());
                println!();

                if !context.description().is_empty() {
                    println!("## Description");
                    println!();
                    println!("{}", context.description());
                    println!();
                }

                if !context.approach().is_empty() {
                    println!("## Approach");
                    println!();
                    println!("{}", context.approach());
                    println!();
                }

                if !context.constraints().is_empty() {
                    println!("## Constraints");
                    println!();
                    for c in context.constraints() {
                        println!("- {c}");
                    }
                    println!();
                }

                let learnings = context.learnings();
                if !learnings.repo.is_empty() || !learnings.code.is_empty() {
                    println!("## Learnings");
                    println!();
                    if !learnings.repo.is_empty() {
                        println!("### Repository");
                        for l in &learnings.repo {
                            println!("- {l}");
                        }
                        println!();
                    }
                    if !learnings.code.is_empty() {
                        println!("### Code");
                        for cl in &learnings.code {
                            let location = match (cl.line, cl.end_line) {
                                (Some(start), Some(end)) => format!("{}:{}-{}", cl.path, start, end),
                                (Some(start), None) => format!("{}:{}", cl.path, start),
                                _ => cl.path.clone(),
                            };
                            println!("- **{}**: {}", location, cl.finding);
                        }
                        println!();
                    }
                }

                if !context.friction().is_empty() {
                    println!("## Friction");
                    println!();
                    for f in context.friction() {
                        println!("- {f}");
                    }
                    println!();
                }

                if !context.open_items().is_empty() {
                    println!("## Open Items");
                    println!();
                    for item in context.open_items() {
                        println!("- {item}");
                    }
                    println!();
                }

                if !context.files_touched().is_empty() {
                    println!("## Files Touched");
                    println!();
                    for f in context.files_touched() {
                        println!("- {f}");
                    }
                    println!();
                }

                if !context.related_commits().is_empty() {
                    println!("## Linked Commits");
                    for sha in context.related_commits() {
                        println!("  - {sha}");
                    }
                    println!();
                }

                if !context.related_issues().is_empty() {
                    println!("## Linked Issues");
                    for issue_id in context.related_issues() {
                        println!("  - {issue_id}");
                    }
                    println!();
                }

                if !context.related_patches().is_empty() {
                    println!("## Linked Patches");
                    for patch_id in context.related_patches() {
                        println!("  - {patch_id}");
                    }
                    println!();
                }

                if !context.related_plans().is_empty() {
                    println!("## Linked Plans");
                    for plan_id in context.related_plans() {
                        println!("  - {plan_id}");
                    }
                    println!();
                }
            }
        }
        Commands::Link { id, commit, issue, patch, plan } => {
            let issue_type = TypeName::from_str("xyz.radicle.issue")?;
            let patch_type = TypeName::from_str("xyz.radicle.patch")?;
            let plan_type = TypeName::from_str("me.hdh.plan")?;

            let mut contexts = Contexts::open(&repo)?;
            let context_id = resolve_cob_prefix(&id, &TYPENAME, &repo)?;
            let signer = profile.signer()?;

            let mut ctx = contexts.get_mut(&context_id)?;

            if let Some(sha) = commit {
                let full_sha = resolve_commit_sha(&sha, &repo)?;
                ctx.link_commit(full_sha.clone(), &signer)?;
                println!("Linked commit {} to context {}", &full_sha[..7], short_id(&context_id));
            }
            if let Some(i) = issue {
                let issue_id = resolve_cob_prefix(&i, &issue_type, &repo)?;
                ctx.link_issue(issue_id, &signer)?;
                println!("Linked issue {} to context {}", short_id(&issue_id), short_id(&context_id));
            }
            if let Some(p) = patch {
                let patch_id = resolve_cob_prefix(&p, &patch_type, &repo)?;
                ctx.link_patch(patch_id, &signer)?;
                println!("Linked patch {} to context {}", short_id(&patch_id), short_id(&context_id));
            }
            if let Some(pl) = plan {
                let plan_id = resolve_cob_prefix(&pl, &plan_type, &repo)?;
                ctx.link_plan(plan_id, &signer)?;
                println!("Linked plan {} to context {}", short_id(&plan_id), short_id(&context_id));
            }
        }
        Commands::Unlink { id, commit, issue, patch, plan } => {
            let issue_type = TypeName::from_str("xyz.radicle.issue")?;
            let patch_type = TypeName::from_str("xyz.radicle.patch")?;
            let plan_type = TypeName::from_str("me.hdh.plan")?;

            let mut contexts = Contexts::open(&repo)?;
            let context_id = resolve_cob_prefix(&id, &TYPENAME, &repo)?;
            let signer = profile.signer()?;

            let mut ctx = contexts.get_mut(&context_id)?;

            if let Some(sha) = commit {
                let full_sha = resolve_commit_sha(&sha, &repo)?;
                ctx.unlink_commit(full_sha.clone(), &signer)?;
                println!("Unlinked commit {} from context {}", &full_sha[..7], short_id(&context_id));
            }
            if let Some(i) = issue {
                let issue_id = resolve_cob_prefix(&i, &issue_type, &repo)?;
                ctx.unlink_issue(issue_id, &signer)?;
                println!("Unlinked issue {} from context {}", short_id(&issue_id), short_id(&context_id));
            }
            if let Some(p) = patch {
                let patch_id = resolve_cob_prefix(&p, &patch_type, &repo)?;
                ctx.unlink_patch(patch_id, &signer)?;
                println!("Unlinked patch {} from context {}", short_id(&patch_id), short_id(&context_id));
            }
            if let Some(pl) = plan {
                let plan_id = resolve_cob_prefix(&pl, &plan_type, &repo)?;
                ctx.unlink_plan(plan_id, &signer)?;
                println!("Unlinked plan {} from context {}", short_id(&plan_id), short_id(&context_id));
            }
        }
    }

    Ok(())
}

/// Minimum prefix length for COB ID resolution.
const MIN_PREFIX_LEN: usize = 7;

/// Validate and normalize a hex prefix string.
/// Returns `Ok(lowercase_prefix)` if valid, or an error describing the problem.
fn validate_hex_prefix(s: &str, label: &str) -> Result<String, Box<dyn std::error::Error>> {
    let prefix = s.to_lowercase();
    if !prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Invalid {label} '{s}': not a valid hex string").into());
    }
    if prefix.len() < MIN_PREFIX_LEN {
        return Err(format!(
            "{label} prefix '{s}' too short: need at least {MIN_PREFIX_LEN} characters"
        )
        .into());
    }
    Ok(prefix)
}

/// Resolve a COB ID from a full ID or short prefix by enumerating refs of the given type.
fn resolve_cob_prefix<R>(
    s: &str,
    type_name: &TypeName,
    repo: &R,
) -> Result<ObjectId, Box<dyn std::error::Error>>
where
    R: ReadRepository + cob::Store,
{
    // Try full ID first
    if let Ok(id) = ObjectId::from_str(s) {
        return Ok(id);
    }

    let prefix = validate_hex_prefix(s, "ID")?;

    // Enumerate all objects of this type and prefix-match
    let all = repo.types(type_name)?;
    let matches: Vec<ObjectId> = all
        .keys()
        .filter(|id| id.to_string().starts_with(&prefix))
        .copied()
        .collect();

    match matches.len() {
        0 => Err(format!("No {type_name} found matching prefix '{s}'").into()),
        1 => Ok(matches[0]),
        n => {
            let ids: Vec<String> = matches.iter().map(|id| short_id(id)).collect();
            Err(format!(
                "Ambiguous {type_name} ID prefix '{s}': {n} objects match ({})",
                ids.join(", ")
            )
            .into())
        }
    }
}

/// Resolve a commit SHA prefix to a full SHA using git object lookup.
fn resolve_commit_sha(s: &str, repo: &Repository) -> Result<String, Box<dyn std::error::Error>> {
    // If it looks like a full SHA already, pass it through
    if s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(s.to_string());
    }

    let prefix = validate_hex_prefix(s, "commit SHA")?;

    // Use git2's revparse to resolve the prefix
    let object = repo
        .backend
        .revparse_single(&prefix)
        .map_err(|_| format!("No commit found matching prefix '{s}'"))?;

    Ok(object.id().to_string())
}

/// Get a short form of an object ID.
fn short_id(id: &ObjectId) -> String {
    let s = id.to_string();
    s[..7.min(s.len())].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_hex_prefix_valid() {
        let result = validate_hex_prefix("abc1234", "ID");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "abc1234");
    }

    #[test]
    fn test_validate_hex_prefix_uppercase_normalized() {
        let result = validate_hex_prefix("ABC1234", "ID");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "abc1234");
    }

    #[test]
    fn test_validate_hex_prefix_too_short() {
        let result = validate_hex_prefix("abc12", "ID");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_validate_hex_prefix_exactly_min_length() {
        let result = validate_hex_prefix("abcdef1", "ID");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_hex_prefix_non_hex() {
        let result = validate_hex_prefix("xyz1234", "ID");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a valid hex"));
    }

    #[test]
    fn test_validate_hex_prefix_empty() {
        let result = validate_hex_prefix("", "ID");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_hex_prefix_label_in_error() {
        let result = validate_hex_prefix("xyz", "commit SHA");
        assert!(result.unwrap_err().to_string().contains("commit SHA"));
    }
}
