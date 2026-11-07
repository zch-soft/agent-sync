//! `sync` — make every Target match the Commons.
//!
//! Two phases (ADR-0007). Phase 1 surveys every family read-only and collects
//! **every** problem; if any exist they are all reported and the run exits
//! having written nothing, so the user fixes one complete list rather than one
//! error per run. Phase 2 executes the plan; a write failure there is reported
//! and the remaining operations continue, because everything was resolved
//! before the first write and an idempotent re-run converges. `--dry-run` is
//! phase 1 plus the printed plan — one code path, so the preview cannot drift
//! from what a real run would do.

use std::path::Path;

use crate::cli::SyncArgs;
use crate::commons::{self, Commons};
use crate::config::Config;
use crate::env::Env;
use crate::family::Family;
use crate::hooks;
use crate::instructions;
use crate::link::{self, Item};
use crate::lock;
use crate::mcp;
use crate::render;
use crate::report::Reporter;
use crate::select::{self, Selection, Surface};
use crate::target::Target;
use crate::{EXIT_CLEAN, EXIT_ERROR};

/// Everything one run intends to do, surveyed and resolved before any write.
struct Plan {
    /// Advisory Commons scan issues, in family order. Never block the run.
    warnings: Vec<String>,
    /// Symlink-family surveys, in family then Target order.
    links: Vec<LinkPlan>,
    instructions: Vec<instructions::Item>,
    mcp: mcp::Survey,
    rendered: Vec<render::Item>,
    hooks: hooks::Survey,
    /// Blocking survey problems, collected across every family. Any entry:
    /// phase 2 never runs.
    problems: Vec<String>,
}

/// One Target directory's worth of symlink work.
struct LinkPlan {
    agent: String,
    dir_display: String,
    items: Vec<Item>,
}

impl Plan {
    /// Phase 1: survey the selected scope, read-only, collecting every problem.
    fn build(
        env: &Env,
        config: &Config,
        commons: &Commons,
        targets: &[Target],
        selection: &Selection,
    ) -> Plan {
        let mut warnings = Vec::new();
        let mut links = Vec::new();

        for family in Family::ALL {
            let surface = Surface::for_family(*family);
            if !selection.allows(surface) {
                continue;
            }

            let scan = commons.scan(*family);
            let only = selection.items(surface);
            let entries: Vec<commons::Entry> = scan
                .entries
                .into_iter()
                .filter(|entry| only.is_none_or(|only| select::matches_entry(entry, only)))
                .collect();
            warnings.extend(
                scan.issues
                    .into_iter()
                    .filter(|issue| select::keeps_issue(issue, only))
                    .map(|issue| issue.to_string()),
            );

            for target in targets {
                if let Some(dir) = target.fanout_dir(*family) {
                    let path = target.path(env, dir);
                    let items = link::survey_selected(&path, commons.root(), &entries, only);
                    links.push(LinkPlan {
                        agent: target.label(env),
                        dir_display: Target::display_path(env, &path),
                        items,
                    });
                } else if let Some(dir) = target.legacy_dir(*family) {
                    // A flipped agent's old fan-out dir: the agent reads it
                    // *and* the Commons, so our links there are live
                    // duplicates. Prune-only — remove ours, create nothing,
                    // touch nothing foreign.
                    let path = target.path(env, dir);
                    let items = link::survey_legacy_selected(&path, commons.root(), only);
                    links.push(LinkPlan {
                        agent: target.label(env),
                        dir_display: Target::display_path(env, &path),
                        items,
                    });
                }
            }
        }

        let instructions = if selection.allows(Surface::Instructions) {
            instructions::survey_for(env, targets, &commons.root().join(commons::INSTRUCTIONS))
        } else {
            Vec::new()
        };
        let mut mcp = if selection.allows(Surface::Mcp) {
            mcp::survey_for(
                env,
                config,
                targets,
                &commons.root().join(commons::MCP),
                selection.items(Surface::Mcp),
            )
        } else {
            mcp::Survey::default()
        };
        let mut rendered = if selection.allows(Surface::Commands) {
            render::survey_for(env, targets, commons, selection.items(Surface::Commands))
        } else {
            render::Survey::default()
        };
        let mut hooks = if selection.allows(Surface::Hooks) {
            hooks::survey_for(
                env,
                targets,
                &commons.family_dir(commons::HOOKS),
                selection.items(Surface::Hooks),
            )
        } else {
            hooks::Survey::default()
        };

        let mut problems = Vec::new();
        problems.append(&mut mcp.problems);
        problems.append(&mut rendered.problems);
        problems.append(&mut hooks.problems);

        Plan {
            warnings,
            links,
            instructions,
            mcp,
            rendered: rendered.items,
            hooks,
            problems,
        }
    }

