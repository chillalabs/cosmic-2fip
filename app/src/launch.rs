//! Launching files with desktop applications: the default one (`xdg-open`)
//! or one picked from the "Open With" dialog.

use std::path::{Path, PathBuf};

use cosmic::desktop::{fde, DesktopEntryData, IconSourceExt};
use cosmic::widget::icon;

/// Launches `path` with whatever application the desktop has associated with
/// it (via `xdg-open`), same as double-clicking a file in any other Linux
/// file manager. If nothing is associated, the desktop's own portal shows the
/// "choose an application" prompt - we don't need to build that ourselves.
/// Fire-and-forget: we don't wait for the launched app to exit. The app's
/// own console output is discarded so it doesn't flood our terminal (the
/// "Open With" path does the same, via libcosmic).
pub fn open_with_default_app(path: &Path) {
    use std::process::Stdio;
    // On the host in a Flatpak, so the system's default apps are used.
    let spawned = fs_ops::sandbox::host_command("xdg-open")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match spawned {
        // `xdg-open` exits as soon as it has handed the file off; reap it in
        // the background so it doesn't linger as a zombie process.
        Ok(mut child) => {
            std::thread::spawn(move || child.wait());
        }
        Err(err) => eprintln!("failed to open {}: {err}", path.display()),
    }
}

/// Opens the user's default terminal with `dir` as its working directory.
/// Fire-and-forget, output discarded, like [`open_with_default_app`].
pub fn open_terminal(dir: &Path) {
    use std::process::Stdio;
    let Some(mut args) = shlex::split(&default_terminal()).filter(|args| !args.is_empty()) else {
        eprintln!("no terminal configured");
        return;
    };
    let program = args.remove(0);
    let spawned = fs_ops::sandbox::host_command_in(&program, dir)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match spawned {
        // A terminal stays open as long as the user uses it; wait for it in
        // the background so it doesn't linger as a zombie once closed.
        Ok(mut child) => {
            std::thread::spawn(move || child.wait());
        }
        Err(err) => eprintln!("failed to start terminal {program}: {err}"),
    }
}

/// The terminal command COSMIC uses for its "Terminal" shortcut (the user's
/// choice in COSMIC Settings, else the system default), then `$TERMINAL`,
/// then the Debian/Ubuntu `x-terminal-emulator`, then `cosmic-term`.
fn default_terminal() -> String {
    let relative = "cosmic/com.system76.CosmicSettings.Shortcuts/v1/system_actions";
    let candidates = [
        fs_ops::sandbox::config_home().join(relative),
        fs_ops::sandbox::host_path(&Path::new("/usr/share").join(relative)),
    ];
    for file in candidates {
        if let Some(terminal) = std::fs::read_to_string(file)
            .ok()
            .and_then(|contents| terminal_from_system_actions(&contents))
        {
            return terminal;
        }
    }
    if let Some(terminal) = std::env::var("TERMINAL").ok().filter(|t| !t.is_empty()) {
        return terminal;
    }
    if fs_ops::sandbox::host_program_exists("x-terminal-emulator") {
        return "x-terminal-emulator".to_string();
    }
    "cosmic-term".to_string()
}

/// Extracts `Terminal: "..."` from COSMIC's `system_actions` (RON) file.
fn terminal_from_system_actions(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let value = line.trim().strip_prefix("Terminal:")?.trim();
        let value = value
            .trim_end_matches(',')
            .trim()
            .strip_prefix('"')?
            .strip_suffix('"')?;
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// How F3 / F4 open files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    /// F3: the file's default app (images: the image viewer).
    View,
    /// F4: the text editor, whatever the file type (images: the image viewer,
    /// since a text editor can't show them).
    Edit,
}

/// Opens each file (folders are skipped) for viewing or editing.
pub async fn view_or_edit(paths: Vec<PathBuf>, mode: OpenMode) {
    let plan = tokio::task::spawn_blocking(move || plan_view_or_edit(&paths, mode))
        .await
        .unwrap_or_default();
    for (app, files) in plan {
        match app {
            Some(app) => launch(app, files).await,
            None => files.iter().for_each(|file| open_with_default_app(file)),
        }
    }
}

