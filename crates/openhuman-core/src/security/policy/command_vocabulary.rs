//! The command vocabulary behind shell classification: which bases only read,
//! reach the network, install on the host, are destructive or run arbitrary
//! code, and which only launch another command. Restored from TinyBox's
//! classifier (tinybox c2e34c5bc2, `tinybox-core/src/shell/{classify,executor,
//! env_guard}.rs`) after TinyBox stopped classifying commands and the host's
//! interim copy kept only a fraction of the lists.
//!
//! On top of that vocabulary, a segment is classified by the command it really
//! runs: leading `NAME=value` assignments and launchers (`env`, `timeout`,
//! `nohup`, `nice`, `command`, `exec`, `time`, `stdbuf`, `xargs`) are looked
//! through, and quotes inside a command word are removed the way the shell
//! removes them (`c''url` is `curl`).

use crate::security::policy::CommandClass;

/// Provably read-only command bases (cross-platform union). A base **not** in
/// this set — and not a recognized network/destructive/executor command, nor a
/// read-only verb of git/npm/cargo — falls through to [`CommandClass::Write`]
/// (the classifier is fail-closed). Conservative on purpose: anything that can
/// write a file under a common flag is intentionally omitted (`sort -o`, `tee`).
const READ_ONLY_BASES: &[&str] = &[
    // POSIX inspection / read-only coreutils
    "ls",
    "cat",
    "pwd",
    "echo",
    "printf",
    "wc",
    "head",
    "tail",
    "date",
    "grep",
    "egrep",
    "fgrep",
    "rg",
    "which",
    "whoami",
    "id",
    "hostname",
    "uname",
    "printenv",
    "stat",
    "file",
    "du",
    "df",
    "tree",
    "realpath",
    "readlink",
    "dirname",
    "basename",
    "cmp",
    "true",
    "false",
    "sleep",
    "seq",
    "tty",
    "groups",
    "locale",
    "ps",
    "top",
    "free",
    "uptime",
    "lsblk",
    "lscpu",
    "cut",
    // NOTE: OS-native launchers (`open`, `xdg-open`, `start`) are deliberately
    // NOT in the read-only set. `classify_command` only sees the base command,
    // not its args, and these launchers can open arbitrary `https://` URLs and
    // custom URI handlers — i.e. trigger outbound network / system actions — so
    // treating them as `Read` (no approval) is too broad. App launching now
    // goes through the dedicated `launch_app` tool, which is scoped to named
    // applications only and carries no shell-arg ambiguity.
    // Windows cmd / PowerShell read verbs + common aliases
    "dir",
    "type",
    "where",
    "whereis",
    "get-childitem",
    "gci",
    "get-content",
    "gc",
    "get-location",
    "gl",
    "select-string",
    "sls",
    "measure-object",
    "get-item",
    "gi",
    "test-path",
    "resolve-path",
    "get-command",
    "gcm",
    "get-process",
];

/// Commands that reach the network. Always-ask in every acting tier.
const NETWORK_BASES: &[&str] = &[
    "curl",
    "wget",
    "ssh",
    "scp",
    "sftp",
    "rsync",
    "nc",
    "ncat",
    "netcat",
    "telnet",
    "ftp",
    "tftp",
    "socat",
    // Windows / PowerShell
    "invoke-webrequest",
    "iwr",
    "invoke-restmethod",
    "irm",
    "start-bitstransfer",
    "bitsadmin",
];

