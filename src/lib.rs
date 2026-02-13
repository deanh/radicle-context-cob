//! # radicle-context-cob
//!
//! A Radicle Collaborative Object (COB) type for storing AI session context.
//!
//! Contexts are observation records from development sessions — what was tried,
//! what was learned, what constraints exist, and what went wrong. They complement
//! Plan COBs: Plans capture *coordination* (what should be done), Contexts capture
//! *observation* (how the work was done).
//!
//! ## Type Name
//!
//! The COB type name is `me.hdh.context` following the reverse domain notation pattern.

#![warn(clippy::unwrap_used)]
#![warn(missing_docs)]

pub mod actions;
pub mod state;

use std::collections::BTreeSet;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::LazyLock;

use thiserror::Error;

use radicle::cob;
use radicle::cob::common::{Authorization, Timestamp, Uri};
use radicle::cob::store::Cob;
use radicle::cob::{op, store, ActorId, Embed, EntryId, ObjectId, TypeName};
use radicle::crypto;
use radicle::identity::doc::DocError;
use radicle::node::device::Device;
use radicle::node::NodeId;
use radicle::prelude::{Doc, ReadRepository, RepoId};
use radicle::storage::{HasRepoId, RepositoryError, SignRepository, WriteRepository};

pub use actions::Action;
pub use state::{CodeLearning, Context, LearningsSummary};

/// Context operation.
pub type Op = cob::Op<Action>;

/// Type name of a context COB.
pub static TYPENAME: LazyLock<TypeName> =
    LazyLock::new(|| FromStr::from_str("me.hdh.context").expect("type name is valid"));

/// Identifier for a context.
pub type ContextId = ObjectId;