    /// How many items `--force` would change. Used to decide whether the run
    /// needs `--yes`; zero means `--force` is a no-op.
    fn forced_count(&self, force: bool) -> usize {
        if !force {
            return 0;
        }
        let links: usize = self
            .links
            .iter()
            .flat_map(|plan| &plan.items)
            .filter(|item| link::forceable(item))
            .count();
        let instructions = self
            .instructions
            .iter()
            .filter(|item| instructions::forceable(item))
            .count();
        let rendered = self
            .rendered
            .iter()
            .filter(|item| render::forceable(item))
            .count();
        links + instructions + rendered
    }
}

pub fn run(env: &Env, config: &Config, r: &mut Reporter, args: &SyncArgs) -> i32 {
    let dry_run = args.dry_run;
    let commons = Commons::new(env.commons());
    if !commons.exists() {
        r.problem(commons::missing_message(commons.root()));
        return EXIT_ERROR;
    }

    let mut selection = match Selection::parse(args, env) {
        Ok(selection) => selection,
        Err(e) => {
            r.problem(e);
            return EXIT_ERROR;
        }
    };

    let targets = match selection.resolve_targets(env, config) {
        Ok(targets) => targets,
        Err(errors) => {
            for error in errors {
                r.problem(error);
            }
            return EXIT_ERROR;
        }
    };

    let item_errors = selection.resolve_items(&commons);
    if !item_errors.is_empty() {
        for error in item_errors {
            r.problem(error);
        }
        return EXIT_ERROR;
    }

    // A dry run only reads, so it never contends for the lock.
    let _lock = if dry_run {
        None
    } else {
        match lock::acquire(env.state_dir(), lock::timeout(env)) {
            Ok(guard) => Some(guard),
            Err(e) => {
                r.problem(e.to_string());
                return EXIT_ERROR;
            }
        }
    };

    if dry_run {
        r.line("dry run — no changes will be made");
        r.blank();
    }

    // `--strict` audits the whole machine while the selected plan still decides
    // what is written. An audit fault blocks the run exactly like a normal
    // survey fault; the selected survey must never hide it.
    let audit = if selection.strict && selection.is_selective() {
        let audit_targets = match selection.audit_targets(env, config) {
            Ok(targets) => targets,
            Err(errors) => {
                for error in errors {
                    r.problem(error);
                }
                return EXIT_ERROR;
            }
        };
        Some(Plan::build(
            env,
            config,
            &commons,
            &audit_targets,
            &Selection::all(),
        ))
    } else {
        None
    };

    if let Some(audit) = &audit {
        for warning in &audit.warnings {
            r.warn(warning);
        }
        let skipped: Vec<&String> = audit
            .mcp
            .skipped
            .iter()
            .chain(&audit.hooks.skipped)
            .collect();
        if !audit.problems.is_empty() || !skipped.is_empty() {
            for problem in &audit.problems {
                r.problem(problem);
            }
            for problem in skipped {
                r.problem(problem);
            }
            return EXIT_ERROR;
        }
    }

    // Phase 1 — survey the selected scope before the first write.
    let plan = Plan::build(env, config, &commons, &targets, &selection);
    if audit.is_none() {
        for warning in &plan.warnings {
            r.warn(warning);
        }
    }

    let mut changes = 0usize;
    if plan.problems.is_empty() {
        // `--force` may change states sync normally leaves alone. Without
        // `--yes`, print that plan and stop before writing; `--dry-run`
        // previews it the same way.
        let forced = plan.forced_count(args.force);
        let refusing = args.force && !args.yes && !dry_run && forced > 0;
        let preview = dry_run || refusing;

        // Phase 2 — execute the plan (a dry run prints it instead). Hooks are
        // written after MCP because Gemini keeps both in one settings file,
        // and each family re-reads it before merging.
        for link_plan in &plan.links {
            changes += sync_target(link_plan, env.home(), r, preview, args.force);
        }
        changes += sync_instructions(env, &plan.instructions, r, preview, args.force);
        changes += sync_mcp(&plan.mcp, r, preview);
        changes += sync_rendered(&plan.rendered, r, preview, args.force);
        changes += sync_hooks(&plan.hooks, r, preview);

        if refusing {
            r.blank();
            r.problem(format!(
                "refusing to force {forced} change(s) without --yes — rerun with --yes, or preview with --dry-run"
            ));
            return EXIT_ERROR;
        }
    } else {
        // A survey problem anywhere in the selection stops the whole run before
        // its first write, so a config error can never leave a half-synced
        // machine.
        for problem in &plan.problems {
            r.problem(problem);
        }
        // Skipped Targets are faults too; name every problem in the same run.
        for skipped in plan.mcp.skipped.iter().chain(&plan.hooks.skipped) {
            r.problem(skipped);
        }
    }

    if changes == 0 && r.problem_count() == 0 {
        r.line("Everything is up to date.");
    } else if changes == 0 {
        r.blank();
        r.line("No changes were made — see the errors above.");
    } else {
        r.blank();
        let noun = if changes == 1 { "change" } else { "changes" };
        let verb = if dry_run { "would be made" } else { "made" };
        r.line(format!("{changes} {noun} {verb}."));
    }

    if r.problem_count() > 0 {
        EXIT_ERROR
    } else {
        EXIT_CLEAN
    }
}

