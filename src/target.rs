//! Runtime Targets: the built-in registry rows plus anything the user's config
//! adds, filtered down to what is actually installed and enabled.
//!
//! The registry stays pure static data; this is where it meets the machine and
//! the config, so every command iterates the same resolved list.

use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::env::Env;
use crate::family::Family;
use crate::registry::{self, Agent, Instructions};

/// One Target this invocation will act on.
#[derive(Debug, Clone)]
pub struct Target {
    pub name: String,
    pub root: String,
    /// The registry or config root this Target's paths are written against,
    /// before any alternate root is applied.
    base_root: String,
    /// When set, every path of this agent is placed under this root.
    root_override: Option<PathBuf>,
    /// Home-relative fan-out directory per family.
    fanout: Vec<(Family, String)>,
    /// The registry row, when this is a built-in agent. Config-defined targets
    /// have none, and therefore take fan-out families only.
    pub agent: Option<&'static Agent>,
    /// The instructions route this Target takes — the user's choice, else the
    /// row's default — resolved once so every command reads the same answer.
    /// Read this, never the registry row. `None`: no instructions surface.
    pub instructions: Option<Instructions>,
}

impl Target {
    fn from_agent(agent: &'static Agent, config: &Config) -> Self {
        let fanout = Family::ALL
            .iter()
            .filter_map(|family| {
                agent
                    .fanout_dir(*family)
                    .map(|dir| (*family, dir.to_string()))
            })
            .collect();

        Self {
            name: agent.name.to_string(),
            root: agent.root.to_string(),
            base_root: agent.root.to_string(),
            root_override: None,
            fanout,
            agent: Some(agent),
            instructions: config
                .route(agent.name)
                .and_then(|route| agent.instructions.iter().find(|i| i.route() == route))
                .or(agent.instructions.first())
                .copied(),
        }
    }

    /// The same Target, with every path rebased under an alternate config root.
    pub fn with_root(agent: &'static Agent, config: &Config, root: PathBuf) -> Self {
        Self {
            root_override: Some(root),
            ..Self::from_agent(agent, config)
        }
    }

    /// How reports name this Target. A profile root is part of the label so two
    /// profiles of the same agent are distinguishable.
    pub fn label(&self, env: &Env) -> String {
        match &self.root_override {
            Some(root) => format!("{} ({})", self.name, Self::display_path(env, root)),
            None => self.name.clone(),
        }
    }

    /// Resolve one registry path for this Target. Without an alternate root
    /// this is the old home-relative lookup. With one, a path under the agent's
    /// own root is rebased; every other path is still profile-local, so even
    /// Claude's `.claude.json` lands under the alternate root.
    pub fn path(&self, env: &Env, rel: &str) -> PathBuf {
        let Some(root) = &self.root_override else {
            return env.in_home(rel);
        };
        let rel = Path::new(rel);
        match rel.strip_prefix(Path::new(&self.base_root)) {
            Ok(suffix) => root.join(suffix),
            Err(_) => root.join(rel),
        }
    }

    /// A path for a report, relative to home when it lives there.
    pub fn display_path(env: &Env, path: &Path) -> String {
        path.strip_prefix(env.home())
            .unwrap_or(path)
            .display()
            .to_string()
    }

    /// The home-relative fan-out directory for a family, if this Target takes one.
    pub fn fanout_dir(&self, family: Family) -> Option<&str> {
        self.fanout
            .iter()
            .find(|(f, _)| *f == family)
            .map(|(_, dir)| dir.as_str())
    }

    /// The fan-out directory this Target once took for a family and still
    /// reads beside the Commons — sync visits it in prune-only mode. Only
    /// registry rows carry one; config-defined targets never do.
    pub fn legacy_dir(&self, family: Family) -> Option<&'static str> {
        self.agent.and_then(|agent| agent.legacy_dir(family))
    }

    pub fn detected(&self, home: &Path) -> bool {
        match &self.root_override {
            Some(root) => root.is_dir(),
            None => home.join(&self.root).is_dir(),
        }
    }

    /// Capability summary for reports.
    pub fn capabilities(&self) -> Vec<(&'static str, String)> {
        match self.agent {
            Some(agent) => agent.capabilities(self.instructions),
            None => self
                .fanout
                .iter()
                .map(|(family, dir)| (family.name(), format!("fan-out → {dir}")))
                .collect(),
        }
    }
}

/// Every Target that is installed and not switched off, built-ins first.
pub fn resolve(env: &Env, config: &Config) -> Vec<Target> {
    let mut targets: Vec<Target> = registry::AGENTS
        .iter()
        .filter(|agent| !config.is_disabled(agent.name))
        .map(|agent| Target::from_agent(agent, config))
        .collect();

    for custom in config.custom() {
        // `targets.<name> = false` switches off a config-defined agent the same
        // way it does a built-in one — revert's refusal prints that line, so it
        // has to mean what it says for every target.
        if config.is_disabled(&custom.name) {
            continue;
        }
        targets.push(Target {
            name: custom.name.clone(),
            root: custom.root.clone(),
            base_root: custom.root.clone(),
            root_override: None,
            fanout: custom.fanout.iter().map(|(f, d)| (*f, d.clone())).collect(),
            agent: None,
            instructions: None,
        });
    }

    targets.retain(|t| t.detected(env.home()));
    targets
}

/// The one Target with this name — a registry row or a config-defined agent —
/// found without the disabled filter and without detection. Revert is the only
/// caller: it exists to reach the agents [`resolve`] deliberately drops.
pub fn find(config: &Config, name: &str) -> Option<Target> {
    if let Some(agent) = registry::by_name(name) {
        return Some(Target::from_agent(agent, config));
    }
    config
        .custom()
        .iter()
        .find(|c| c.name == name)
        .map(|custom| Target {
            name: custom.name.clone(),
            root: custom.root.clone(),
            base_root: custom.root.clone(),
            root_override: None,
            fanout: custom.fanout.iter().map(|(f, d)| (*f, d.clone())).collect(),
            agent: None,
            instructions: None,
        })
}