/// Groups `paths` by the app that should open them (`None` = the file's
/// default app, via `xdg-open`). Blocking: queries MIME types and reads
/// `.desktop` files.
fn plan_view_or_edit(paths: &[PathBuf], mode: OpenMode) -> Vec<(Option<AppEntry>, Vec<PathBuf>)> {
    let mut groups: Vec<(Option<String>, Vec<PathBuf>)> = Vec::new();
    for path in paths.iter().filter(|path| !path.is_dir()) {
        let mime = fs_ops::mime::mime_type(path);
        let is_image = mime
            .as_deref()
            .is_some_and(|mime| mime.starts_with("image/"));
        let app_id = if is_image {
            // The viewer registered for this image type, or the one for PNG
            // (formats like WebP often have no default of their own).
            mime.as_deref()
                .and_then(default_app_id)
                .or_else(|| default_app_id("image/png"))
        } else if mode == OpenMode::Edit {
            // The text editor: e.g. JSON's default app is often a browser,
            // and source files may have no default at all.
            default_app_id("text/plain")
        } else {
            None
        };
        match groups.iter_mut().find(|(id, _)| *id == app_id) {
            Some((_, files)) => files.push(path.clone()),
            None => groups.push((app_id, vec![path.clone()])),
        }
    }

    // Resolve desktop ids to launchable apps (reading .desktop files once).
    let apps = if groups.iter().any(|(id, _)| id.is_some()) {
        load_apps(None)
    } else {
        Vec::new()
    };
    groups
        .into_iter()
        .map(|(id, files)| {
            let app = id.and_then(|id| {
                let id = id.trim_end_matches(".desktop").to_string();
                let app = apps.iter().find(|app| app.id == id).cloned();
                if app.is_none() {
                    eprintln!("default app {id} not found; using xdg-open");
                }
                app
            });
            (app, files)
        })
        .collect()
}

/// An installed application the user can pick in the "Open With" dialog.
#[derive(Debug, Clone)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub icon: icon::Handle,
    exec: String,
    terminal: bool,
    /// Declares support for the file's MIME type.
    pub recommended: bool,
    /// The desktop's current default for the file's MIME type.
    pub is_default: bool,
}

/// Lists launchable applications, recommended ones (those declaring `mime`)
/// first with the default app at the very top, the rest alphabetically.
/// Blocking: reads every `.desktop` file on the system.
pub fn load_apps(mime: Option<&str>) -> Vec<AppEntry> {
    let default_id = mime.and_then(default_app_id);
    let locales = fde::get_languages_from_env();

    let entries: Vec<DesktopEntryData> = if fs_ops::sandbox::in_flatpak() {
        host_applications(&locales)
    } else {
        cosmic::desktop::load_applications(&locales, false, None).collect()
    };
    let mut apps: Vec<AppEntry> = entries
        .into_iter()
        .filter_map(|app| {
            let exec = app.exec?;
            let recommended =
                mime.is_some_and(|mime| app.mime_types.iter().any(|m| m.essence_str() == mime));
            let is_default = default_id
                .as_deref()
                .is_some_and(|id| id.trim_end_matches(".desktop") == app.id);
            Some(AppEntry {
                icon: app.icon.as_cosmic_icon(),
                id: app.id,
                name: app.name,
                exec,
                terminal: app.terminal,
                recommended,
                is_default,
            })
        })
        .collect();

    apps.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then(b.recommended.cmp(&a.recommended))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    apps
}

/// The host's applications, read from its `applications` folders (in a
/// Flatpak, libcosmic only sees the sandbox's). The first `.desktop` file
/// with an ID wins, as in the desktop entry spec; hidden ones are skipped.
fn host_applications(locales: &[String]) -> Vec<DesktopEntryData> {
    let data_dirs = fs_ops::sandbox::host_data_dirs();
    let dirs = data_dirs.iter().map(|dir| dir.join("applications"));
    let mut seen = std::collections::HashSet::new();
    fde::Iter::new(dirs)
        .filter_map(|path| fde::DesktopEntry::from_path(path, Some(locales)).ok())
        .filter(|entry| seen.insert(entry.appid.clone()))
        .filter(|entry| !entry.no_display() && !entry.hidden())
        .map(|entry| {
            let mut app = DesktopEntryData::from_desktop_entry(locales, entry);
            if let fde::IconSource::Name(name) = &app.icon {
                if let Some(path) = host_icon_file(&data_dirs, name) {
                    app.icon = fde::IconSource::Path(path);
                }
            }
            app
        })
        .collect()
}

/// An app icon file in the host's `hicolor` theme or pixmaps (e.g. icons of
/// other Flatpak apps, which the sandbox's icon theme can't see).
fn host_icon_file(data_dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    let sizes = ["scalable", "256x256", "128x128", "96x96", "64x64", "48x48"];
    data_dirs.iter().find_map(|dir| {
        sizes
            .iter()
            .flat_map(|size| {
                let apps = dir.join("icons/hicolor").join(size).join("apps");
                [
                    apps.join(format!("{name}.svg")),
                    apps.join(format!("{name}.png")),
                ]
            })
            .chain([dir.join("pixmaps").join(format!("{name}.png"))])
            .find(|path| path.is_file())
    })
}

