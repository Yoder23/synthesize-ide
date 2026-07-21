use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub argv: Vec<String>,
    pub cwd: String,
    pub requires_network: bool,
    pub may_modify_files: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandRisk {
    ReadOnly,
    TestOrBuild,
    ModifiesRepo,
    Network,
    Destructive,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleAction {
    Allow,
    RequireApproval(CommandRisk),
    Block,
}

#[derive(Debug, Clone)]
pub struct CommandRule {
    pub program: String,
    pub args_prefix: Vec<String>,
    pub args_contains: Vec<String>,
    pub action: RuleAction,
    pub reason: String,
}

impl CommandRule {
    pub fn matches(&self, argv: &[String]) -> bool {
        let Some(program) = argv.first() else {
            return false;
        };
        if normalize(program) != normalize(&self.program) {
            return false;
        }

        let args: Vec<String> = normalize_args(&argv.iter().skip(1).cloned().collect::<Vec<_>>());

        if !self.args_prefix.is_empty() {
            let prefix: Vec<String> = self.args_prefix.iter().map(|a| normalize(a)).collect();
            if args.len() < prefix.len() || args[..prefix.len()] != prefix[..] {
                return false;
            }
        }

        self.args_contains
            .iter()
            .all(|needle| args.iter().any(|arg| arg == &normalize(needle)))
    }
}

#[derive(Debug, Clone)]
pub struct CommandPolicy {
    pub allow_programs: Vec<String>,
    pub deny_programs: Vec<String>,
    pub rules: Vec<CommandRule>,
    pub max_runtime_seconds: u64,
    pub max_output_bytes: usize,
    pub network_default_disabled: bool,
    /// When true, an allowlisted program is still refused unless a concrete rule matched.
    /// This is the safer mode for user-entered Personal Terminal commands.
    pub require_explicit_rule: bool,
}

impl Default for CommandPolicy {
    fn default() -> Self {
        Self {
            allow_programs: vec![
                "git".into(),
                "rg".into(),
                "ls".into(),
                "cat".into(),
                "npm".into(),
                "pnpm".into(),
                "yarn".into(),
                "pytest".into(),
                "cargo".into(),
                "go".into(),
                "dotnet".into(),
            ],
            deny_programs: vec![
                "sudo".into(),
                "ssh".into(),
                "scp".into(),
                "curl".into(),
                "wget".into(),
                "rm".into(),
                "chmod".into(),
                "chown".into(),
                "bash".into(),
                "sh".into(),
                "zsh".into(),
                "fish".into(),
                "powershell".into(),
                "pwsh".into(),
                "cmd".into(),
                "node".into(),
            ],
            rules: default_rules(),
            max_runtime_seconds: 120,
            max_output_bytes: 2_000_000,
            network_default_disabled: true,
            require_explicit_rule: false,
        }
    }
}

fn default_rules() -> Vec<CommandRule> {
    vec![
        block(
            "git",
            &["push"],
            &[],
            "git push mutates a remote and uses network",
        ),
        block(
            "git",
            &["pull"],
            &[],
            "git pull mutates the working tree and uses network",
        ),
        block("git", &["fetch"], &[], "git fetch uses network"),
        block(
            "git",
            &["checkout"],
            &[],
            "git checkout can mutate the working tree",
        ),
        block(
            "git",
            &["switch"],
            &[],
            "git switch can mutate the working tree",
        ),
        block("git", &["restore"], &[], "git restore can overwrite files"),
        block("git", &["add"], &[], "git add mutates the index"),
        block(
            "git",
            &["commit"],
            &[],
            "git commit mutates repository history",
        ),
        block(
            "git",
            &["merge"],
            &[],
            "git merge can mutate the working tree",
        ),
        block(
            "git",
            &["rebase"],
            &[],
            "git rebase rewrites history and mutates the working tree",
        ),
        block(
            "git",
            &["reset"],
            &[],
            "git reset can mutate the index and working tree",
        ),
        block("git", &["clean"], &[], "git clean deletes untracked files"),
        block("git", &["stash"], &[], "git stash mutates repository state"),
        approve(
            "git",
            &["status"],
            &[],
            CommandRisk::ReadOnly,
            "git status is read-only",
        ),
        approve(
            "git",
            &["diff"],
            &[],
            CommandRisk::ReadOnly,
            "git diff is read-only",
        ),
        approve(
            "git",
            &["log"],
            &[],
            CommandRisk::ReadOnly,
            "git log is read-only",
        ),
        approve(
            "rg",
            &[],
            &[],
            CommandRisk::ReadOnly,
            "ripgrep searches files",
        ),
        approve("ls", &[], &[], CommandRisk::ReadOnly, "ls lists files"),
        approve("cat", &[], &[], CommandRisk::ReadOnly, "cat prints files"),
        approve(
            "npm",
            &["install"],
            &[],
            CommandRisk::Network,
            "npm install may use network and mutates node_modules/lockfiles",
        ),
        approve(
            "pnpm",
            &["install"],
            &[],
            CommandRisk::Network,
            "pnpm install may use network and mutates node_modules/lockfiles",
        ),
        approve(
            "yarn",
            &["install"],
            &[],
            CommandRisk::Network,
            "yarn install may use network and mutates node_modules/lockfiles",
        ),
        approve(
            "npm",
            &["test"],
            &[],
            CommandRisk::TestOrBuild,
            "npm test executes package scripts",
        ),
        approve(
            "pnpm",
            &["test"],
            &[],
            CommandRisk::TestOrBuild,
            "pnpm test executes package scripts",
        ),
        approve(
            "yarn",
            &["test"],
            &[],
            CommandRisk::TestOrBuild,
            "yarn test executes package scripts",
        ),
        approve(
            "npm",
            &["run", "test"],
            &[],
            CommandRisk::TestOrBuild,
            "npm run test executes package scripts",
        ),
        approve(
            "pnpm",
            &["run", "test"],
            &[],
            CommandRisk::TestOrBuild,
            "pnpm run test executes package scripts",
        ),
        approve(
            "yarn",
            &["run", "test"],
            &[],
            CommandRisk::TestOrBuild,
            "yarn run test executes package scripts",
        ),
        approve(
            "npm",
            &["run", "lint"],
            &[],
            CommandRisk::TestOrBuild,
            "npm run lint executes package scripts",
        ),
        approve(
            "pnpm",
            &["run", "lint"],
            &[],
            CommandRisk::TestOrBuild,
            "pnpm run lint executes package scripts",
        ),
        approve(
            "yarn",
            &["run", "lint"],
            &[],
            CommandRisk::TestOrBuild,
            "yarn run lint executes package scripts",
        ),
        approve(
            "npm",
            &["run", "build"],
            &[],
            CommandRisk::TestOrBuild,
            "npm run build executes package scripts",
        ),
        approve(
            "pnpm",
            &["run", "build"],
            &[],
            CommandRisk::TestOrBuild,
            "pnpm run build executes package scripts",
        ),
        approve(
            "yarn",
            &["run", "build"],
            &[],
            CommandRisk::TestOrBuild,
            "yarn run build executes package scripts",
        ),
        approve(
            "npm",
            &["run", "typecheck"],
            &[],
            CommandRisk::TestOrBuild,
            "npm run typecheck executes package scripts",
        ),
        approve(
            "pnpm",
            &["run", "typecheck"],
            &[],
            CommandRisk::TestOrBuild,
            "pnpm run typecheck executes package scripts",
        ),
        approve(
            "yarn",
            &["run", "typecheck"],
            &[],
            CommandRisk::TestOrBuild,
            "yarn run typecheck executes package scripts",
        ),
        approve(
            "cargo",
            &["test"],
            &[],
            CommandRisk::TestOrBuild,
            "cargo test is a test/build command",
        ),
        approve(
            "go",
            &["test"],
            &[],
            CommandRisk::TestOrBuild,
            "go test is a test/build command",
        ),
        approve(
            "dotnet",
            &["test"],
            &[],
            CommandRisk::TestOrBuild,
            "dotnet test is a test/build command",
        ),
        approve(
            "pytest",
            &[],
            &[],
            CommandRisk::TestOrBuild,
            "pytest is a test command",
        ),
    ]
}

fn block(program: &str, prefix: &[&str], contains: &[&str], reason: &str) -> CommandRule {
    CommandRule {
        program: program.into(),
        args_prefix: prefix.iter().map(|s| s.to_string()).collect(),
        args_contains: contains.iter().map(|s| s.to_string()).collect(),
        action: RuleAction::Block,
        reason: reason.into(),
    }
}

fn approve(
    program: &str,
    prefix: &[&str],
    contains: &[&str],
    risk: CommandRisk,
    reason: &str,
) -> CommandRule {
    CommandRule {
        program: program.into(),
        args_prefix: prefix.iter().map(|s| s.to_string()).collect(),
        args_contains: contains.iter().map(|s| s.to_string()).collect(),
        action: RuleAction::RequireApproval(risk),
        reason: reason.into(),
    }
}

#[derive(Debug, Error)]
pub enum CommandGuardError {
    #[error("empty argv")]
    EmptyArgv,
    #[error("program is blocked: {0}")]
    Blocked(String),
    #[error("program is not allowlisted: {0}")]
    NotAllowed(String),
}

pub fn personal_terminal_policy() -> CommandPolicy {
    CommandPolicy {
        require_explicit_rule: true,
        ..CommandPolicy::default()
    }
}

pub fn classify(
    req: &CommandRequest,
    policy: &CommandPolicy,
) -> Result<CommandRisk, CommandGuardError> {
    let program = req.argv.first().ok_or(CommandGuardError::EmptyArgv)?;
    let program_norm = normalize(program);

    if policy
        .deny_programs
        .iter()
        .any(|p| normalize(p) == program_norm)
    {
        return Err(CommandGuardError::Blocked(program_norm));
    }

    for rule in &policy.rules {
        if rule.matches(&req.argv) {
            return match &rule.action {
                RuleAction::Block => Err(CommandGuardError::Blocked(format!(
                    "{} ({})",
                    program_norm, rule.reason
                ))),
                RuleAction::Allow => Ok(CommandRisk::ReadOnly),
                RuleAction::RequireApproval(risk) => Ok(risk.clone()),
            };
        }
    }

    if !policy
        .allow_programs
        .iter()
        .any(|p| normalize(p) == program_norm)
    {
        return Err(CommandGuardError::NotAllowed(program_norm));
    }

    if policy.require_explicit_rule {
        return Err(CommandGuardError::NotAllowed(format!(
            "{} (no explicit safe rule matched)",
            program_norm
        )));
    }

    if req.requires_network {
        return Ok(CommandRisk::Network);
    }
    if req.may_modify_files {
        return Ok(CommandRisk::ModifiesRepo);
    }
    if looks_test_or_build_structured(&req.argv) {
        return Ok(CommandRisk::TestOrBuild);
    }

    Ok(CommandRisk::ReadOnly)
}

fn normalize_args(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for raw in args {
        let arg = normalize(raw);
        if arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 2 {
            // Expand combined short flags: -fd => -f, -d; -fxd => -f, -x, -d.
            // Preserve the original too so exact rules can still match.
            out.push(arg.clone());
            for ch in arg.trim_start_matches('-').chars() {
                out.push(format!("-{}", ch));
            }
        } else {
            out.push(arg);
        }
    }
    out
}

fn looks_test_or_build_structured(argv: &[String]) -> bool {
    argv.iter().any(|arg| {
        let arg = normalize(arg);
        arg == "test"
            || arg == "lint"
            || arg == "build"
            || arg.ends_with(":test")
            || arg.ends_with(":lint")
    })
}

fn normalize(s: &str) -> String {
    s.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(argv: &[&str]) -> CommandRequest {
        CommandRequest {
            argv: argv.iter().map(|s| s.to_string()).collect(),
            cwd: ".".into(),
            requires_network: false,
            may_modify_files: false,
        }
    }

    #[test]
    fn blocks_git_push_structurally() {
        let policy = CommandPolicy::default();
        let result = classify(&req(&["git", "push"]), &policy);
        assert!(matches!(result, Err(CommandGuardError::Blocked(_))));
    }

    #[test]
    fn blocks_git_reset_hard_structurally() {
        let policy = CommandPolicy::default();
        let result = classify(&req(&["git", "reset", "--hard"]), &policy);
        assert!(matches!(result, Err(CommandGuardError::Blocked(_))));
    }

    #[test]
    fn classifies_npm_install_as_network() {
        let policy = CommandPolicy::default();
        assert_eq!(
            classify(&req(&["npm", "install"]), &policy).unwrap(),
            CommandRisk::Network
        );
    }

    #[test]
    fn blocks_shell_entrypoints() {
        let policy = CommandPolicy::default();
        for argv in [
            ["bash", "-c", "echo hi"],
            ["sh", "-c", "echo hi"],
            ["powershell", "-Command", "Get-ChildItem"],
        ] {
            let result = classify(&req(&argv), &policy);
            assert!(matches!(result, Err(CommandGuardError::Blocked(_))));
        }
    }

    #[test]
    fn classifies_test_scripts_as_test_or_build() {
        let policy = CommandPolicy::default();
        assert_eq!(
            classify(&req(&["pnpm", "test", "auth"]), &policy).unwrap(),
            CommandRisk::TestOrBuild
        );
        assert_eq!(
            classify(&req(&["cargo", "test"]), &policy).unwrap(),
            CommandRisk::TestOrBuild
        );
    }

    #[test]
    fn blocks_git_clean_split_and_combined_flags() {
        let policy = CommandPolicy::default();
        let cases: Vec<Vec<&str>> = vec![
            vec!["git", "clean", "-fd"],
            vec!["git", "clean", "-df"],
            vec!["git", "clean", "-f", "-d"],
            vec!["git", "clean", "-d", "-f"],
            vec!["git", "clean", "-fxd"],
            vec!["git", "clean", "-x", "-d", "-f"],
        ];
        for argv in cases {
            let result = classify(&req(&argv), &policy);
            assert!(
                matches!(result, Err(CommandGuardError::Blocked(_))),
                "expected block for {:?}, got {:?}",
                argv,
                result
            );
        }
    }

    #[test]
    fn package_scripts_still_require_approval() {
        let policy = CommandPolicy::default();
        assert_eq!(
            classify(&req(&["npm", "run", "test"]), &policy).unwrap(),
            CommandRisk::TestOrBuild
        );
        assert_eq!(
            classify(&req(&["pnpm", "test"]), &policy).unwrap(),
            CommandRisk::TestOrBuild
        );
    }

    #[test]
    fn arbitrary_node_execution_is_blocked() {
        let risk = classify(
            &req(&["node", "scripts/dangerous.js"]),
            &CommandPolicy::default(),
        );
        assert!(matches!(risk, Err(CommandGuardError::Blocked(_))));
    }

    #[test]
    fn strict_personal_policy_blocks_unmatched_allowlisted_programs() {
        let policy = personal_terminal_policy();
        let risk = classify(&req(&["git", "branch"]), &policy);
        assert!(matches!(risk, Err(CommandGuardError::NotAllowed(_))));
    }

    #[test]
    fn strict_personal_policy_allows_only_explicit_safe_git() {
        let policy = personal_terminal_policy();
        assert_eq!(
            classify(&req(&["git", "status"]), &policy).unwrap(),
            CommandRisk::ReadOnly
        );
        assert_eq!(
            classify(&req(&["git", "diff", "--", "src/main.rs"]), &policy).unwrap(),
            CommandRisk::ReadOnly
        );
        assert_eq!(
            classify(&req(&["git", "log", "--oneline"]), &policy).unwrap(),
            CommandRisk::ReadOnly
        );
    }

    #[test]
    fn strict_personal_policy_blocks_dangerous_git_even_without_flags() {
        let policy = personal_terminal_policy();
        let cases: Vec<Vec<&str>> = vec![
            vec!["git", "checkout", "main"],
            vec!["git", "restore", "."],
            vec!["git", "add", "."],
            vec!["git", "commit", "-m", "x"],
            vec!["git", "merge", "feature"],
            vec!["git", "rebase", "main"],
            vec!["git", "pull"],
            vec!["git", "fetch"],
            vec!["git", "stash"],
        ];
        for argv in cases {
            let result = classify(&req(&argv), &policy);
            assert!(
                matches!(result, Err(CommandGuardError::Blocked(_))),
                "expected block for {:?}, got {:?}",
                argv,
                result
            );
        }
    }

    #[test]
    fn strict_personal_policy_does_not_let_user_flags_downgrade_risk() {
        let policy = personal_terminal_policy();
        let mut install = req(&["pnpm", "install"]);
        install.requires_network = false;
        install.may_modify_files = false;
        assert_eq!(classify(&install, &policy).unwrap(), CommandRisk::Network);

        let mut unknown = req(&["pnpm", "exec", "ts-node", "script.ts"]);
        unknown.requires_network = false;
        unknown.may_modify_files = false;
        assert!(matches!(
            classify(&unknown, &policy),
            Err(CommandGuardError::NotAllowed(_))
        ));
    }

    #[test]
    fn strict_personal_policy_allows_core_test_commands() {
        let policy = personal_terminal_policy();
        for argv in [
            vec!["pnpm", "test"],
            vec!["pnpm", "run", "test"],
            vec!["pnpm", "run", "lint"],
            vec!["pnpm", "run", "build"],
            vec!["npm", "test"],
            vec!["npm", "run", "typecheck"],
            vec!["yarn", "test"],
            vec!["cargo", "test"],
            vec!["go", "test", "./..."],
            vec!["pytest"],
            vec!["dotnet", "test"],
        ] {
            assert_eq!(
                classify(&req(&argv), &policy).unwrap(),
                CommandRisk::TestOrBuild,
                "{:?}",
                argv
            );
        }
    }
}
