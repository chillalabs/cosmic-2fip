//! Content previews ("thumbnails") following the freedesktop Thumbnail
//! Managing Standard, so they're shared with COSMIC Files and other file
//! managers through `~/.cache/thumbnails`.
//!
//! A cached thumbnail is only used if the `Thumb::MTime` stored in it matches
//! the file's modification time, so edited files get a fresh preview. Missing
//! ones are made by the system's thumbnailers (`/usr/share/thumbnailers`:
//! images, PDFs, videos, fonts, ...).

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Pixel size of the thumbnails we generate (the spec's "normal" size).
const NORMAL_SIZE: u32 = 128;
/// A thumbnailer taking longer than this is killed (e.g. a huge video).
const THUMBNAILER_TIMEOUT: Duration = Duration::from_secs(15);

/// The MIME type guessed from `path`'s extension (e.g. `image/png`). Fast (no
/// disk access), so it can run for every file in a listing.
pub fn guess_mime(path: &Path) -> Option<String> {
    mime_guess::from_path(path)
        .first()
        .map(|mime| mime.essence_str().to_string())
}

/// Whether some installed thumbnailer can preview files of type `mime`.
pub fn can_thumbnail(mime: &str) -> bool {
    thumbnailers().contains_key(mime)
}

/// A valid cached thumbnail for `path` (any size), or a newly generated one.
/// `None` if the type has no thumbnailer or generating failed. Blocking.
pub fn thumbnail(path: &Path, mime: &str) -> Option<PathBuf> {
    let mtime = mtime_secs(path)?;
    let uri = file_uri(path);
    let name = format!("{:x}.png", md5::compute(uri.as_bytes()));
    let cache = cache_dir()?;

    for size in ["normal", "large", "x-large", "xx-large"] {
        let candidate = cache.join(size).join(&name);
        if is_fresh(&candidate, mtime) {
            return Some(candidate);
        }
    }

    let target = cache.join("normal").join(&name);
    generate(path, mime, &uri, mtime, &target).then_some(target)
}

/// `file://` URI for `path`, escaped like GLib's `g_filename_to_uri` (the
/// spec's reference), so the cache file names match other file managers'.
pub fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut uri = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        let unreserved = byte.is_ascii_alphanumeric() || b"-_.!~*'()/:@&=+$,".contains(&byte);
        if unreserved {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

fn mtime_secs(path: &Path) -> Option<u64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    Some(modified.duration_since(UNIX_EPOCH).ok()?.as_secs())
}

fn cache_dir() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CACHE_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".cache"),
    };
    Some(base.join("thumbnails"))
}

/// Whether `thumb` exists and was made from the file's current version.
fn is_fresh(thumb: &Path, mtime: u64) -> bool {
    let Ok(file) = File::open(thumb) else {
        return false;
    };
    let Ok(reader) = png::Decoder::new(BufReader::new(file)).read_info() else {
        return false;
    };
    let info = reader.info();
    let stored = info
        .uncompressed_latin1_text
        .iter()
        .map(|chunk| (chunk.keyword.as_str(), chunk.text.clone()))
        .chain(
            info.utf8_text
                .iter()
                .filter_map(|chunk| Some((chunk.keyword.as_str(), chunk.get_text().ok()?))),
        )
        .find(|(keyword, _)| *keyword == "Thumb::MTime")
        .map(|(_, text)| text);
    stored.and_then(|text| text.trim().parse::<u64>().ok()) == Some(mtime)
}