/// The rendered family: whole files generated for agents that cannot take a
/// symlink, each carrying the Marker that makes it ours to replace.
fn sync_rendered(items: &[render::Item], r: &mut Reporter, dry_run: bool, force: bool) -> usize {
    let noteworthy: Vec<&render::Item> = items
        .iter()
        .filter(|i| i.state != render::State::Managed)
        .collect();
    if noteworthy.is_empty() {
        return 0;
    }

    r.line("rendered commands");
    for item in &noteworthy {
        let note = if force && render::forceable(item) {
            item.force_note()
        } else {
            item.state.note()
        };
        r.line(format!(
            "  {:<10} {} → {} — {}",
            item.state.label(),
            item.name,
            item.target,
            note
        ));
    }

    let changing: Vec<&&render::Item> = noteworthy
        .iter()
        .filter(|i| i.state.needs_change() || (force && render::forceable(i)))
        .collect();
    if changing.is_empty() || dry_run {
        return changing.len();
    }

    let mut written = 0usize;
    for item in &changing {
        let forced = force && render::forceable(item);
        let result = if forced {
            render::apply_forced(item)
        } else {
            render::apply(item)
        };
        match result {
            Ok(()) => written += 1,
            Err(e) => r.problem(e.to_string()),
        }
    }
    written
}

/// The hooks family: command-hooks merged into each agent's hook arrays.
fn sync_hooks(survey: &hooks::Survey, r: &mut Reporter, dry_run: bool) -> usize {
    for skipped in &survey.skipped {
        r.problem(skipped);
    }

    let noteworthy: Vec<&hooks::Item> = survey
        .items
        .iter()
        .filter(|i| i.state != hooks::State::Managed)
        .collect();
    if noteworthy.is_empty() {
        return 0;
    }

    r.line("hooks");
    for item in &noteworthy {
        r.line(format!(
            "  {:<12} {} {} → {} — {}",
            item.state.label(),
            item.event,
            short(&item.label),
            item.target,
            item.note()
        ));
    }

    let changing: Vec<&hooks::Item> = noteworthy
        .into_iter()
        .filter(|i| i.state.needs_change())
        .collect();
    if changing.is_empty() || dry_run {
        return changing.len();
    }

    let mut written = 0usize;
    for (path, group) in by_file(&changing) {
        match hooks::apply(path, group[0].root_key, &group) {
            Ok(report) => {
                written += group.len();
                if report.exposed {
                    r.warn(format!(
                        "{} has mode {:o} — it now holds resolved secrets and can be read by others",
                        report.path.display(),
                        report.mode
                    ));
                }
            }
            Err(e) => r.problem(e.to_string()),
        }
    }

    written
}

/// A command, shortened for a report line. Never resolved.
fn short(command: &str) -> String {
    const MAX: usize = 40;
    if command.chars().count() <= MAX {
        return command.to_string();
    }
    let head: String = command.chars().take(MAX - 1).collect();
    format!("{head}…")
}