/// Catastrophic / irreversible / privilege / system-control bases. Always-ask
/// in every acting tier (Full included). Coarse on the broad Windows verbs
/// (`reg`/`net`/`sc`) — over-prompting there is the safe default.
const DESTRUCTIVE_BASES: &[&str] = &[
    // POSIX privilege / disk / system-control
    "sudo",
    "su",
    "doas",
    "dd",
    "mkfs",
    "fdisk",
    "sfdisk",
    "parted",
    "wipefs",
    "shred",
    "shutdown",
    "reboot",
    "halt",
    "poweroff",
    "init",
    "telinit",
    "mount",
    "umount",
    "swapoff",
    "iptables",
    "ip6tables",
    "nft",
    "ufw",
    "firewall-cmd",
    "useradd",
    "userdel",
    "usermod",
    "groupadd",
    "groupdel",
    "passwd",
    "chpasswd",
    "visudo",
    "modprobe",
    "insmod",
    "rmmod",
    // Windows / PowerShell
    "format",
    "diskpart",
    "bcdedit",
    "takeown",
    "cipher",
    "vssadmin",
    "reg",
    "regedit",
    "runas",
    "sc",
    "net",
    "set-executionpolicy",
    "stop-computer",
    "restart-computer",
    "clear-disk",
    "format-volume",
    "remove-partition",
    "disable-computerrestore",
];

/// Git subcommands that only read repository state. Anything else — including
/// `commit`/`push`/`branch`/`config`/unknown/bare `git` — is fail-closed to
/// `Write`.
const GIT_READ_VERBS: &[&str] = &[
    "status",
    "log",
    "diff",
    "show",
    "remote",
    "describe",
    "blame",
    "ls-files",
    "ls-tree",
    "rev-parse",
    "cat-file",
    "shortlog",
    "reflog",
    "rev-list",
    "name-rev",
    "var",
    "check-ignore",
    "check-attr",
    "verify-commit",
    "count-objects",
    "fsck",
    "whatchanged",
    "grep",
    "version",
    "help",
];

/// npm/pnpm/yarn read-only subcommands. `install`/`run`/`test`/`exec` (which
/// run arbitrary scripts) and unknown verbs are fail-closed to `Write`.
const NODE_PKG_READ_VERBS: &[&str] = &[
    "ls", "list", "view", "info", "outdated", "ping", "whoami", "help", "why", "audit", "doctor",
];

/// cargo read-only subcommands. `build`/`run`/`test`/`check` compile and may
/// run build scripts, so they are fail-closed to `Write`.
const CARGO_READ_VERBS: &[&str] = &["tree", "metadata", "search", "info", "version", "help"];

/// Environment variable names that can trigger arbitrary command execution
/// when supplied as a leading inline assignment on an otherwise-allowed
/// command. Each name here is either a hook variable that a downstream tool
/// will spawn as a subprocess (`GIT_PAGER`, `GIT_SSH_COMMAND`, `EDITOR`,
/// `LESS`/`LESSOPEN`, `MANPAGER`, `BROWSER`, `BAT_PAGER`), a runtime
/// configuration knob that affects how Python or the shell evaluate user
/// input (`PYTHONSTARTUP`, `BASH_ENV`, `ENV`, `PROMPT_COMMAND`), or a loader
/// override that lets an attacker inject a library into the next process
/// (`LD_PRELOAD`, `LD_LIBRARY_PATH`, `LD_AUDIT`, `DYLD_INSERT_LIBRARIES`,
/// `DYLD_LIBRARY_PATH`, `DYLD_FORCE_FLAT_NAMESPACE`).
///
/// `PATH` and `SHELL` are listed so an inline override cannot redirect
/// resolution of any allowed binary to an attacker-controlled path. `IFS`
/// is listed because the shell uses it for word splitting and a malicious
/// value can hide command boundaries from later parsers.
const DANGEROUS_ENV_NAMES: &[&str] = &[
    "BASH_ENV",
    "BAT_PAGER",
    "BROWSER",
    "DYLD_FORCE_FLAT_NAMESPACE",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    "EDITOR",
    "ENV",
    "GIT_EDITOR",
    "GIT_EXTERNAL_DIFF",
    "GIT_EXTERNAL_FILTER",
    "GIT_PAGER",
    "GIT_SSH",
    "GIT_SSH_COMMAND",
    "IFS",
    "LD_AUDIT",
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "LESS",
    "LESSCLOSE",
    "LESSOPEN",
    "MANOPT",
    "MANPAGER",
    "PAGER",
    "PATH",
    "PROMPT_COMMAND",
    "PS1",
    "PS2",
    "PS3",
    "PS4",
    "PYTHONPATH",
    "PYTHONSTARTUP",
    "SHELL",
    "VISUAL",
];

