//! The scope one `sync` invocation was asked for: which Targets, which
//! surfaces, and which named items. An empty selection is the whole machine.
//!
//! Parsing is split from resolution so syntax errors and unknown names are
//! reported before the lock, and before any survey can touch the filesystem.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::cli::SyncArgs;
use crate::commons::{self, Commons, Entry};
use crate::config::Config;
use crate::env::Env;
use crate::family::Family;
use crate::hooks;
use crate::link;
use crate::mcp;
use crate::target::{self, Target};

/// A surface `sync` can be narrowed to. `Commands` covers both the fan-out
/// commands family and the rendered-commands family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Surface {
    Skills,
    Commands,
    Agents,
    Instructions,
    Mcp,
    Hooks,
}

impl Surface {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "skills" => Some(Self::Skills),
            "commands" => Some(Self::Commands),
            "agents" => Some(Self::Agents),
            "instructions" => Some(Self::Instructions),
            "mcp" => Some(Self::Mcp),
            "hooks" => Some(Self::Hooks),
            _ => None,
        }
    }

    /// The surface a link family belongs to.
    pub fn for_family(family: Family) -> Self {
        match family {
            Family::Skills => Self::Skills,
            Family::Commands => Self::Commands,
            Family::Agents => Self::Agents,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Skills => "skills",
            Self::Commands => "commands",
            Self::Agents => "agents",
            Self::Instructions => "instructions",
            Self::Mcp => "mcp",
            Self::Hooks => "hooks",
        }
    }
}

/// One invocation's narrowed scope.
#[derive(Debug, Clone)]
pub struct Selection {
    /// Empty means every resolved Target.
    agents: Vec<String>,
    /// Empty means each selected Target's own root. Only legal with one agent.
    roots: Vec<PathBuf>,
    /// Empty means every surface. `None` means every item of that surface.
    scope: BTreeMap<Surface, Option<BTreeSet<String>>>,
    pub strict: bool,
}

impl Selection {
    /// Parse the CLI flags and resolve `--root` paths against the home.
    pub fn parse(args: &SyncArgs, env: &Env) -> Result<Self, String> {
        let mut agents: Vec<String> = Vec::new();
        for agent in &args.agent {
            if !agents.contains(agent) {
                agents.push(agent.clone());
            }
        }

        if !args.root.is_empty() && agents.len() != 1 {
            return Err(format!(
                "--root requires exactly one --agent (got {})",
                agents.len()
            ));
        }

        let mut roots: Vec<PathBuf> = Vec::new();
        for root in &args.root {
            let root = expand_root(root, env.home());
            if !roots.contains(&root) {
                roots.push(root);
            }
        }

        let mut scope = BTreeMap::new();
        for value in &args.only {
            parse_only(value, &mut scope)?;
        }

        Ok(Self {
            agents,
            roots,
            scope,
            strict: args.strict,
        })
    }

    /// The unscoped selection: every Target, every surface, every item.
    pub fn all() -> Self {
        Self {
            agents: Vec::new(),
            roots: Vec::new(),
            scope: BTreeMap::new(),
            strict: false,
        }
    }

    pub fn is_selective(&self) -> bool {
        !self.agents.is_empty() || !self.roots.is_empty() || !self.scope.is_empty()
    }

    /// Whether a surface is in scope at all.
    pub fn allows(&self, surface: Surface) -> bool {
        self.scope.is_empty() || self.scope.contains_key(&surface)
    }

    /// The item set for a surface. `None` means the whole surface; callers must
    /// check [`Self::allows`] first.
    pub fn items(&self, surface: Surface) -> Option<&BTreeSet<String>> {
        self.scope.get(&surface).and_then(Option::as_ref)
    }

    pub fn allows_family(&self, family: Family) -> bool {
        self.allows(Surface::for_family(family))
    }

