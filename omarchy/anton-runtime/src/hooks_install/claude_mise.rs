//! The mise `[dotfiles]` declaration read of the D5 mise check: which config
//! files mise can load from the home, and which targets their `[dotfiles]`
//! tables declare, whatever the mode. Nothing here runs mise; the caller
//! reads each file with `mise config get -f`, which renders no templates.
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// Environment variables that move mise's config files. Each is read from
/// the installer's environment, which the mise runs also inherit.
pub(crate) const MISE_VARS: [&str; 12] = [
    "XDG_CONFIG_HOME",
    "MISE_CONFIG_DIR",
    "MISE_CONFIG_FILE",
    "MISE_CONFIG_ROOT",
    "MISE_GLOBAL_CONFIG_FILE",
    "MISE_GLOBAL_CONFIG_ROOT",
    "MISE_SYSTEM_CONFIG_DIR",
    "MISE_SYSTEM_CONFIG_FILE",
    "MISE_SYSTEM_DIR",
    "MISE_DEFAULT_CONFIG_FILENAME",
    "MISE_DEFAULT_CONFIG_FILENAMES",
    "MISE_OVERRIDE_CONFIG_FILENAMES",
];
/// More candidate files than this refuses.
const MAX_FILES: usize = 256;
/// Deeper value nesting than this refuses.
const MAX_DEPTH: usize = 64;

