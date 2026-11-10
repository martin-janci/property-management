use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseState {
    Candidate,
    Staging,
    Prod,
    Previous,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Staging,
    Prod,
}

impl TargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TargetKind::Staging => "staging",
            TargetKind::Prod => "prod",
        }
    }

    /// String form of the live ReleaseState for this target.
    /// Released to this target = its own name (per existing schema invariant).
    pub fn live_release_state_str(self) -> &'static str {
        self.as_str()
    }

    pub fn live_release_state(self) -> ReleaseState {
        match self {
            TargetKind::Staging => ReleaseState::Staging,
            TargetKind::Prod => ReleaseState::Prod,
        }
    }
}

impl std::fmt::Display for TargetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for TargetKind {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s {
            "staging" => Ok(TargetKind::Staging),
            "prod" => Ok(TargetKind::Prod),
            other => Err(format!("unknown target: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub tag: String,
    /// Map of service name (e.g. "api-server") -> image ref.
    pub images: HashMap<String, String>,
    pub state: ReleaseState,
    pub target: Option<String>,
    pub promoted_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

/// The OCI label every image built by this repo's pipeline carries (T6).
/// `docker/{backend,frontend}/*` set it from `--build-arg GIT_SHA=${{ github.sha }}`,
/// and `docker-{backend,frontend}-images.yml` fails the build when the pushed
/// manifest does not carry the matching version label, so for anything the
/// pipeline published this label is the commit the image was built from.
pub const REVISION_LABEL: &str = "org.opencontainers.image.revision";

/// One service of a deploy request, paired with the
/// `org.opencontainers.image.revision` read off the image it names.
/// `revision` is `None` when the image carries no such label — a pre-T6 image,
/// or one built outside the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRevision {
    pub service: String,
    pub image: String,
    pub revision: Option<String>,
}

impl ServiceRevision {
    pub fn new(
        service: impl Into<String>,
        image: impl Into<String>,
        revision: Option<String>,
    ) -> Self {
        Self {
            service: service.into(),
            image: image.into(),
            revision: revision.filter(|r| !r.trim().is_empty()),
        }
    }
}

/// Verdict of [`check_revisions`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevisionCheck {
    /// Nothing to compare (no services).
    Empty,
    /// Every image carries the same non-empty revision.
    Uniform(String),
    /// No image carries the label at all. A whole release predating T6 —
    /// unverifiable, but internally consistent, so the deployer warns and
    /// proceeds rather than removing the rollback path to those releases.
    Unlabelled,
    /// The services do not agree on one commit. This is the F-P8 shape: a June
    /// frontend deployed alongside a September backend, which shipped for three
    /// months because nothing compared the two. Groups are `(revision,
    /// services)` sorted by revision, with the missing label rendered as
    /// `<unlabelled>`; a mix of labelled and unlabelled images counts as
    /// divergent, because an unlabelled image cannot be shown to be the same
    /// build as a labelled one.
    Divergent(Vec<(String, Vec<String>)>),
}

/// Rendered as the label value for an image that carries no revision label.
pub const UNLABELLED: &str = "<unlabelled>";

impl RevisionCheck {
    /// Human-readable rejection reason, naming every divergent service.
    /// `None` when the check passed.
    pub fn rejection(&self) -> Option<String> {
        match self {
            RevisionCheck::Empty | RevisionCheck::Uniform(_) | RevisionCheck::Unlabelled => None,
            RevisionCheck::Divergent(groups) => {
                let detail = groups
                    .iter()
                    .map(|(rev, services)| format!("{rev}: {}", services.join(", ")))
                    .collect::<Vec<_>>()
                    .join(" | ");
                Some(format!(
                    "mixed-revision deploy refused: the service images do not share one \
                     {REVISION_LABEL} ({detail}). Every service must come from the same \
                     commit; rebuild the lagging image(s) and retry."
                ))
            }
        }
    }
}

/// Group the services of a deploy request by the commit their image was built
/// from. See [`RevisionCheck`] for what each outcome means.
pub fn check_revisions(entries: &[ServiceRevision]) -> RevisionCheck {
    if entries.is_empty() {
        return RevisionCheck::Empty;
    }
    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for e in entries {
        groups
            .entry(e.revision.clone().unwrap_or_else(|| UNLABELLED.to_string()))
            .or_default()
            .push(e.service.clone());
    }
    for services in groups.values_mut() {
        services.sort();
    }
    if groups.len() > 1 {
        return RevisionCheck::Divergent(groups.into_iter().collect());
    }
    let (rev, _) = groups.into_iter().next().expect("non-empty");
    if rev == UNLABELLED {
        RevisionCheck::Unlabelled
    } else {
        RevisionCheck::Uniform(rev)
    }
}

#[cfg(test)]
mod revision_tests {
    use super::*;