/// Runs the thumbnailer for `mime` and stores its output at `target` with the
/// spec's `Thumb::URI` / `Thumb::MTime` tags. Returns whether it worked.
fn generate(path: &Path, mime: &str, uri: &str, mtime: u64, target: &Path) -> bool {
    let Some(exec) = thumbnailers().get(mime) else {
        return false;
    };
    let Some(dir) = target.parent() else {
        return false;
    };
    if create_private_dir(dir).is_err() {
        return false;
    }
    // The thumbnailer writes to /tmp under GNOME's thumbnail-factory name:
    // some thumbnailers are confined (e.g. Ubuntu's AppArmor profile lets
    // evince-thumbnailer create only /tmp/.gnome_desktop_thumbnail*). Then we
    // tag it next to the target and rename it in: never a half-written PNG.
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = format!(
        "{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let raw = PathBuf::from(format!("/tmp/.gnome_desktop_thumbnail.2fip-{unique}.png"));
    let tagged = dir.join(format!(".2fip-{unique}.png"));

    let ok = run_thumbnailer(exec, path, uri, &raw)
        && add_thumbnail_tags(&raw, &tagged, uri, mtime).is_some()
        && fs::rename(&tagged, target).is_ok();
    let _ = fs::remove_file(&raw);
    let _ = fs::remove_file(&tagged);
    ok
}

fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

/// Expands the thumbnailer's `Exec` line (`%s` size, `%u` URI, `%i` path,
/// `%o` output, `%%`) and runs it, killing it after [`THUMBNAILER_TIMEOUT`].
fn run_thumbnailer(exec: &str, path: &Path, uri: &str, output: &Path) -> bool {
    let Some(args) = expand_exec(exec, path, uri, output) else {
        return false;
    };
    let Some((program, args)) = args.split_first() else {
        return false;
    };
    let Ok(mut child) = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success() && output.exists(),
            Ok(None) if started.elapsed() < THUMBNAILER_TIMEOUT => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

fn expand_exec(exec: &str, path: &Path, uri: &str, output: &Path) -> Option<Vec<String>> {
    let size = NORMAL_SIZE.to_string();
    let path = path.to_string_lossy();
    let output = output.to_string_lossy();
    let args = shlex::split(exec)?
        .into_iter()
        .map(|arg| {
            let mut expanded = String::with_capacity(arg.len());
            let mut chars = arg.chars();
            while let Some(c) = chars.next() {
                if c != '%' {
                    expanded.push(c);
                    continue;
                }
                match chars.next() {
                    Some('s') => expanded.push_str(&size),
                    Some('u') => expanded.push_str(uri),
                    Some('i') => expanded.push_str(&path),
                    Some('o') => expanded.push_str(&output),
                    Some('%') => expanded.push('%'),
                    Some(other) => {
                        expanded.push('%');
                        expanded.push(other);
                    }
                    None => expanded.push('%'),
                }
            }
            expanded
        })
        .collect();
    Some(args)
}

/// Re-encodes the thumbnailer's PNG with the spec's tags added.
fn add_thumbnail_tags(raw: &Path, tagged: &Path, uri: &str, mtime: u64) -> Option<()> {
    let mut decoder = png::Decoder::new(BufReader::new(File::open(raw).ok()?));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut pixels = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut pixels).ok()?;
    pixels.truncate(frame.buffer_size());

    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(File::create(tagged).ok()?),
        frame.width,
        frame.height,
    );
    encoder.set_color(frame.color_type);
    encoder.set_depth(frame.bit_depth);
    encoder
        .add_text_chunk("Thumb::URI".to_string(), uri.to_string())
        .ok()?;
    encoder
        .add_text_chunk("Thumb::MTime".to_string(), mtime.to_string())
        .ok()?;
    encoder
        .add_text_chunk("Software".to_string(), "2fip".to_string())
        .ok()?;
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(&pixels).ok()?;
    writer.finish().ok()
}

/// MIME type → thumbnailer `Exec` line, from the system's and the user's
/// `.thumbnailer` files (user entries win). Read once.
fn thumbnailers() -> &'static HashMap<String, String> {
    static THUMBNAILERS: OnceLock<HashMap<String, String>> = OnceLock::new();
    THUMBNAILERS.get_or_init(|| {
        let mut dirs: Vec<PathBuf> = std::env::var("XDG_DATA_DIRS")
            .ok()
            .filter(|dirs| !dirs.is_empty())
            .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string())
            .split(':')
            .rev()
            .map(|dir| PathBuf::from(dir).join("thumbnailers"))
            .collect();
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join(".local/share/thumbnailers"));
        }

        let mut map = HashMap::new();
        for dir in dirs {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                if let Ok(contents) = fs::read_to_string(entry.path()) {
                    if let Some((exec, mimes)) = parse_thumbnailer(&contents) {
                        for mime in mimes {
                            map.insert(mime, exec.clone());
                        }
                    }
                }
            }
        }
        map
    })
}