/// Bases that run arbitrary code: interpreters, shells, `xargs`, `env`, JS
/// runtimes and Windows launchers.
pub(super) fn is_command_executor(base: &str) -> bool {
    let is_python = ["python", "pythonw"].iter().any(|prefix| {
        base.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(|c: char| c.is_ascii_digit()))
    });
    is_python
        || matches!(
            base,
            "xargs"
                | "awk"
                | "gawk"
                | "mawk"
                | "nawk"
                | "perl"
                | "ruby"
                | "bash"
                | "sh"
                | "dash"
                | "zsh"
                | "ksh"
                | "fish"
                | "env"
                | "node"
                | "nodejs"
                | "deno"
                | "bun"
                | "iex"
                | "invoke-expression"
                | "cmd"
                | "pwsh"
                | "powershell"
                | "wscript"
                | "cscript"
                | "mshta"
                | "rundll32"
                | "start-process"
        )
}

/// The program name a shell runs for `word`: quotes removed (the shell joins
/// `c''url` into `curl`), basename only, lowercase, `.exe` stripped.
pub(super) fn normalized_command_name(word: &str) -> String {
    let unquoted = word.replace(['\'', '"'], "");
    let base = unquoted.rsplit(['/', '\\']).next().unwrap_or_default();
    let base = base.to_ascii_lowercase();
    base.strip_suffix(".exe").unwrap_or(&base).to_owned()
}

/// A shell variable assignment (`NAME=value`) rather than a command word.
pub(super) fn is_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// Whether any assignment in the segment sets a variable that can redirect
/// what runs (a pager, editor, loader override, `PATH`, …).
pub(super) fn has_dangerous_env_assignment(words: &[&str]) -> bool {
    words.iter().any(|word| {
        is_assignment(word)
            && word.split_once('=').is_some_and(|(name, _)| {
                DANGEROUS_ENV_NAMES.contains(&name.to_ascii_uppercase().as_str())
            })
    })
}

/// Launchers that run their operand, with the options that take a separate
/// value (so the value is not mistaken for the command).
fn launcher_value_options(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "env" => &["-u", "--unset", "-C", "--chdir"],
        "timeout" => &["-s", "--signal", "-k", "--kill-after"],
        "nice" => &["-n", "--adjustment"],
        "stdbuf" => &["-i", "-o", "-e"],
        "xargs" => &["-I", "-n", "-P", "-L", "-s", "-d", "-E", "-a"],
        "time" => &["-f", "-o"],
        "exec" => &["-a"],
        "nohup" | "command" => &[],
        _ => return None,
    })
}

/// The command a segment really runs and its arguments, looking through
/// leading assignments and launchers: `FOO=1 env -u X timeout 5 curl u` runs
/// `curl u`. A launcher with no operand is itself the command.
pub(super) fn effective_command<'a, 'w>(words: &'a [&'w str]) -> (String, &'a [&'w str]) {
    let (mut name, mut args, mut rest) = (String::new(), &words[..0], words);
    while let Some((first, tail)) = rest.split_first() {
        if is_assignment(first) {
            rest = tail;
            continue;
        }
        name = normalized_command_name(first);
        args = tail;
        let Some(value_options) = launcher_value_options(&name) else {
            break;
        };
        let mut operand = 0;
        while let Some(word) = tail.get(operand) {
            if value_options.contains(word) {
                operand += 2;
            } else if word.starts_with('-') || is_assignment(word) {
                operand += 1;
            } else {
                break;
            }
        }
        if name == "timeout" {
            operand += 1; // the duration
        }
        rest = tail.get(operand..).unwrap_or_default();
    }
    (name, args)
}

