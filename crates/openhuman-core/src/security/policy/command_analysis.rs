use crate::security::policy::CommandClass;

/// Host-owned interpretation of TinyBox's structural shell facts.
///
/// TinyBox reports live command segments and redirection only. OpenHuman adds
/// the policy fields below locally so command permission and autonomy remain
/// host decisions instead of bus-contract vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct CommandAnalysis {
    pub segments: Vec<CommandSegment>,
    pub hidden_execution: bool,
    pub redirection: bool,
    pub expansion: bool,
    pub tee: bool,
    pub background: bool,
    pub literal_words: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct CommandSegment {
    pub source: String,
    pub command: String,
    pub basename: String,
    pub normalized_name: String,
    pub arguments: Vec<String>,
    pub class: CommandClass,
    pub leading_env_assignment: bool,
    pub dangerous_env_prefix: bool,
    pub executor: bool,
}

impl CommandAnalysis {
    pub(crate) fn from_shell_analysis(facts: tinybox_bus::ShellAnalysis) -> Self {
        let mut analysis = Self {
            segments: Vec::new(),
            hidden_execution: false,
            redirection: facts.redirection,
            expansion: false,
            tee: false,
            background: false,
            literal_words: Vec::new(),
        };

        for source in facts.segments {
            let words = source.split_whitespace().collect::<Vec<_>>();
            let Some(raw_command) = words.first().copied() else {
                continue;
            };
            let command = raw_command.trim_matches(|c: char| matches!(c, '\'' | '"'));
            let basename = command.rsplit(['/', '\\']).next().unwrap_or(command);
            let normalized_name = basename.to_ascii_lowercase();
            let arguments = words
                .iter()
                .skip(1)
                .map(|word| {
                    word.trim_matches(|c: char| matches!(c, '\'' | '"'))
                        .to_owned()
                })
                .collect::<Vec<_>>();
            let lower = source.to_ascii_lowercase();
            let leading_env_assignment = command.contains('=')
                && command.split_once('=').is_some_and(|(name, _)| {
                    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                });
            let dangerous_env_prefix =
                words
                    .iter()
                    .take_while(|word| word.contains('='))
                    .any(|word| {
                        let name = word
                            .split('=')
                            .next()
                            .unwrap_or_default()
                            .to_ascii_uppercase();
                        matches!(
                            name.as_str(),
                            "LD_PRELOAD"
                                | "LD_LIBRARY_PATH"
                                | "DYLD_INSERT_LIBRARIES"
                                | "DYLD_LIBRARY_PATH"
                                | "PATH"
                        )
                    });
            let executor = is_command_executor_name(&normalized_name)
                || normalized_name == "env"
                    && arguments
                        .iter()
                        .any(|argument| is_command_executor_name(argument));
            let class = classify_host_command(&normalized_name, &arguments, &lower);
            analysis.hidden_execution |= lower.contains("$(")
                || lower.contains("<(")
                || lower.contains(">(")
                || source.contains('`')
                || has_background_operator(&source);
            analysis.expansion |= source.contains('$') || source.contains('`');
            analysis.background |= has_background_operator(&source);
            analysis.tee |= words
                .iter()
                .any(|word| word == &"tee" || word.ends_with("/tee"));
            analysis.literal_words.extend(
                words
                    .iter()
                    .map(|word| {
                        word.trim_matches(|c: char| matches!(c, '\'' | '"' | '<' | '>' | '&' | '|'))
                            .to_owned()
                    })
                    .filter(|word| !word.is_empty()),
            );
            analysis.segments.push(CommandSegment {
                source: source.clone(),
                command: source.clone(),
                basename: basename.to_owned(),
                normalized_name,
                arguments,
                class,
                leading_env_assignment,
                dangerous_env_prefix,
                executor,
            });
        }
        analysis
    }
}

fn has_background_operator(source: &str) -> bool {
    let bytes = source.as_bytes();
    bytes.iter().enumerate().any(|(index, byte)| {
        *byte == b'&'
            && bytes.get(index + 1) != Some(&b'&')
            && (index == 0 || bytes[index - 1] != b'&')
            && (index == 0 || bytes[index - 1] != b'>')
    })
}

pub(super) fn classify_host_command(base: &str, args: &[String], source: &str) -> CommandClass {
    if matches!(
        base,
        "sudo"
            | "su"
            | "mkfs"
            | "dd"
            | "shutdown"
            | "reboot"
            | "halt"
            | "poweroff"
            | "useradd"
            | "userdel"
            | "usermod"
            | "passwd"
            | "format"
            | "diskpart"
    ) || source.contains("rm -rf /")
        || source.contains("rm -fr /")
        || source.contains(":(){:|:&};:")
    {
        return CommandClass::Destructive;
    }
    if matches!(
        base,
        "curl"
            | "wget"
            | "ssh"
            | "scp"
            | "sftp"
            | "ftp"
            | "telnet"
            | "nc"
            | "ncat"
            | "netcat"
            | "invoke-webrequest"
            | "invoke-restmethod"
            | "iwr"
            | "irm"
    ) {
        return CommandClass::Network;
    }
    let first = args.first().map(String::as_str).unwrap_or_default();
    if base == "find"
        && args.iter().any(|argument| {
            matches!(
                argument.as_str(),
                "-delete" | "-fprint" | "-fprint0" | "-fprintf" | "-fls"
            )
        })
    {
        return CommandClass::Write;
    }
    let global_flag = args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-g" | "--global"));
    let second = args.get(1).map(String::as_str).unwrap_or_default();
    let system_install = matches!(base, "apt" | "apt-get" | "brew" | "dnf") && first == "install"
        || base == "apk" && first == "add"
        || base == "pacman" && matches!(first, "-s" | "-sy" | "-syu")
        || matches!(base, "pip" | "pip3" | "pipx" | "gem" | "cargo" | "go") && first == "install";
    let global_node_install =
        matches!(base, "npm" | "pnpm") && matches!(first, "install" | "add") && global_flag
            || base == "yarn" && first == "global" && second == "add";
    if system_install || global_node_install {
        return CommandClass::Install;
    }
    if matches!(
        base,
        "ls" | "pwd"
            | "cat"
            | "head"
            | "tail"
            | "grep"
            | "rg"
            | "find"
            | "stat"
            | "wc"
            | "sort"
            | "echo"
            | "printf"
            | "whoami"
            | "date"
            | "dir"
            | "type"
            | "get-content"
    ) || base == "git" && matches!(first, "status" | "diff" | "log" | "show" | "branch")
        || base == "cargo" && matches!(first, "check" | "test" | "tree" | "metadata")
        || matches!(base, "npm" | "pnpm" | "yarn") && matches!(first, "ls" | "list")
    {
        return CommandClass::Read;
    }
    CommandClass::Write
}

pub(super) fn is_command_executor_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    let python = ["pythonw", "python"]
        .iter()
        .any(|prefix| is_versioned_executable(name, prefix));
    python
        || matches!(
            name,
            "sh" | "bash"
                | "zsh"
                | "fish"
                | "node"
                | "ruby"
                | "perl"
                | "php"
                | "lua"
                | "awk"
                | "xargs"
        )
}

fn is_versioned_executable(name: &str, prefix: &str) -> bool {
    let Some(suffix) = name.strip_prefix(prefix) else {
        return false;
    };
    suffix.is_empty()
        || (suffix.starts_with(|character: char| character.is_ascii_digit())
            && suffix
                .chars()
                .all(|character| character.is_ascii_digit() || character == '.')
            && !suffix.ends_with('.'))
}