/// The MCP family: one Commons file, rendered and key-merged per agent.
fn sync_mcp(survey: &mcp::Survey, r: &mut Reporter, dry_run: bool) -> usize {
    // A config agent-sync cannot parse is a real fault worth an exit code, but
    // it must not stop the Targets that are healthy.
    for skipped in &survey.skipped {
        r.problem(skipped);
    }
    let noteworthy: Vec<&mcp::Item> = survey
        .items
        .iter()
        .filter(|i| i.state != mcp::State::Managed)
        .collect();
    if noteworthy.is_empty() {
        return 0;
    }

    r.line("mcp (mcp.json)");
    for item in &noteworthy {
        r.line(format!(
            "  {:<9} {} → {} — {}",
            item.state.label(),
            item.name,
            item.target,
            item.note()
        ));
    }

    let changing: Vec<&mcp::Item> = noteworthy
        .into_iter()
        .filter(|i| i.state.needs_change())
        .collect();
    if changing.is_empty() || dry_run {
        return changing.len();
    }

    // A Disabled server's leftover render leaves through the one deletion
    // primitive: its name is still in the Commons, so the removal rests on the
    // same name identity as the write that put it there.
    let (removals, writes): (Vec<&mcp::Item>, Vec<&mcp::Item>) = changing
        .into_iter()
        .partition(|i| i.state == mcp::State::Disabled);

    // One agent's write failing must not cancel the others: everything was
    // already resolved before any file was opened, so the remaining writes are
    // still correct. Each failure is reported and the run ends non-clean.
    let mut written = 0usize;
    for item in &removals {
        match mcp::strip(&item.path, item.root_key, item.format, &item.name) {
            Ok(_) => written += 1,
            Err(e) => r.problem(e.to_string()),
        }
    }
    for (path, group) in by_file(&writes) {
        let root_key = group[0].root_key;
        match mcp::apply(path, root_key, group[0].format, &group) {
            Ok(report) => {
                written += group.len();
                if report.exposed {
                    r.warn(format!(
                        "{} has mode {:o} — it now holds resolved secrets and can be read by others",
                        report.path.display(),
                        report.mode
                    ));
                }
            }
            Err(e) => r.problem(e.to_string()),
        }
    }

    written
}

/// A trait for items that know which config file they belong to, so shared
/// files receive one coherent write.
trait InFile {
    fn path(&self) -> &Path;
}

impl InFile for mcp::Item {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl InFile for hooks::Item {
    fn path(&self) -> &Path {
        &self.path
    }
}

/// Group items by the config file they land in, preserving order. One write
/// per file, not per item: an agent may take more than one entry — or more
/// than one family — in the same file, and a write per item would clobber the
/// previous one.
fn by_file<'i, T: InFile>(items: &[&'i T]) -> Vec<(&'i Path, Vec<&'i T>)> {
    let mut grouped: Vec<(&Path, Vec<&T>)> = Vec::new();
    for item in items.iter().copied() {
        match grouped.iter_mut().find(|(path, _)| *path == item.path()) {
            Some((_, group)) => group.push(item),
            None => grouped.push((item.path(), vec![item])),
        }
    }
    grouped
}

/// The instructions family: one Commons file, each agent on its chosen route.
fn sync_instructions(
    env: &Env,
    items: &[instructions::Item],
    r: &mut Reporter,
    dry_run: bool,
    force: bool,
) -> usize {
    let noteworthy: Vec<&instructions::Item> = items
        .iter()
        .filter(|i| i.state != instructions::State::Linked)
        .filter(|i| i.state != instructions::State::ImportPresent)
        .collect();
    if noteworthy.is_empty() {
        return 0;
    }

    r.line("instructions (AGENTS.md)");
    let mut changed = 0;
    // A Target's leftovers follow its chosen route; once that write fails,
    // removing them would leave the agent with no instructions at all.
    let mut failed: Vec<&str> = Vec::new();

    for item in noteworthy {
        if failed.contains(&item.target.as_str()) {
            continue;
        }
        let forced = force && instructions::forceable(item);
        let changing = item.state.needs_change() || forced;
        if !changing {
            r.line(format!(
                "  {:<9} {} — {}",
                item.state.label(),
                item.target,
                item.note()
            ));
            continue;
        }
        let note = if forced { item.force_note() } else { "" };
        if dry_run {
            if forced {
                r.line(format!(
                    "  {:<9} {} — {note}",
                    item.state.label(),
                    item.target
                ));
            } else {
                r.line(format!("  {:<9} {}", item.state.label(), item.target));
            }
            changed += 1;
        } else {
            let result = if forced {
                instructions::apply_forced(env, item)
            } else {
                instructions::apply(env, item)
            };
            match result {
                Err(e) => {
                    r.problem(format!("cannot write {}: {e}", item.path.display()));
                    failed.push(&item.target);
                }
                Ok(()) => {
                    if forced {
                        r.line(format!(
                            "  {:<9} {} — {note}",
                            item.state.label(),
                            item.target
                        ));
                    } else {
                        r.line(format!("  {:<9} {}", item.state.label(), item.target));
                    }
                    changed += 1;
                }
            }
        }
    }

    changed
}

