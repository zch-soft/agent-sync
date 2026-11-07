//! Selective sync: agents, alternate roots, surfaces, and named items.

mod common;

use std::collections::BTreeSet;

use common::Fixture;

/// A machine with a fan-out agent, a second fan-out agent, and a renderer.
fn machine() -> Fixture {
    let f = Fixture::new();
    f.agent(".claude");
    f.agent(".pi");
    f.agent(".gemini");
    f
}

/// The home tree minus agent-sync's own state directory.
fn tree_sans_state(f: &Fixture) -> BTreeSet<String> {
    f.tree()
        .into_iter()
        .filter(|entry| !entry.contains(".local"))
        .collect()
}

#[test]
fn only_one_agent_changes_only_that_agent() {
    let f = machine();
    f.commons_skill("research");

    f.run(&["sync", "--agent", "claude"])
        .assert_clean()
        .assert_stdout_has("claude")
        .assert_stdout_lacks("pi (");

    assert!(f.is_symlink(".claude/skills/research"));
    assert!(!f.present(".pi/agent/skills/research"));
}

#[test]
fn only_one_family_changes_only_that_family() {
    let f = machine();
    f.commons_skill("research");
    f.commons_file(
        "commands/ship.md",
        "---\ndescription: Ship\n---\n\nShip it.\n",
    );
    f.commons_mcp(r#"{"mcpServers": {"serena": {"command": "uvx"}}}"#);
    f.commons_file("AGENTS.md", "# shared instructions\n");
    f.commons_file(
        "hooks/SessionStart.toml",
        "[[hook]]\ncommand = \"notify\"\n",
    );

    f.run(&["sync", "--only", "skills"]).assert_clean();

    assert!(f.is_symlink(".claude/skills/research"));
    assert!(!f.present(".claude/commands/ship.md"));
    assert!(!f.present(".gemini/commands/ship.toml"));
    assert!(!f.present(".claude.json"));
    assert!(!f.present(".claude/CLAUDE.md"));
    assert!(!f.present(".claude/settings.json"));
}

#[test]
fn only_named_items_link_only_those() {
    let f = machine();
    f.commons_skill("research");
    f.commons_skill("tdd");
    // A wrong-shape entry in the same family must not leak into a narrowed run.
    f.commons_file("skills/README.md", "not a skill\n");

    let out = f.run(&["sync", "--only", "skills:research"]);

    out.assert_clean()
        .assert_stdout_has("research")
        .assert_stdout_lacks("tdd")
        .assert_stderr_lacks("README.md");
    assert!(f.is_symlink(".claude/skills/research"));
    assert!(!f.present(".claude/skills/tdd"));
}

#[test]
fn strict_blocks_on_a_problem_outside_the_selection() {
    let f = machine();
    f.commons_skill("research");
    f.commons_skill("tdd");
    f.commons_mcp(
        r#"{"mcpServers": {"serena": {"command": "uvx", "env": {"TOKEN": "${env:MISSING_VAR}"}}}}"#,
    );

    // Narrowed: the MCP surface is not surveyed, so its fault cannot block.
    f.run(&["sync", "--only", "skills:research"]).assert_clean();
    assert!(f.is_symlink(".claude/skills/research"));

    // Strict: the whole machine is audited, so the same fault blocks the
    // selected write.
    let out = f.run(&["sync", "--only", "skills:tdd", "--strict"]);
    out.assert_code(1).assert_stderr_has("MISSING_VAR");
    assert!(!f.present(".claude/skills/tdd"));
}

#[test]
fn only_named_mcp_servers_ignore_the_rest() {
    let f = machine();
    f.commons_mcp(
        r#"{"mcpServers": {
            "serena": {"command": "uvx"},
            "broken": {"command": "uvx", "env": {"TOKEN": "${env:MISSING_VAR}"}}
        }}"#,
    );

    f.run(&["sync", "--only", "mcp:serena"]).assert_clean();

    let config = f.json(".claude.json");
    assert!(config["mcpServers"]["serena"].is_object());
    assert!(
        config["mcpServers"]["broken"].is_null(),
        "an unselected server was not rendered"
    );

    let out = f.run(&["sync", "--only", "mcp:serena", "--strict"]);
    out.assert_code(1).assert_stderr_has("MISSING_VAR");
}

#[test]
fn only_named_hook_events_ignore_broken_siblings() {
    let f = machine();
    f.commons_file(
        "hooks/SessionStart.toml",
        "[[hook]]\ncommand = \"notify session\"\n",
    );
    f.commons_file("hooks/Stop.toml", "this is not TOML");

    f.run(&["sync", "--only", "hooks:SessionStart"])
        .assert_clean();

    let config = f.json(".claude/settings.json");
    assert!(config["hooks"]["SessionStart"].is_array());
    assert!(config["hooks"]["Stop"].is_null());

    let out = f.run(&["sync", "--only", "hooks:SessionStart", "--strict"]);
    out.assert_code(1).assert_stderr_has("Stop.toml");
}