    fn sr(service: &str, rev: Option<&str>) -> ServiceRevision {
        ServiceRevision::new(
            service,
            format!("ghcr.io/test/ppt-{service}:dev"),
            rev.map(Into::into),
        )
    }

    #[test]
    fn one_shared_revision_is_accepted() {
        let got = check_revisions(&[
            sr("api-server", Some("abc123")),
            sr("reality-server", Some("abc123")),
            sr("ppt-web", Some("abc123")),
            sr("reality-web", Some("abc123")),
            sr("admin-web", Some("abc123")),
        ]);
        assert_eq!(got, RevisionCheck::Uniform("abc123".into()));
        assert!(got.rejection().is_none());
    }

    #[test]
    fn divergent_revisions_are_refused_and_name_the_services() {
        // The literal F-P8 shape: frontend images from June, backend from
        // September, deployed together under one `:dev` tag.
        let got = check_revisions(&[
            sr("api-server", Some("sep2026")),
            sr("reality-server", Some("sep2026")),
            sr("ppt-web", Some("jun2026")),
            sr("reality-web", Some("jun2026")),
            sr("admin-web", Some("jun2026")),
        ]);
        let msg = got.rejection().expect("must be refused");
        assert!(msg.contains("mixed-revision deploy refused"), "{msg}");
        assert!(
            msg.contains("jun2026: admin-web, ppt-web, reality-web"),
            "{msg}"
        );
        assert!(msg.contains("sep2026: api-server, reality-server"), "{msg}");
    }

    #[test]
    fn a_single_unlabelled_image_among_labelled_ones_is_refused() {
        // A stale image that predates T6 alongside fresh ones cannot be shown
        // to be the same build, so it must not ride along silently.
        let got = check_revisions(&[sr("api-server", Some("abc123")), sr("ppt-web", None)]);
        let msg = got.rejection().expect("must be refused");
        assert!(msg.contains("<unlabelled>: ppt-web"), "{msg}");
        assert!(msg.contains("abc123: api-server"), "{msg}");
    }

    #[test]
    fn an_all_unlabelled_release_is_allowed_so_rollback_keeps_working() {
        // Every pre-T6 Release row looks like this. Refusing it would take away
        // the rollback path to those releases, which is worse than the risk it
        // carries — the deployer logs a warning instead.
        let got = check_revisions(&[sr("api-server", None), sr("ppt-web", None)]);
        assert_eq!(got, RevisionCheck::Unlabelled);
        assert!(got.rejection().is_none());
    }

    #[test]
    fn blank_label_counts_as_missing() {
        // `LABEL org.opencontainers.image.revision="${GIT_SHA}"` with an unset
        // build arg yields an empty string, not an absent label.
        let got = check_revisions(&[
            ServiceRevision::new("api-server", "img", Some("   ".into())),
            ServiceRevision::new("ppt-web", "img", None),
        ]);
        assert_eq!(got, RevisionCheck::Unlabelled);
    }

    #[test]
    fn empty_request_is_a_no_op() {
        assert_eq!(check_revisions(&[]), RevisionCheck::Empty);
    }
}