/// Report — and unless this is a dry run, apply — one Target's items. Returns
/// how many of them changed the filesystem.
fn sync_target(
    plan: &LinkPlan,
    home: &Path,
    r: &mut Reporter,
    dry_run: bool,
    force: bool,
) -> usize {
    let interesting: Vec<&Item> = plan
        .items
        .iter()
        .filter(|i| i.state != link::State::Linked)
        .collect();
    if interesting.is_empty() {
        return 0;
    }

    r.line(format!("{} ({})", plan.agent, plan.dir_display));
    let mut changed = 0;

    for item in interesting {
        let forced = force && link::forceable(item);
        let changing = item.state.needs_change() || forced;
        let note = if forced {
            item.force_note()
        } else {
            item.state.note()
        };
        if !changing {
            r.line(format!(
                "  {:<9} {} — {note}",
                item.state.label(),
                item.name
            ));
            continue;
        }

        if dry_run {
            if forced {
                r.line(format!(
                    "  {:<9} {} — {note}",
                    item.state.label(),
                    item.name
                ));
            } else {
                r.line(format!("  {:<9} {}", item.state.label(), item.name));
            }
            changed += 1;
        } else {
            let result = if forced {
                link::apply_forced(item)
            } else {
                link::apply(item)
            };
            match result {
                Err(e) => r.problem(format!(
                    "cannot fix {}: {e}",
                    item.path.strip_prefix(home).unwrap_or(&item.path).display()
                )),
                Ok(()) => {
                    if forced {
                        r.line(format!(
                            "  {:<9} {} — {note}",
                            item.state.label(),
                            item.name
                        ));
                    } else {
                        r.line(format!("  {:<9} {}", item.state.label(), item.name));
                    }
                    changed += 1;
                }
            }
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_write_skips_the_rest_of_that_targets_instructions() {
        let home = tempfile::tempdir().unwrap();
        let h = home.path();
        let entry = "{\"instructions\": [\"~/.agents/AGENTS.md\"]}\n";
        for (path, text) in [
            (".agents/AGENTS.md", "# shared instructions\n"),
            (".config/opencode/opencode.json", entry),
            (
                ".config/agent-sync/agent-sync.toml",
                "[instructions]\nopencode = \"symlink\"\n",
            ),
            ("blocker", ""),
        ] {
            std::fs::create_dir_all(h.join(path).parent().unwrap()).unwrap();
            std::fs::write(h.join(path), text).unwrap();
        }
        let env = Env::resolve(&[("HOME".into(), h.display().to_string())]).unwrap();
        let config = Config::load(env.config_dir()).unwrap();
        let mut items = instructions::survey(&env, &config, &h.join(".agents/AGENTS.md"));
        let states: Vec<_> = items
            .iter()
            .map(|i| (i.target.as_str(), &i.state))
            .collect();
        assert_eq!(
            states,
            [
                ("opencode", &instructions::State::Missing),
                ("opencode", &instructions::State::Legacy),
            ],
            "the chosen link, then the include entry it replaces"
        );
        // A file where the link's folder should be fails the write on every platform.
        items[0].path = h.join("blocker/AGENTS.md");

        let (mut out, mut err) = (Vec::new(), Vec::new());
        let mut r = Reporter::new(&mut out, &mut err);
        sync_instructions(&env, &items, &mut r, false, false);

        assert_eq!(r.problem_count(), 1);
        assert_eq!(
            std::fs::read_to_string(h.join(".config/opencode/opencode.json")).unwrap(),
            entry,
            "a failed link must not cost opencode the entry it still reads"
        );
    }
}