fn default_app_id(mime: &str) -> Option<String> {
    let output = fs_ops::sandbox::host_command("xdg-mime")
        .args(["query", "default", mime])
        .output()
        .ok()?;
    let id = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!id.is_empty()).then_some(id)
}

/// Launches `app` on `paths`, expanding the `Exec` field codes as the Desktop
/// Entry spec describes. Apps that take a single file (`%f`/`%u`) get one
/// instance per path.
pub async fn launch(app: AppEntry, paths: Vec<PathBuf>) {
    if fs_ops::sandbox::in_flatpak() {
        launch_on_host(&app, &paths);
        return;
    }
    for command in expand_exec(&app.exec, &paths) {
        cosmic::desktop::spawn_desktop_exec(
            command,
            std::iter::empty::<(&str, &str)>(),
            Some(&app.id),
            app.terminal,
        )
        .await;
    }
}

/// [`launch`] in a Flatpak: each command runs on the host (terminal apps in
/// the user's terminal), fire-and-forget like [`open_with_default_app`].
fn launch_on_host(app: &AppEntry, paths: &[PathBuf]) {
    use std::process::Stdio;
    for command in expand_exec(&app.exec, paths) {
        let command = if app.terminal {
            format!("{} -e {command}", default_terminal())
        } else {
            command
        };
        let Some(args) = shlex::split(&command).filter(|args| !args.is_empty()) else {
            continue;
        };
        let spawned = fs_ops::sandbox::host_command(&args[0])
            .args(&args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match spawned {
            Ok(mut child) => {
                std::thread::spawn(move || child.wait());
            }
            Err(err) => eprintln!("failed to start {}: {err}", app.name),
        }
    }
}

/// Expands `exec`'s field codes for `paths`, returning one shell-quoted
/// command line per process to spawn.
fn expand_exec(exec: &str, paths: &[PathBuf]) -> Vec<String> {
    let Some(args) = shlex::split(exec) else {
        return Vec::new();
    };
    let single_file = args
        .iter()
        .any(|arg| arg.contains("%f") || arg.contains("%u"));

    let build = |files: &[PathBuf]| -> Option<String> {
        let mut out: Vec<String> = Vec::new();
        for arg in &args {
            match arg.as_str() {
                "%F" => out.extend(files.iter().map(|p| p.to_string_lossy().into_owned())),
                "%U" => out.extend(files.iter().map(|p| file_uri(p))),
                // Deprecated or icon/name/location codes we don't provide.
                "%i" | "%c" | "%k" | "%d" | "%D" | "%n" | "%N" | "%v" | "%m" => {}
                _ => {
                    let file = files.first();
                    let expanded = arg
                        .replace(
                            "%f",
                            &file
                                .map(|p| p.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                        )
                        .replace("%u", &file.map(|p| file_uri(p)).unwrap_or_default())
                        .replace("%%", "%");
                    if !expanded.is_empty() {
                        out.push(expanded);
                    }
                }
            }
        }
        shlex::try_join(out.iter().map(String::as_str)).ok()
    };

    if single_file && paths.len() > 1 {
        paths
            .iter()
            .filter_map(|path| build(std::slice::from_ref(path)))
            .collect()
    } else {
        build(paths).into_iter().collect()
    }
}

fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut uri = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-_.~".contains(&byte) {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_multi_file_codes_into_one_command() {
        let paths = vec![PathBuf::from("/tmp/a b.txt"), PathBuf::from("/tmp/c.txt")];
        assert_eq!(
            expand_exec("gedit %F", &paths),
            vec!["gedit '/tmp/a b.txt' /tmp/c.txt".to_string()]
        );
    }

    #[test]
    fn launches_single_file_apps_once_per_file() {
        let paths = vec![PathBuf::from("/tmp/a.txt"), PathBuf::from("/tmp/c.txt")];
        let commands: Vec<_> = expand_exec("viewer --file=%f %i", &paths)
            .iter()
            .map(|command| shlex::split(command).unwrap())
            .collect();
        assert_eq!(
            commands,
            vec![
                vec!["viewer", "--file=/tmp/a.txt"],
                vec!["viewer", "--file=/tmp/c.txt"]
            ]
        );
    }

    #[test]
    fn reads_the_terminal_from_cosmic_system_actions() {
        let contents = r#"{
    /// Opens the launcher
    Launcher: "cosmic-launcher",
    /// Opens the system default terminal
    Terminal: "cosmic-term",
}"#;
        assert_eq!(
            terminal_from_system_actions(contents),
            Some("cosmic-term".to_string())
        );
        assert_eq!(terminal_from_system_actions("{ Launcher: \"x\" }"), None);
    }

    #[test]
    fn encodes_uris() {
        assert_eq!(
            file_uri(Path::new("/tmp/a b#.txt")),
            "file:///tmp/a%20b%23.txt"
        );
    }
}