#[test]
fn commands_selector_covers_fan_out_and_render() {
    let f = machine();
    f.commons_file(
        "commands/ship.md",
        "---\ndescription: Ship\n---\n\nShip it.\n",
    );
    f.commons_file(
        "commands/deploy.md",
        "---\ndescription: Deploy\n---\n\nDeploy it.\n",
    );
    f.file(
        ".gemini/commands/old.toml",
        &format!("{}\nname = \"old\"\n", agent_sync::render::MARKER),
    );

    f.run(&["sync", "--only", "commands:ship"]).assert_clean();

    assert!(f.is_symlink(".claude/commands/ship.md"));
    assert!(!f.present(".claude/commands/deploy.md"));
    assert!(f.present(".gemini/commands/ship.toml"));
    assert!(!f.present(".gemini/commands/deploy.toml"));
    assert!(
        f.present(".gemini/commands/old.toml"),
        "an item filter skips orphan discovery"
    );

    // A whole-family selection still prunes the orphan.
    f.run(&["sync", "--only", "commands"]).assert_clean();
    assert!(!f.present(".gemini/commands/old.toml"));
}

#[test]
fn instructions_can_be_selected_alone_and_rejects_items() {
    let f = machine();
    f.commons_file("AGENTS.md", "# shared instructions\n");
    f.commons_skill("research");

    f.run(&["sync", "--only", "instructions"]).assert_clean();

    assert!(f.present(".claude/CLAUDE.md"));
    assert!(
        f.contents(".claude/CLAUDE.md")
            .contains("@~/.agents/AGENTS.md")
    );
    assert!(!f.present(".claude/skills/research"));

    f.run(&["sync", "--only", "instructions:research"])
        .assert_code(1)
        .assert_stderr_has("takes no items");
}

#[test]
fn alternate_roots_move_every_agent_path() {
    let f = Fixture::new();
    f.agent(".hermes");
    f.dir("profiles/work");
    f.dir("profiles/home");
    f.commons_skill("research");

    f.run(&["sync", "--agent", "hermes", "--root", "profiles/work"])
        .assert_clean()
        .assert_stdout_has("profiles/work");

    assert!(f.is_symlink("profiles/work/skills/research"));
    assert!(!f.present(".hermes/skills/research"));

    f.run(&[
        "sync",
        "--agent",
        "hermes",
        "--root",
        "~/profiles/home",
        "--root",
        "profiles/work",
    ])
    .assert_clean();

    assert!(f.is_symlink("profiles/home/skills/research"));
    assert!(f.is_symlink("profiles/work/skills/research"));
}