    /// Resolve the selected Targets, refusing explicit names the machine cannot
    /// satisfy. Errors are collected so one run names every bad selector.
    pub fn resolve_targets(&self, env: &Env, config: &Config) -> Result<Vec<Target>, Vec<String>> {
        if self.agents.is_empty() {
            return Ok(target::resolve(env, config));
        }

        let mut errors = Vec::new();
        let mut targets = Vec::new();

        for name in &self.agents {
            let Some(base) = target::find(config, name) else {
                errors.push(format!("no Target named `{name}`"));
                continue;
            };
            if config.is_disabled(name) {
                errors.push(format!(
                    "Target `{name}` is disabled in agent-sync.toml — remove `targets.{name} = false` to sync it"
                ));
                continue;
            }

            if self.roots.is_empty() {
                if !base.detected(env.home()) {
                    errors.push(format!(
                        "Target `{name}` is not installed at {}",
                        env.home().join(&base.root).display()
                    ));
                    continue;
                }
                targets.push(base);
                continue;
            }

            let Some(agent) = base.agent else {
                errors.push(format!(
                    "--root is not supported for custom Target `{name}` — set its root in [custom.{name}]"
                ));
                continue;
            };
            for root in &self.roots {
                if !root.is_dir() {
                    errors.push(format!("--root {} is not a directory", root.display()));
                    continue;
                }
                targets.push(Target::with_root(agent, config, root.clone()));
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }
        if targets.is_empty() {
            return Err(vec!["no Target matched the selection".into()]);
        }
        Ok(targets)
    }

    /// Every detected Target, with an explicitly selected agent's default-root
    /// Target replaced by the profile Targets. Used by `--strict` to audit the
    /// whole machine while writing only the selection.
    pub fn audit_targets(&self, env: &Env, config: &Config) -> Result<Vec<Target>, Vec<String>> {
        let all = target::resolve(env, config);
        if self.agents.is_empty() {
            return Ok(all);
        }

        let selected = self.resolve_targets(env, config)?;
        let mut targets: Vec<Target> = all
            .into_iter()
            .filter(|target| !self.agents.contains(&target.name))
            .collect();
        targets.extend(selected);
        Ok(targets)
    }

    /// Check every named item against the Commons. Returns every unknown name
    /// so one run names them all.
    pub fn validate_items(&self, commons: &Commons) -> Vec<String> {
        let mut errors = Vec::new();

        for (surface, items) in &self.scope {
            let Some(items) = items else {
                continue;
            };
            let known = match surface {
                Surface::Skills => family_names(commons, Family::Skills),
                Surface::Commands => family_names(commons, Family::Commands),
                Surface::Agents => family_names(commons, Family::Agents),
                Surface::Instructions => {
                    errors.push("--only instructions takes no items".into());
                    continue;
                }
                Surface::Mcp => {
                    let Some(names) = mcp_names(commons) else {
                        // A malformed Commons file is the survey's problem to
                        // report, not an unknown item.
                        continue;
                    };
                    names
                }
                Surface::Hooks => hooks::declared_events(&commons.family_dir(commons::HOOKS)),
            };

            let noun = match surface {
                Surface::Skills => "skills entry",
                Surface::Commands => "commands entry",
                Surface::Agents => "agents entry",
                Surface::Instructions => unreachable!(),
                Surface::Mcp => "MCP server",
                Surface::Hooks => "hooks/<event>.toml file",
            };

            for item in items {
                if !known.contains(item) {
                    errors.push(format!(
                        "unknown --only item `{}:{item}` — no Commons {noun} by that name",
                        surface.name()
                    ));
                }
            }
        }

        errors
    }
}

fn expand_root(text: &str, home: &Path) -> PathBuf {
    let text = text.trim();
    let path = if text == "~" {
        home.to_path_buf()
    } else if let Some(rest) = text.strip_prefix("~/").or_else(|| text.strip_prefix("~\\")) {
        home.join(rest)
    } else {
        PathBuf::from(text)
    };
    let path = if path.is_absolute() {
        path
    } else {
        home.join(path)
    };
    link::normalize(&path)
}

fn parse_only(
    value: &str,
    scope: &mut BTreeMap<Surface, Option<BTreeSet<String>>>,
) -> Result<(), String> {
    let (family, items) = match value.split_once(':') {
        Some((family, rest)) => (family, Some(rest)),
        None => (value, None),
    };

    let surface = Surface::from_name(family).ok_or_else(|| {
        format!(
            "unknown --only family `{family}` — expected skills, commands, agents, instructions, mcp, hooks"
        )
    })?;

    let items = match items {
        None => None,
        Some("") => {
            return Err(format!(
                "--only `{value}` has an empty item list — omit the `:` to select the whole surface"
            ));
        }
        Some(list) => {
            let mut set = BTreeSet::new();
            for item in list.split(',') {
                let item = item.trim();
                if item.is_empty() {
                    return Err(format!("--only `{value}` has an empty item"));
                }
                set.insert(item.to_string());
            }
            Some(set)
        }
    };

    if surface == Surface::Instructions && items.is_some() {
        return Err("--only instructions takes no items — it is one whole surface".into());
    }

    match (scope.get_mut(&surface), items) {
        (None, items) => {
            scope.insert(surface, items);
        }
        (Some(existing), Some(new)) => {
            if let Some(set) = existing {
                set.extend(new);
            }
        }
        // A whole-surface selector wins over any item set for that surface.
        (Some(existing), None) => *existing = None,
    }
    Ok(())
}

fn family_names(commons: &Commons, family: Family) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in commons.scan(family).entries {
        names.insert(entry.name);
        names.insert(entry.file_name);
    }
    names
}

fn mcp_names(commons: &Commons) -> Option<BTreeSet<String>> {
    mcp::commons_servers(&commons.root().join(commons::MCP))
        .ok()
        .map(|servers| servers.keys().cloned().collect())
}

/// Whether a link or file name matches one of the selected item names. Markdown
/// families keep their `.md` suffix on disk, so `ship` and `ship.md` are the
/// same item.
pub fn matches_file_name(name: &str, wanted: &BTreeSet<String>) -> bool {
    wanted.contains(name)
        || name
            .strip_suffix(".md")
            .is_some_and(|stem| wanted.contains(stem))
}

/// Whether a Commons entry matches one of the selected item names.
pub fn matches_entry(entry: &Entry, wanted: &BTreeSet<String>) -> bool {
    matches_file_name(&entry.name, wanted) || matches_file_name(&entry.file_name, wanted)
}

/// Whether a Commons scan issue is one of the selected items. A family-level
/// issue (an unreadable directory) has no name and stays with the family.
pub fn keeps_issue(issue: &commons::Issue, wanted: Option<&BTreeSet<String>>) -> bool {
    match wanted {
        None => true,
        Some(wanted) => issue
            .name()
            .is_none_or(|name| matches_file_name(name, wanted)),
    }
}