/// Reads a `.thumbnailer` file: its `Exec` line and MIME types, if its
/// `TryExec` program is installed.
fn parse_thumbnailer(contents: &str) -> Option<(String, Vec<String>)> {
    let mut exec = None;
    let mut mimes = Vec::new();
    let mut try_exec = None;
    for line in contents.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "Exec" => exec = Some(value.trim().to_string()),
            "TryExec" => try_exec = Some(value.trim().to_string()),
            "MimeType" => {
                mimes = value
                    .split(';')
                    .map(str::trim)
                    .filter(|mime| !mime.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            _ => {}
        }
    }
    if let Some(program) = try_exec {
        if !program_exists(&program) {
            return None;
        }
    }
    Some((exec?, mimes))
}

fn program_exists(program: &str) -> bool {
    if program.contains('/') {
        return Path::new(program).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
}

/// Seconds since the epoch of `time`, for callers keying caches by mtime.
pub fn secs(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_are_escaped_like_glib() {
        assert_eq!(
            file_uri(Path::new("/home/a b/(1)#x.png")),
            "file:///home/a%20b/(1)%23x.png"
        );
        assert_eq!(file_uri(Path::new("/tmp/ñ")), "file:///tmp/%C3%B1");
    }

    #[test]
    fn cache_names_follow_the_spec() {
        // Example from the Thumbnail Managing Standard.
        let uri = "file:///home/jens/photos/me.png";
        assert_eq!(
            format!("{:x}", md5::compute(uri.as_bytes())),
            "c6ee772d9e49320e97ec29a7eb5b1697"
        );
    }

    #[test]
    fn parses_thumbnailer_files() {
        let contents = "[Thumbnailer Entry]\nTryExec=sh\nExec=evince-thumbnailer -s %s %u %o\nMimeType=application/pdf;image/tiff;\n";
        let (exec, mimes) = parse_thumbnailer(contents).unwrap();
        assert_eq!(exec, "evince-thumbnailer -s %s %u %o");
        assert_eq!(mimes, ["application/pdf", "image/tiff"]);
        assert!(parse_thumbnailer("TryExec=no-such-program-xyz\nExec=x\n").is_none());
    }

    #[test]
    fn expands_exec_field_codes() {
        let args = expand_exec(
            "thumb -s %s %u %o --in=%i 100%%",
            Path::new("/a b.png"),
            "file:///a%20b.png",
            Path::new("/tmp/out.png"),
        )
        .unwrap();
        assert_eq!(
            args,
            [
                "thumb",
                "-s",
                "128",
                "file:///a%20b.png",
                "/tmp/out.png",
                "--in=/a b.png",
                "100%"
            ]
        );
    }

    #[test]
    fn guesses_mime_from_extension() {
        assert_eq!(guess_mime(Path::new("x.PNG")).as_deref(), Some("image/png"));
        assert_eq!(
            guess_mime(Path::new("doc.pdf")).as_deref(),
            Some("application/pdf")
        );
        assert_eq!(guess_mime(Path::new("README")), None);
    }

    #[test]
    fn tags_roundtrip_and_freshness_check() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("raw.png");
        let tagged = dir.path().join("tagged.png");
        // A 1x1 RGBA PNG as a thumbnailer would write it.
        {
            let file = File::create(&raw).unwrap();
            let mut encoder = png::Encoder::new(file, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[1, 2, 3, 255]).unwrap();
        }

        add_thumbnail_tags(&raw, &tagged, "file:///x.png", 1234).unwrap();

        assert!(is_fresh(&tagged, 1234));
        assert!(!is_fresh(&tagged, 1235), "a changed file must not reuse it");
        assert!(!is_fresh(&raw, 1234), "untagged thumbnails aren't trusted");
    }

    /// Runs the real system thumbnailers; `cargo test -- --ignored`.
    #[test]
    #[ignore = "needs the desktop's thumbnailers installed"]
    fn generates_previews_with_system_thumbnailers() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", dir.path().join("cache"));

        let image = dir.path().join("pixel.png");
        {
            let file = File::create(&image).unwrap();
            let mut encoder = png::Encoder::new(file, 2, 2);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[255; 12]).unwrap();
        }
        let thumb = thumbnail(&image, "image/png").expect("image thumbnail");
        assert!(thumb.starts_with(dir.path().join("cache/thumbnails/normal")));
        assert!(is_fresh(&thumb, mtime_secs(&image).unwrap()));

        if let Ok(pdf) = std::env::var("TEST_PDF") {
            let thumb = thumbnail(Path::new(&pdf), "application/pdf").expect("pdf thumbnail");
            assert!(thumb.exists());
        }
    }
}