/// Error updating or creating contexts.
#[derive(Error, Debug)]
pub enum Error {
    /// Error loading the identity document.
    #[error("identity doc failed to load: {0}")]
    Doc(#[from] DocError),
    /// Store error.
    #[error("store: {0}")]
    Store(#[from] store::Error),
    /// Action not authorized.
    #[error("{0} not authorized to apply {1:?}")]
    NotAuthorized(ActorId, Action),
    /// Identity document is missing.
    #[error("identity document missing")]
    MissingIdentity,
    /// General error initializing a context.
    #[error("initialization failed: {0}")]
    Init(&'static str),
    /// Error decoding an operation.
    #[error("op decoding failed: {0}")]
    Op(#[from] op::OpEncodingError),
}

impl cob::store::CobWithType for Context {
    fn type_name() -> &'static TypeName {
        &TYPENAME
    }
}

impl store::Cob for Context {
    type Action = Action;
    type Error = Error;

    fn from_root<R: ReadRepository>(op: Op, repo: &R) -> Result<Self, Self::Error> {
        let doc = op.identity_doc(repo)?.ok_or(Error::MissingIdentity)?;
        let mut actions = op.actions.into_iter();

        // The first action must be Open
        let Some(Action::Open {
            title,
            description,
            approach,
            constraints,
            learnings,
            friction,
            open_items,
            files_touched,
            embeds: _,
        }) = actions.next()
        else {
            return Err(Error::Init("the first action must be of type `Open`"));
        };

        let mut context = Context::new(
            title,
            description,
            approach,
            constraints,
            learnings,
            friction,
            open_items,
            files_touched,
            op.author.into(),
            op.timestamp,
        );

        for action in actions {
            match context.authorization(&action, &op.author, &doc)? {
                Authorization::Allow => {
                    context.apply_action(action, op.timestamp);
                }
                Authorization::Deny => {
                    return Err(Error::NotAuthorized(op.author, action));
                }
                Authorization::Unknown => {
                    continue;
                }
            }
        }
        Ok(context)
    }

    fn op<'a, R: ReadRepository, I: IntoIterator<Item = &'a cob::Entry>>(
        &mut self,
        op: Op,
        concurrent: I,
        repo: &R,
    ) -> Result<(), Error> {
        let doc = op.identity_doc(repo)?.ok_or(Error::MissingIdentity)?;
        let _concurrent = concurrent.into_iter().collect::<Vec<_>>();

        for action in op.actions {
            log::trace!(target: "context", "Applying {} {action:?}", op.id);

            match self.authorization(&action, &op.author, &doc)? {
                Authorization::Allow => {
                    self.apply_action(action, op.timestamp);
                }
                Authorization::Deny => {
                    return Err(Error::NotAuthorized(op.author, action));
                }
                Authorization::Unknown => {
                    continue;
                }
            }
        }
        Ok(())
    }
}

impl<R: ReadRepository> cob::Evaluate<R> for Context {
    type Error = Error;

    fn init(entry: &cob::Entry, repo: &R) -> Result<Self, Self::Error> {
        let op = Op::try_from(entry)?;
        let object = Context::from_root(op, repo)?;
        Ok(object)
    }

    fn apply<'a, I: Iterator<Item = (&'a EntryId, &'a cob::Entry)>>(
        &mut self,
        entry: &cob::Entry,
        concurrent: I,
        repo: &R,
    ) -> Result<(), Self::Error> {
        let op = Op::try_from(entry)?;
        self.op(op, concurrent.map(|(_, e)| e), repo)
    }
}

impl Context {
    /// Apply a single action to the context.
    fn apply_action(&mut self, action: Action, _timestamp: Timestamp) {
        match action {
            Action::Open {
                title,
                description,
                approach,
                constraints,
                learnings,
                friction,
                open_items,
                files_touched,
                ..
            } => {
                self.title = title;
                self.description = description;
                self.approach = approach;
                self.constraints = constraints;
                self.learnings = learnings;
                self.friction = friction;
                self.open_items = open_items;
                self.files_touched = files_touched;
            }
            Action::LinkCommit { sha } => {
                self.related_commits.insert(sha);
            }
            Action::UnlinkCommit { sha } => {
                self.related_commits.remove(&sha);
            }
            Action::LinkIssue { issue_id } => {
                self.related_issues.insert(issue_id);
            }
            Action::UnlinkIssue { issue_id } => {
                self.related_issues.remove(&issue_id);
            }
            Action::LinkPatch { patch_id } => {
                self.related_patches.insert(patch_id);
            }
            Action::UnlinkPatch { patch_id } => {
                self.related_patches.remove(&patch_id);
            }
            Action::LinkPlan { plan_id } => {
                self.related_plans.insert(plan_id);
            }
            Action::UnlinkPlan { plan_id } => {
                self.related_plans.remove(&plan_id);
            }
        }
    }

    /// Apply authorization rules on context actions.
    pub fn authorization(
        &self,
        action: &Action,
        actor: &ActorId,
        doc: &Doc,
    ) -> Result<Authorization, Error> {
        if doc.is_delegate(&actor.into()) {
            // A delegate is authorized to do all actions.
            return Ok(Authorization::Allow);
        }
        let author: ActorId = *self.author.id().as_key();
        let outcome = match action {
            // Context author can perform link/unlink operations on their own contexts.
            Action::Open { .. }
            | Action::LinkCommit { .. }
            | Action::UnlinkCommit { .. }
            | Action::LinkIssue { .. }
            | Action::UnlinkIssue { .. }
            | Action::LinkPatch { .. }
            | Action::UnlinkPatch { .. }
            | Action::LinkPlan { .. }
            | Action::UnlinkPlan { .. } => Authorization::from(*actor == author),
        };
        Ok(outcome)
    }
}

/// Contexts store for a repository.
pub struct Contexts<'a, R> {
    raw: store::Store<'a, Context, R>,
}

impl<'a, R> Deref for Contexts<'a, R> {
    type Target = store::Store<'a, Context, R>;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl<R> HasRepoId for Contexts<'_, R>
where
    R: ReadRepository,
{
    fn rid(&self) -> RepoId {
        self.raw.as_ref().id()
    }
}

impl<'a, R> Contexts<'a, R>
where
    R: ReadRepository + cob::Store<Namespace = NodeId>,
{
    /// Open a contexts store.
    pub fn open(repository: &'a R) -> Result<Self, RepositoryError> {
        let identity = repository.identity_head()?;
        let raw = store::Store::open(repository)?.identity(identity);
        Ok(Self { raw })
    }
}

impl<'a, R> Contexts<'a, R>
where
    R: WriteRepository + cob::Store<Namespace = NodeId>,
{
    /// Create a new context.
    #[allow(clippy::too_many_arguments)]
    pub fn create<G>(
        &mut self,
        title: String,
        description: String,
        approach: String,
        constraints: Vec<String>,
        learnings: LearningsSummary,
        friction: Vec<String>,
        open_items: Vec<String>,
        files_touched: BTreeSet<String>,
        embeds: Vec<Embed<Uri>>,
        signer: &Device<G>,
    ) -> Result<(ObjectId, Context), Error>
    where
        G: crypto::signature::Signer<crypto::Signature>,
    {
        use nonempty::NonEmpty;

        let action = Action::Open {
            title,
            description,
            approach,
            constraints,
            learnings,
            friction,
            open_items,
            files_touched,
            embeds: embeds.clone(),
        };
        let actions = NonEmpty::new(action);

        self.raw
            .create("Create context", actions, embeds, signer)
            .map_err(Error::from)
    }
}

impl<R> Contexts<'_, R>
where
    R: ReadRepository + cob::Store,
{
    /// Get a context.
    pub fn get(&self, id: &ObjectId) -> Result<Option<Context>, store::Error> {
        self.raw.get(id)
    }
}

impl<'a, R> Contexts<'a, R>
where
    R: WriteRepository + SignRepository + cob::Store<Namespace = NodeId>,
{
    /// Get a context for mutation.
    pub fn get_mut<'g>(
        &'g mut self,
        id: &ObjectId,
    ) -> Result<ContextMut<'a, 'g, R>, store::Error> {
        let context = self
            .raw
            .get(id)?
            .ok_or_else(move || store::Error::NotFound(TYPENAME.clone(), *id))?;

        Ok(ContextMut {
            id: *id,
            context,
            store: self,
        })
    }
}

/// A mutable context handle for performing updates.
pub struct ContextMut<'a, 'g, R> {
    id: ObjectId,
    context: Context,
    store: &'g mut Contexts<'a, R>,
}

impl<R> std::fmt::Debug for ContextMut<'_, '_, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("ContextMut")
            .field("id", &self.id)
            .field("context", &self.context)
            .finish()
    }
}