#[test]
fn alternate_root_moves_paths_outside_the_registry_root_too() {
    let f = Fixture::new();
    f.agent(".claude");
    f.dir("profile");
    f.commons_skill("research");
    f.commons_mcp(r#"{"mcpServers": {"serena": {"command": "uvx"}}}"#);

    f.run(&["sync", "--agent", "claude", "--root", "profile"])
        .assert_clean();

    assert!(f.is_symlink("profile/skills/research"));
    assert!(f.present("profile/.claude.json"));
    assert!(!f.present(".claude/skills/research"));
    assert!(!f.present(".claude.json"));
}

#[test]
fn bad_selections_fail_before_any_write() {
    let f = machine();
    f.commons_skill("research");

    for (args, needle) in [
        (vec!["sync", "--agent", "nope"], "no Target named"),
        (vec!["sync", "--agent", "codex"], "is not installed"),
        (vec!["sync", "--only", "skills:nope"], "unknown --only item"),
        (vec!["sync", "--only", "nope"], "unknown --only family"),
        (vec!["sync", "--only", "skills:"], "empty item"),
        (
            vec!["sync", "--agent", "claude", "--agent", "pi", "--root", "x"],
            "exactly one --agent",
        ),
        (
            vec!["sync", "--agent", "claude", "--root", "missing-dir"],
            "is not a directory",
        ),
    ] {
        let out = f.run(&args);
        out.assert_code(1).assert_stderr_has(needle);
        assert!(
            !f.present(".claude/skills/research"),
            "no write for {args:?}"
        );
    }
}

#[test]
fn an_explicitly_selected_disabled_agent_is_refused() {
    let f = machine();
    f.file(
        ".config/agent-sync/agent-sync.toml",
        "[targets]\nclaude = false\n",
    );
    f.commons_skill("research");

    f.run(&["sync", "--agent", "claude"])
        .assert_code(1)
        .assert_stderr_has("disabled");
    assert!(!f.present(".claude/skills/research"));
}

#[test]
fn alternate_root_is_refused_for_a_custom_target() {
    let f = Fixture::new();
    f.agent(".myagent");
    f.dir("profile");
    f.file(
        ".config/agent-sync/agent-sync.toml",
        "[custom.myagent]\nroot = \".myagent\"\nskills = \".myagent/skills\"\n",
    );
    f.commons_skill("research");

    f.run(&["sync", "--agent", "myagent", "--root", "profile"])
        .assert_code(1)
        .assert_stderr_has("not supported for custom Target");
    assert!(!f.present(".myagent/skills/research"));
}

#[test]
fn family_and_item_names_accept_unique_case_insensitive_prefixes() {
    let f = machine();
    f.commons_file("AGENTS.md", "# shared instructions\n");
    f.commons_skill("research");
    f.commons_skill("ima-skills");
    f.commons_skill("ima-notes");
    f.commons_mcp(r#"{"mcpServers": {"serena": {"command": "uvx"}}}"#);

    // Single-letter family and upper-case item prefixes.
    f.run(&["sync", "--only", "I"]).assert_clean();
    assert!(
        f.contents(".claude/CLAUDE.md")
            .contains("@~/.agents/AGENTS.md")
    );

    f.run(&["sync", "--only", "S:RES", "--only", "MCP:SER"])
        .assert_clean();
    assert!(f.is_symlink(".claude/skills/research"));
    assert!(f.json(".claude.json")["mcpServers"]["serena"].is_object());
    assert!(!f.present(".claude/skills/ima-skills"));

    // An ambiguous prefix names every candidate and writes nothing.
    let out = f.run(&["sync", "--only", "skills:IMA"]);
    out.assert_code(1)
        .assert_stderr_has("ambiguous")
        .assert_stderr_has("ima-notes")
        .assert_stderr_has("ima-skills");
    assert!(!f.present(".claude/skills/ima-skills"));

    // Exact names win over prefixes.
    f.commons_skill("ima");
    f.run(&["sync", "--only", "skills:IMA"]).assert_clean();
    assert!(f.is_symlink(".claude/skills/ima"));
    assert!(!f.present(".claude/skills/ima-skills"));

    f.run(&["sync", "--only", "skills:ZZZ"])
        .assert_code(1)
        .assert_stderr_has("unknown")
        .assert_stderr_has("ZZZ");
}

#[test]
fn repeated_agents_and_selectors_merge() {
    let f = machine();
    f.commons_skill("research");
    f.commons_skill("tdd");
    f.commons_mcp(r#"{"mcpServers": {"serena": {"command": "uvx"}}}"#);

    f.run(&[
        "sync",
        "--agent",
        "claude",
        "--agent",
        "pi",
        "--only",
        "skills:research",
        "--only",
        "skills:tdd",
        "--only",
        "mcp:serena",
    ])
    .assert_clean()
    .assert_stdout_has("claude")
    .assert_stdout_has("pi (");

    assert!(f.is_symlink(".claude/skills/research"));
    assert!(f.is_symlink(".claude/skills/tdd"));
    assert!(f.is_symlink(".pi/agent/skills/research"));
    assert!(f.is_symlink(".pi/agent/skills/tdd"));
    assert!(f.json(".claude.json")["mcpServers"]["serena"].is_object());

    // A whole-surface selector wins over an item set for that surface.
    f.commons_skill("third");
    f.run(&["sync", "--only", "skills:tdd", "--only", "skills"])
        .assert_clean();
    assert!(f.is_symlink(".claude/skills/third"));
}

#[test]
fn a_narrowed_hook_event_does_not_reclassify_other_events_as_foreign() {
    let f = machine();
    f.commons_file(
        "hooks/SessionStart.toml",
        "[[hook]]\ncommand = \"notify session\"\n",
    );
    f.commons_file("hooks/Stop.toml", "[[hook]]\ncommand = \"clean up\"\n");
    f.run(&["sync"]).assert_clean();

    let out = f.run(&["sync", "--only", "hooks:SessionStart"]);
    out.assert_clean().assert_stdout_lacks("foreign");

    // The unselected event is left exactly as the full sync wrote it.
    let config = f.json(".claude/settings.json");
    assert!(config["hooks"]["Stop"].is_array());
}

#[test]
fn a_malformed_mcp_file_reports_its_parse_error_not_an_unknown_item() {
    let f = machine();
    f.commons_mcp("{ this is not JSON");

    f.run(&["sync", "--only", "mcp:serena"])
        .assert_code(1)
        .assert_stderr_has("not valid JSON");
}

#[test]
fn a_filtered_dry_run_prints_the_plan_the_filtered_run_makes() {
    let f = machine();
    f.commons_skill("research");
    f.commons_skill("tdd");

    let before = tree_sans_state(&f);
    let dry = f.run(&["sync", "--only", "skills:research", "--dry-run"]);
    dry.assert_clean();
    assert_eq!(before, tree_sans_state(&f), "a dry run changes nothing");

    let real = f.run(&["sync", "--only", "skills:research"]);
    real.assert_clean();

    let plan = dry
        .stdout
        .strip_prefix("dry run — no changes will be made\n\n")
        .expect("the dry-run banner")
        .replace("would be made.", "made.");
    assert_eq!(plan, real.stdout, "the filtered preview must match the run");
    assert!(!f.present(".claude/skills/tdd"));
}
