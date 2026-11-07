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
    const NAMES: [&'static str; 6] = [
        "skills",
        "commands",
        "agents",
        "instructions",
        "mcp",
        "hooks",
    ];

    /// Resolve a family name, accepting any unique case-insensitive prefix.
    fn resolve(name: &str) -> Result<Self, String> {
        match match_aliases(name, Self::NAMES.iter().map(|name| (*name, *name))) {
            NameMatch::Exact(name) | NameMatch::One(name) => Ok(Self::from_exact(name)),
            NameMatch::Many(names) => Err(format!(
                "ambiguous --only family `{name}` — matches {}; type more characters",
                names.join(", ")
            )),
            NameMatch::None => Err(format!(
                "unknown --only family `{name}` — expected {}",
                Self::NAMES.join(", ")
            )),
        }
    }

    fn from_exact(name: &str) -> Self {
        match name {
            "skills" => Self::Skills,
            "commands" => Self::Commands,
            "agents" => Self::Agents,
            "instructions" => Self::Instructions,
            "mcp" => Self::Mcp,
            "hooks" => Self::Hooks,
            _ => unreachable!("resolved from the known surface names"),
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

    /// Resolve every named item against the Commons, accepting any unique
    /// case-insensitive prefix. Exact names win over prefixes. Returns every
    /// unknown or ambiguous name so one run names them all.
    pub fn resolve_items(&mut self, commons: &Commons) -> Vec<String> {
        let mut errors = Vec::new();

        for (surface, items) in &mut self.scope {
            let surface = *surface;
            let Some(raw_items) = items else {
                continue;
            };
            let (known, noun): (Vec<(String, String)>, &str) = match surface {
                Surface::Skills => (family_aliases(commons, Family::Skills), "skills entry"),
                Surface::Commands => (family_aliases(commons, Family::Commands), "commands entry"),
                Surface::Agents => (family_aliases(commons, Family::Agents), "agents entry"),
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
                    (
                        names.into_iter().map(|name| (name.clone(), name)).collect(),
                        "MCP server",
                    )
                }
                Surface::Hooks => (
                    hooks::declared_events(&commons.family_dir(commons::HOOKS))
                        .into_iter()
                        .map(|name| (name.clone(), name))
                        .collect(),
                    "hooks/<event>.toml file",
                ),
            };

            let mut resolved = BTreeSet::new();
            for raw in raw_items.iter() {
                let aliases = known
                    .iter()
                    .map(|(alias, canonical)| (alias.as_str(), canonical.as_str()));
                match match_aliases(raw, aliases) {
                    NameMatch::Exact(name) | NameMatch::One(name) => {
                        resolved.insert(name.to_string());
                    }
                    NameMatch::Many(names) => errors.push(format!(
                        "ambiguous --only item `{}:{raw}` — matches {}; type more characters",
                        surface.name(),
                        names.join(", ")
                    )),
                    NameMatch::None => errors.push(format!(
                        "unknown --only item `{}:{raw}` — no Commons {noun} by that name",
                        surface.name()
                    )),
                }
            }
            *raw_items = resolved;
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

    let family = family.trim();
    if family.is_empty() {
        return Err(format!("--only `{value}` has an empty family"));
    }
    let surface = Surface::resolve(family)?;

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

fn family_aliases(commons: &Commons, family: Family) -> Vec<(String, String)> {
    let mut aliases = Vec::new();
    for entry in commons.scan(family).entries {
        aliases.push((entry.name.clone(), entry.name.clone()));
        if entry.file_name != entry.name {
            aliases.push((entry.file_name, entry.name));
        }
    }
    aliases
}

fn mcp_names(commons: &Commons) -> Option<BTreeSet<String>> {
    mcp::commons_servers(&commons.root().join(commons::MCP))
        .ok()
        .map(|servers| servers.keys().cloned().collect())
}

/// The result of resolving one typed name against a set of candidates.
enum NameMatch<'a> {
    Exact(&'a str),
    One(&'a str),
    Many(Vec<&'a str>),
    None,
}

/// Resolve `input` against `(alias, canonical)` pairs. Exact matches win, then
/// a unique case-insensitive prefix. Several matches are `Many` so the caller
/// can ask for more characters.
fn match_aliases<'a>(
    input: &str,
    aliases: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> NameMatch<'a> {
    let needle = input.to_lowercase();
    let mut exact = Vec::new();
    let mut prefix = Vec::new();

    for (alias, canonical) in aliases {
        let lower = alias.to_lowercase();
        if lower == needle {
            if !exact.contains(&canonical) {
                exact.push(canonical);
            }
        } else if lower.starts_with(&needle) && !prefix.contains(&canonical) {
            prefix.push(canonical);
        }
    }

    if exact.len() == 1 {
        NameMatch::Exact(exact[0])
    } else if exact.len() > 1 {
        exact.sort();
        NameMatch::Many(exact)
    } else {
        match prefix.len() {
            0 => NameMatch::None,
            1 => NameMatch::One(prefix[0]),
            _ => {
                prefix.sort();
                NameMatch::Many(prefix)
            }
        }
    }
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
