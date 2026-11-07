//! `sync --force`: override the states sync normally leaves alone.

mod common;

use std::collections::BTreeSet;

use common::Fixture;

fn machine() -> Fixture {
    let f = Fixture::new();
    f.agent(".claude");
    f
}

fn tree_sans_state(f: &Fixture) -> BTreeSet<String> {
    f.tree()
        .into_iter()
        .filter(|entry| !entry.contains(".local"))
        .collect()
}

#[test]
fn force_replaces_a_variant_identical_skill() {
    let f = machine();
    f.commons_skill("research");
    let body = f.contents_of_commons("skills/research/SKILL.md");
    f.file(".claude/skills/research/SKILL.md", &body);

    let before = tree_sans_state(&f);
    f.run(&["sync", "--force"])
        .assert_code(1)
        .assert_stderr_has("--yes");
    assert_eq!(before, tree_sans_state(&f), "refusal changes nothing");

    f.run(&["sync", "--force", "--dry-run"])
        .assert_clean()
        .assert_stdout_has("force:");
    assert_eq!(before, tree_sans_state(&f), "preview changes nothing");

    f.run(&["sync", "--force", "--yes"]).assert_clean();
    assert!(f.is_symlink(".claude/skills/research"));
    assert!(
        f.contents(".claude/skills/research/SKILL.md")
            .contains("body of research")
    );
}

#[test]
fn force_discards_a_divergent_variant() {
    let f = machine();
    f.commons_skill("research");
    f.file(".claude/skills/research/SKILL.md", "local only\n");

    f.run(&["sync", "--force", "-y"]).assert_clean();

    assert!(f.is_symlink(".claude/skills/research"));
    assert!(
        f.contents(".claude/skills/research/SKILL.md")
            .contains("body of research")
    );
}

#[test]
fn force_replaces_a_foreign_link_but_not_a_foreign_extra() {
    let f = machine();
    f.dir("decoy");
    f.commons_skill("research");
    let decoy = f.path("decoy").display().to_string();
    f.symlink(".claude/skills/research", &decoy);
    f.symlink(".claude/skills/other", &decoy);

    f.run(&["sync", "--force", "--yes"]).assert_clean();

    assert!(
        f.contents(".claude/skills/research/SKILL.md")
            .contains("body of research")
    );
    assert!(
        f.is_symlink(".claude/skills/other"),
        "a foreign link with no Commons entry has nothing to be forced to"
    );
    assert_eq!(
        f.resolves_to(".claude/skills/other"),
        f.resolves_to("decoy")
    );
}

#[test]
fn force_replaces_a_symlink_route_conflict() {
    let f = machine();
    f.commons_file("AGENTS.md", "# shared instructions\n");
    f.file(
        ".config/agent-sync/agent-sync.toml",
        "[instructions]\nclaude = \"symlink\"\n",
    );
    f.file(".claude/CLAUDE.md", "the user's own prose\n");

    f.run(&["sync", "--force", "--yes"]).assert_clean();

    assert!(f.is_symlink(".claude/CLAUDE.md"));
    assert!(
        f.contents(".claude/CLAUDE.md")
            .contains("# shared instructions")
    );
}

#[test]
fn force_replaces_an_import_route_conflict_with_the_import_line() {
    let f = machine();
    f.commons_file("AGENTS.md", "# shared instructions\n");
    f.commons_file("other.md", "# other\n");
    f.symlink(".claude/CLAUDE.md", "../.agents/other.md");

    f.run(&["sync", "--force", "--yes"]).assert_clean();

    assert!(!f.is_symlink(".claude/CLAUDE.md"));
    assert!(
        f.contents(".claude/CLAUDE.md")
            .contains("@~/.agents/AGENTS.md")
    );
    assert_eq!(f.contents_of_commons("other.md"), "# other\n");
}

#[test]
fn force_overwrites_a_foreign_rendered_command() {
    let f = Fixture::new();
    f.agent(".gemini");
    f.commons_file(
        "commands/ship.md",
        "---\ndescription: Ship\n---\n\nShip it.\n",
    );
    f.file(".gemini/commands/ship.toml", "name = \"hand written\"\n");

    f.run(&["sync", "--force", "--yes"]).assert_clean();

    let body = f.contents(".gemini/commands/ship.toml");
    assert!(body.contains("agent-sync: this file is generated"));
    assert!(body.contains("Ship it."));
}

#[test]
fn force_leaves_mcp_and_hook_foreign_entries_alone() {
    let f = machine();
    f.commons_mcp(r#"{"mcpServers": {"serena": {"command": "uvx"}}}"#);
    f.file(
        ".claude.json",
        r#"{"mcpServers": {"user-server": {"command": "mine"}}}"#,
    );
    f.commons_file(
        "hooks/SessionStart.toml",
        "[[hook]]\ncommand = \"notify session\"\n",
    );
    f.file(
        ".claude/settings.json",
        r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"user-hook"}]}]}}"#,
    );

    // No forced item is present, so even without --yes the normal work runs.
    f.run(&["sync", "--force"]).assert_clean();

    let config = f.json(".claude.json");
    assert!(config["mcpServers"]["serena"].is_object());
    assert!(config["mcpServers"]["user-server"].is_object());

    let hooks = f.contents(".claude/settings.json");
    assert!(hooks.contains("notify session"));
    assert!(hooks.contains("user-hook"));
}

#[test]
fn force_respects_the_selection() {
    let f = machine();
    f.commons_skill("research");
    f.commons_skill("tdd");
    let research = f.contents_of_commons("skills/research/SKILL.md");
    let tdd = f.contents_of_commons("skills/tdd/SKILL.md");
    f.file(".claude/skills/research/SKILL.md", &research);
    f.file(".claude/skills/tdd/SKILL.md", &tdd);

    f.run(&["sync", "--force", "--yes", "--only", "skills:research"])
        .assert_clean();

    assert!(f.is_symlink(".claude/skills/research"));
    assert!(f.is_real_dir(".claude/skills/tdd"));
}

#[test]
fn force_without_forced_work_needs_no_confirmation() {
    let f = machine();
    f.commons_skill("research");

    f.run(&["sync", "--force"]).assert_clean();

    assert!(f.is_symlink(".claude/skills/research"));
}

#[test]
fn yes_without_force_is_refused() {
    let f = machine();

    f.run(&["sync", "--yes"])
        .assert_code(1)
        .assert_stderr_has("--force");
}

#[test]
fn an_include_entry_conflict_is_not_forced() {
    let f = Fixture::new();
    f.agent(".config/opencode");
    f.commons_file("AGENTS.md", "# shared instructions\n");
    f.file(".config/opencode/opencode.json", "{ this is not JSON");

    f.run(&["sync", "--force", "--yes"])
        .assert_clean()
        .assert_stdout_has("conflict");

    assert_eq!(
        f.contents(".config/opencode/opencode.json"),
        "{ this is not JSON",
        "force must not rewrite an agent config it cannot parse"
    );
}