impl<R> std::ops::Deref for ContextMut<'_, '_, R> {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl<'a, 'g, R> ContextMut<'a, 'g, R>
where
    R: WriteRepository + SignRepository + cob::Store<Namespace = NodeId>,
{
    /// Get the context ID.
    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    /// Run a transaction on the context.
    fn transaction<G, F>(
        &mut self,
        message: &str,
        signer: &Device<G>,
        operations: F,
    ) -> Result<EntryId, Error>
    where
        G: crypto::signature::Signer<crypto::Signature>,
        F: FnOnce(&mut store::Transaction<Context, R>) -> Result<(), store::Error>,
    {
        let mut tx = store::Transaction::default();
        operations(&mut tx)?;

        let (context, commit) = tx.commit(message, self.id, &mut self.store.raw, signer)?;
        self.context = context;

        Ok(commit)
    }

    /// Link a git commit to the context.
    pub fn link_commit<G>(&mut self, sha: String, signer: &Device<G>) -> Result<EntryId, Error>
    where
        G: crypto::signature::Signer<crypto::Signature>,
    {
        self.transaction("Link commit", signer, |tx| {
            tx.push(Action::LinkCommit { sha })
        })
    }

    /// Unlink a git commit from the context.
    pub fn unlink_commit<G>(&mut self, sha: String, signer: &Device<G>) -> Result<EntryId, Error>
    where
        G: crypto::signature::Signer<crypto::Signature>,
    {
        self.transaction("Unlink commit", signer, |tx| {
            tx.push(Action::UnlinkCommit { sha })
        })
    }

    /// Link an issue to the context.
    pub fn link_issue<G>(
        &mut self,
        issue_id: ObjectId,
        signer: &Device<G>,
    ) -> Result<EntryId, Error>
    where
        G: crypto::signature::Signer<crypto::Signature>,
    {
        self.transaction("Link issue", signer, |tx| {
            tx.push(Action::LinkIssue { issue_id })
        })
    }

    /// Unlink an issue from the context.
    pub fn unlink_issue<G>(
        &mut self,
        issue_id: ObjectId,
        signer: &Device<G>,
    ) -> Result<EntryId, Error>
    where
        G: crypto::signature::Signer<crypto::Signature>,
    {
        self.transaction("Unlink issue", signer, |tx| {
            tx.push(Action::UnlinkIssue { issue_id })
        })
    }

    /// Link a patch to the context.
    pub fn link_patch<G>(
        &mut self,
        patch_id: ObjectId,
        signer: &Device<G>,
    ) -> Result<EntryId, Error>
    where
        G: crypto::signature::Signer<crypto::Signature>,
    {
        self.transaction("Link patch", signer, |tx| {
            tx.push(Action::LinkPatch { patch_id })
        })
    }

    /// Unlink a patch from the context.
    pub fn unlink_patch<G>(
        &mut self,
        patch_id: ObjectId,
        signer: &Device<G>,
    ) -> Result<EntryId, Error>
    where
        G: crypto::signature::Signer<crypto::Signature>,
    {
        self.transaction("Unlink patch", signer, |tx| {
            tx.push(Action::UnlinkPatch { patch_id })
        })
    }

    /// Link a plan to the context.
    pub fn link_plan<G>(
        &mut self,
        plan_id: ObjectId,
        signer: &Device<G>,
    ) -> Result<EntryId, Error>
    where
        G: crypto::signature::Signer<crypto::Signature>,
    {
        self.transaction("Link plan", signer, |tx| {
            tx.push(Action::LinkPlan { plan_id })
        })
    }

    /// Unlink a plan from the context.
    pub fn unlink_plan<G>(
        &mut self,
        plan_id: ObjectId,
        signer: &Device<G>,
    ) -> Result<EntryId, Error>
    where
        G: crypto::signature::Signer<crypto::Signature>,
    {
        self.transaction("Unlink plan", signer, |tx| {
            tx.push(Action::UnlinkPlan { plan_id })
        })
    }
}