/// The value of `name` in `vars`, if set and not empty.
fn var<'a>(vars: &'a [(&'static str, OsString)], name: &str) -> Option<&'a OsStr> {
    vars.iter()
        .find(|(key, value)| *key == name && !value.is_empty())
        .map(|(_, value)| value.as_os_str())
}

/// Collects candidate config files, erring only on the side of too many.
struct Candidates {
    files: BTreeSet<PathBuf>,
}
impl Candidates {
    /// The entries of `dir`, or none when it does not exist. Any other
    /// failure to list an existing directory refuses.
    fn entries(dir: &Path) -> Result<Vec<(OsString, PathBuf)>, ()> {
        match std::fs::read_dir(dir) {
            Ok(entries) => entries
                .map(|entry| entry.map(|e| (e.file_name(), e.path())).map_err(|_| ()))
                .collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) if e.kind() == std::io::ErrorKind::NotADirectory => Ok(Vec::new()),
            Err(_) => Err(()),
        }
    }
    /// `path` when it is a file (symlinks followed). Absent is skipped; a
    /// path that cannot be inspected refuses.
    fn file(&mut self, path: &Path) -> Result<(), ()> {
        match std::fs::metadata(path) {
            Ok(info) if info.is_file() => {
                self.files.insert(path.to_owned());
                Ok(())
            }
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotADirectory => Ok(()),
            Err(_) => Err(()),
        }
    }
    /// Each file of `dir` whose name ends in `.toml` and passes `keep`.
    fn tomls(&mut self, dir: &Path, keep: impl Fn(&[u8]) -> bool) -> Result<(), ()> {
        for (name, path) in Self::entries(dir)? {
            let name = name.as_encoded_bytes();
            if name.ends_with(b".toml") && keep(name) {
                self.file(&path)?;
            }
        }
        Ok(())
    }
    /// A mise directory (`~/.config/mise`, `/etc/mise`, `<dir>/.mise`): every
    /// TOML file in it, and its `conf.d` fragments.
    fn mise_dir(&mut self, dir: &Path) -> Result<(), ()> {
        self.tomls(dir, |_| true)?;
        let conf = dir.join("conf.d");
        self.tomls(&conf, |name| !name.starts_with(b"."))?;
        for (name, path) in Self::entries(&conf)? {
            if !name.as_encoded_bytes().starts_with(b".")
                && std::fs::metadata(&path).is_ok_and(|m| m.is_dir())
            {
                self.tomls(&path, |_| true)?;
            }
        }
        Ok(())
    }
    /// A directory of the project hierarchy: `mise*.toml`, `.mise*.toml` and
    /// the legacy `.rtx*.toml` in it, the same names in its `.config`, and
    /// the mise directories `mise`, `.mise`, `.config/mise`, `.config/.mise`.
    fn project_dir(&mut self, dir: &Path, names: &[OsString]) -> Result<(), ()> {
        let mise_name = |name: &[u8]| {
            [b"mise".as_slice(), b".mise", b".rtx"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
        };
        self.tomls(dir, mise_name)?;
        self.tomls(&dir.join(".config"), mise_name)?;
        for sub in ["mise", ".mise", ".config/mise", ".config/.mise"] {
            self.mise_dir(&dir.join(sub))?;
        }
        for name in names {
            self.file(&dir.join(name))?;
        }
        Ok(())
    }
}

/// `value` as a path, taken relative to `home` (where mise runs) when it is
/// not absolute.
fn at_home(value: &OsStr, home: &Path) -> PathBuf {
    home.join(value)
}

/// Every config file mise could load when run in `home`, over-inclusive by
/// design: the home and each ancestor, the global and system directories
/// (moved by `vars`), and any file `vars` names. `None` refuses: a directory
/// or file that exists but cannot be inspected, or too many files.
pub(crate) fn candidates(home: &Path, vars: &[(&'static str, OsString)]) -> Option<Vec<PathBuf>> {
    let mut found = Candidates {
        files: BTreeSet::new(),
    };
    let names: Vec<OsString> = [
        "MISE_DEFAULT_CONFIG_FILENAME",
        "MISE_DEFAULT_CONFIG_FILENAMES",
        "MISE_OVERRIDE_CONFIG_FILENAMES",
    ]
    .iter()
    .filter_map(|name| var(vars, name))
    .flat_map(|value| {
        value
            .as_encoded_bytes()
            .split(|b| *b == b':' || *b == b',')
            .filter(|part| !part.is_empty())
            .map(|part| OsStr::from_bytes(part).to_owned())
            .collect::<Vec<_>>()
    })
    .collect();
    let walked: Result<(), ()> = (|| {
        for dir in home.ancestors() {
            found.project_dir(dir, &names)?;
        }
        for root in ["MISE_CONFIG_ROOT", "MISE_GLOBAL_CONFIG_ROOT"] {
            if let Some(value) = var(vars, root) {
                found.project_dir(&at_home(value, home), &names)?;
            }
        }
        let mut global = vec![home.join(".config/mise")];
        if let Some(value) = var(vars, "XDG_CONFIG_HOME") {
            let xdg = at_home(value, home);
            found.tomls(&xdg, |name| name.starts_with(b"mise"))?;
            global.push(xdg.join("mise"));
        }
        if let Some(value) = var(vars, "MISE_CONFIG_DIR") {
            global.push(at_home(value, home));
        }
        match var(vars, "MISE_SYSTEM_CONFIG_DIR") {
            Some(value) => global.push(at_home(value, home)),
            None => global.push(PathBuf::from("/etc/mise")),
        }
        if let Some(value) = var(vars, "MISE_SYSTEM_DIR") {
            global.push(at_home(value, home));
        }
        for dir in global {
            found.mise_dir(&dir)?;
        }
        for file in [
            "MISE_CONFIG_FILE",
            "MISE_GLOBAL_CONFIG_FILE",
            "MISE_SYSTEM_CONFIG_FILE",
        ] {
            if let Some(value) = var(vars, file) {
                found.file(&at_home(value, home))?;
            }
        }
        Ok(())
    })();
    walked.ok()?;
    (found.files.len() <= MAX_FILES).then(|| found.files.into_iter().collect())
}

/// A minimal TOML reader over `mise config get -f` output, which only finds
/// the keys `[dotfiles]` declares. Values are skipped, not decoded; anything
/// it does not understand is an error, never a guess.
struct Reader<'a> {
    s: &'a [u8],
    i: usize,
}
impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn starts(&self, text: &[u8]) -> bool {
        self.s[self.i..].starts_with(text)
    }
    fn spaces(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.i += 1;
        }
    }
    fn comment(&mut self) {
        if self.peek() == Some(b'#') {
            while !matches!(self.peek(), None | Some(b'\n')) {
                self.i += 1;
            }
        }
    }
    /// Spaces, newlines and comments.
    fn blank(&mut self) {
        loop {
            self.spaces();
            self.comment();
            match self.peek() {
                Some(b'\n') => self.i += 1,
                Some(b'\r') if self.s.get(self.i + 1) == Some(&b'\n') => self.i += 2,
                _ => return,
            }
        }
    }
    /// The end of a line: spaces, an optional comment, then a newline or the
    /// end of the text.
    fn end_of_line(&mut self) -> Option<()> {
        self.spaces();
        self.comment();
        match self.peek() {
            None => Some(()),
            Some(b'\n') => {
                self.i += 1;
                Some(())
            }
            Some(b'\r') if self.s.get(self.i + 1) == Some(&b'\n') => {
                self.i += 2;
                Some(())
            }
            _ => None,
        }
    }
    fn expect(&mut self, byte: u8) -> Option<()> {
        (self.peek() == Some(byte)).then(|| self.i += 1)
    }
    /// One key: bare, basic (with its escapes decoded) or literal.
    fn key(&mut self) -> Option<String> {
        match self.peek()? {
            b'"' => {
                self.i += 1;
                let mut out = Vec::new();
                loop {
                    match self.peek()? {
                        b'"' => {
                            self.i += 1;
                            return String::from_utf8(out).ok();
                        }
                        b'\n' | b'\r' => return None,
                        b'\\' => {
                            self.i += 1;
                            let escaped = self.peek()?;
                            self.i += 1;
                            let simple = match escaped {
                                b'"' => Some(b'"'),
                                b'\\' => Some(b'\\'),
                                b'b' => Some(8),
                                b't' => Some(b'\t'),
                                b'n' => Some(b'\n'),
                                b'f' => Some(12),
                                b'r' => Some(b'\r'),
                                b'e' => Some(27),
                                _ => None,
                            };
                            if let Some(byte) = simple {
                                out.push(byte);
                                continue;
                            }
                            let width = match escaped {
                                b'u' => 4,
                                b'U' => 8,
                                _ => return None,
                            };
                            let hex =
                                std::str::from_utf8(self.s.get(self.i..self.i + width)?).ok()?;
                            let code = u32::from_str_radix(hex, 16).ok()?;
                            self.i += width;
                            let mut buffer = [0u8; 4];
                            out.extend_from_slice(
                                char::from_u32(code)?.encode_utf8(&mut buffer).as_bytes(),
                            );
                        }
                        byte => {
                            out.push(byte);
                            self.i += 1;
                        }
                    }
                }
            }
            b'\'' => {
                self.i += 1;
                let start = self.i;
                loop {
                    match self.peek()? {
                        b'\'' => break,
                        b'\n' | b'\r' => return None,
                        _ => self.i += 1,
                    }
                }
                let key = String::from_utf8(self.s[start..self.i].to_vec()).ok();
                self.i += 1;
                key
            }
            _ => {
                let start = self.i;
                while matches!(
                    self.peek(),
                    Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-')
                ) {
                    self.i += 1;
                }
                (self.i > start).then(|| String::from_utf8(self.s[start..self.i].to_vec()).ok())?
            }
        }
    }
    /// A dotted key path.
    fn path(&mut self) -> Option<Vec<String>> {
        let mut path = Vec::new();
        loop {
            self.spaces();
            path.push(self.key()?);
            self.spaces();
            if self.peek() != Some(b'.') {
                return Some(path);
            }
            self.i += 1;
        }
    }
    /// A multi-line string closed by `quote` three times, which may be
    /// followed by up to two more quotes of its content.
    fn multiline(&mut self, quote: u8, escapes: bool) -> Option<()> {
        self.i += 3;
        loop {
            let byte = self.peek()?;
            if escapes && byte == b'\\' {
                self.i += 2;
                continue;
            }
            if byte == quote && self.starts(&[quote; 3]) {
                let mut run = 0;
                while self.peek() == Some(quote) {
                    run += 1;
                    self.i += 1;
                }
                return (run <= 5).then_some(());
            }
            self.i += 1;
        }
    }
    /// Skips one value.
    fn value(&mut self, depth: usize) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        match self.peek()? {
            b'"' if self.starts(b"\"\"\"") => self.multiline(b'"', true),
            b'\'' if self.starts(b"'''") => self.multiline(b'\'', false),
            b'"' | b'\'' => self.key().map(drop),
            b'[' => {
                self.i += 1;
                loop {
                    self.blank();
                    if self.peek()? == b']' {
                        self.i += 1;
                        return Some(());
                    }
                    self.value(depth + 1)?;
                    self.blank();
                    match self.peek()? {
                        b',' => self.i += 1,
                        b']' => {}
                        _ => return None,
                    }
                }
            }
            b'{' => {
                self.i += 1;
                loop {
                    self.blank();
                    if self.peek()? == b'}' {
                        self.i += 1;
                        return Some(());
                    }
                    self.path()?;
                    self.expect(b'=')?;
                    self.spaces();
                    self.value(depth + 1)?;
                    self.blank();
                    match self.peek()? {
                        b',' => self.i += 1,
                        b'}' => {}
                        _ => return None,
                    }
                }
            }
            _ => {
                let start = self.i;
                while !matches!(
                    self.peek(),
                    None | Some(b',' | b']' | b'}' | b'#' | b'\n' | b'\r')
                ) {
                    if matches!(self.peek(), Some(b'"' | b'\'' | b'[' | b'{' | b'=')) {
                        return None;
                    }
                    self.i += 1;
                }
                (!self.s[start..self.i].trim_ascii().is_empty()).then_some(())
            }
        }
    }
}