/// Classify one live shell segment (already split on `;`, `|`, `&&`, …).
pub(super) fn classify_segment_source(source: &str) -> CommandClass {
    let words = source.split_whitespace().collect::<Vec<_>>();
    let (base, args) = effective_command(&words);
    let args = args
        .iter()
        .map(|word| word.replace(['\'', '"'], "").to_ascii_lowercase())
        .collect::<Vec<_>>();
    let joined = source.replace(['\'', '"'], "").to_ascii_lowercase();
    let class = classify_host_command(&base, &args, &joined);
    if has_dangerous_env_assignment(&words) {
        class.max(CommandClass::Write)
    } else {
        class
    }
}

/// Detect a pacman *install/upgrade* from its bundled operation flag. A
/// `-S`-family flag mutates the host only when it carries none of pacman's
/// read-only query modifiers — search (`s`), info (`i`), list (`l`), groups
/// (`g`) or print (`p`). `args` are lowercased.
fn is_pacman_install(args: &[String]) -> bool {
    args.iter().any(|a| {
        a.strip_prefix("-s")
            .is_some_and(|modifiers| !modifiers.contains(['s', 'i', 'l', 'g', 'p']))
    })
}

/// A package-manager install that mutates the host or global environment.
/// Project-local installs (`npm install` without `-g`, `cargo add`) are
/// ordinary writes and deliberately not matched. `args` are lowercased.
fn is_install_command(base: &str, args: &[String]) -> bool {
    let has = |needle: &str| args.iter().any(|a| a == needle);
    let first_is = |verb: &str| args.first().map(String::as_str) == Some(verb);
    match base {
        "pacman" => is_pacman_install(args),
        "apk" => has("add"),
        "apt" | "apt-get" | "dnf" | "yum" | "zypper" | "brew" | "snap" | "flatpak" | "winget"
        | "choco" | "scoop" => has("install"),
        "pip" | "pip3" | "pipx" | "gem" | "go" | "cargo" => first_is("install"),
        "npm" | "pnpm" => {
            (has("install") || has("i") || has("add")) && (has("-g") || has("--global"))
        }
        "yarn" => has("global"),
        _ => false,
    }
}

/// Classify one segment's command. `base` is the normalized program name;
/// `args` the lowercased remaining words; `joined` the lowercased segment for
/// pattern matching. Fail-closed: an unrecognized base is `Write`.
pub(super) fn classify_host_command(base: &str, args: &[String], joined: &str) -> CommandClass {
    if joined.contains("rm -rf /") || joined.contains("rm -fr /") || joined.contains(":(){:|:&};:")
    {
        return CommandClass::Destructive;
    }
    if DESTRUCTIVE_BASES.contains(&base) {
        return CommandClass::Destructive;
    }
    if NETWORK_BASES.contains(&base) {
        return CommandClass::Network;
    }
    if is_install_command(base, args) {
        return CommandClass::Install;
    }
    if is_command_executor(base) {
        return CommandClass::Write;
    }
    // `find` reads unless it executes, deletes, or writes its output to a
    // named file (an arbitrary-path write past the gated file tools).
    if base == "find" {
        let acts = args.iter().any(|a| {
            matches!(
                a.as_str(),
                "-exec"
                    | "-execdir"
                    | "-ok"
                    | "-okdir"
                    | "-delete"
                    | "-fprintf"
                    | "-fprint"
                    | "-fprint0"
                    | "-fls"
            )
        });
        return if acts {
            CommandClass::Write
        } else {
            CommandClass::Read
        };
    }
    if base == "git" {
        return verb_class(args, GIT_READ_VERBS);
    }
    if matches!(base, "npm" | "pnpm" | "yarn") {
        return verb_class(args, NODE_PKG_READ_VERBS);
    }
    if base == "cargo" {
        return verb_class(args, CARGO_READ_VERBS);
    }
    if READ_ONLY_BASES.contains(&base) {
        return CommandClass::Read;
    }
    CommandClass::Write
}

/// `Read` when the first subcommand word is in `read_verbs`, else `Write`.
fn verb_class(args: &[String], read_verbs: &[&str]) -> CommandClass {
    match args.first().map(String::as_str) {
        Some(verb) if read_verbs.contains(&verb) => CommandClass::Read,
        _ => CommandClass::Write,
    }
}