/// Every target the `[dotfiles]` table of the TOML `text` declares, in
/// whatever form: `[dotfiles]` keys, `[dotfiles."<target>"]` tables and
/// top-level `dotfiles."<target>"` keys, and arrays of tables under a
/// target such as `[[dotfiles."<target>".edits]]`. `None` refuses: text this
/// reader does not understand, `[[dotfiles]]`, or a top-level
/// `dotfiles = …` value.
pub(crate) fn dotfiles_targets(text: &[u8]) -> Option<BTreeSet<String>> {
    let mut reader = Reader { s: text, i: 0 };
    let mut table: Vec<String> = Vec::new();
    let mut targets = BTreeSet::new();
    fn record(targets: &mut BTreeSet<String>, path: &[String]) -> Option<()> {
        match path {
            [first, target, ..] if first == "dotfiles" => {
                targets.insert(target.clone());
                Some(())
            }
            [first] if first == "dotfiles" => None,
            _ => Some(()),
        }
    }
    loop {
        reader.blank();
        let Some(byte) = reader.peek() else {
            return Some(targets);
        };
        if byte == b'[' {
            reader.i += 1;
            let array = reader.peek() == Some(b'[');
            if array {
                reader.i += 1;
            }
            table = reader.path()?;
            reader.expect(b']')?;
            if array {
                reader.expect(b']')?;
            }
            // `[dotfiles]` itself names no target; `[[dotfiles]]` refuses
            // below. `[dotfiles."<target>"]` and its subtables, and arrays
            // of them such as `[[dotfiles."<target>".edits]]`, name one.
            if array || table.len() >= 2 {
                record(&mut targets, &table)?;
            }
        } else {
            let key = reader.path()?;
            reader.expect(b'=')?;
            reader.spaces();
            reader.value(0)?;
            let full: Vec<String> = table.iter().cloned().chain(key).collect();
            record(&mut targets, &full)?;
        }
        reader.end_of_line()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets(text: &str) -> Option<Vec<String>> {
        dotfiles_targets(text.as_bytes()).map(|set| set.into_iter().collect())
    }

    #[test]
    fn reads_every_declared_target_whatever_the_mode() {
        // The shape `mise config get -f` prints, here for copy, template,
        // link and track entries, a plain string entry and a dotted key.
        let text = r#"min_version = "2026.1.0"
dotfiles."~/.top" = "x"

[dotfiles]
"~/.bashrc" = "dot_bashrc"
plain = { mode = "copy" }

[dotfiles."~/.claude/skills"]
mode = "copy"

[dotfiles."~/.config/a b.toml"]
mode = "template"
source = "x.tmpl"

[dotfiles.'~/.gitconfig']
mode = "link"

[dotfiles."~/.zsh\u0072c".edit]
block = """
[dotfiles."~/.not-a-target"]
"""

[[dotfiles."~/.profile".edits]]
line = "export A=1"
"#;
        assert_eq!(
            targets(text).unwrap(),
            [
                "plain",
                "~/.bashrc",
                "~/.claude/skills",
                "~/.config/a b.toml",
                "~/.gitconfig",
                "~/.profile",
                "~/.top",
                "~/.zshrc"
            ]
        );
    }

    #[test]
    fn skips_values_and_header_like_lines_inside_strings() {
        let text = r#"[tools]
node = ["20", "22"]
python = { version = "3.12", os = ["linux",
  "macos"] }

[tasks.check]
run = """
[ -f x ] && echo '[dotfiles]'
\"""
"""
depends = [
  "a", # comment
  'b',
]
wait = 1979-05-27 07:32:00
alias = '''
[dotfiles]
'''

[env]
A = "{{ exec(command='true') }}"
"#;
        assert_eq!(targets(text).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn refuses_text_it_does_not_understand() {
        for text in [
            "dotfiles = { \"~/.x\" = \"y\" }\n",
            "[[dotfiles]]\nx = 1\n",
            "[dotfiles\n",
            "[dotfiles]\n\"~/.x = 1\n",
            "[dotfiles]\n\"\\q\" = 1\n",
            "[dotfiles]\nx = \"\"\"never closed\n",
            "[dotfiles]\nx = [1, 2\n",
            "[dotfiles]\nx = \n",
            "[dotfiles]\nx = 1 y = 2\n",
            "[dotfiles]\nx\n",
            "{\"entries\":[]}",
        ] {
            assert!(targets(text).is_none(), "{text:?}");
        }
        let deep = format!("x = {}{}\n", "[".repeat(100), "]".repeat(100));
        assert!(targets(&deep).is_none());
        assert_eq!(targets("").unwrap(), Vec::<String>::new());
    }
}
